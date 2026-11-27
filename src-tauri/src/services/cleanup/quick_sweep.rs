use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::error::Result;
use crate::models::cleanup::{QuickSweepGroupTotal, QuickSweepItem, QuickSweepReport};
use crate::models::intelligence::{RecommendationGroup, RiskLevel};
use crate::services::applications::ApplicationsService;
use crate::services::cleanup::safeguards;
use crate::services::scanner::fast_walker::{effective_file_size, measure_dir_parallel};

pub struct QuickSweepService;

struct SweepTargetSpec {
    path: PathBuf,
    name: String,
    sweep_group: &'static str,
    group_label: &'static str,
    category: &'static str,
    rec_group: RecommendationGroup,
    risk_level: RiskLevel,
    reason: String,
    expected_consequences: String,
    recommended_action: String,
    min_bytes: u64,
    default_selected: bool,
}

impl QuickSweepService {
    /// Performs a fast, bounded-parallel, strictly read-only sweep across high-impact macOS /
    /// developer / browser cache and stale download paths (inspired by `tw93/mole`'s `clean` & `purge`).
    /// Never deletes anything automatically; items must pass through explicit cleanup confirmation.
    pub fn run_quick_sweep() -> Result<QuickSweepReport> {
        let started = Instant::now();
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let home_path = PathBuf::from(&home);
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut specs: Vec<SweepTargetSpec> = Vec::new();

        Self::collect_dev_cache_targets(&home_path, &mut specs);
        Self::collect_browser_cache_targets(&home_path, &mut specs);
        Self::collect_system_and_app_cache_targets(&home_path, &mut specs);
        Self::collect_stale_project_build_targets(&home_path, now_secs, &mut specs);

        // Deduplicate target specs by path
        let mut seen_paths = std::collections::HashSet::new();
        specs.retain(|s| s.path.exists() && seen_paths.insert(s.path.clone()));

        // Measure directory targets + download files + orphaned leftovers concurrently
        let (measured_items, download_items, leftover_items) = std::thread::scope(|scope| {
            let downloads_handle = scope.spawn(|| Self::scan_large_downloads(&home_path, now_secs));
            let leftovers_handle = scope.spawn(ApplicationsService::find_app_leftovers);

            let mut sweep_results = Vec::new();
            if !specs.is_empty() {
                let chunk_size = (specs.len() / 6).max(1);
                let spec_handles: Vec<_> = specs
                    .chunks(chunk_size)
                    .map(|chunk| {
                        scope.spawn(move || {
                            let mut local = Vec::new();
                            for spec in chunk {
                                let stats = measure_dir_parallel(&spec.path);
                                if stats.size_bytes < spec.min_bytes {
                                    continue;
                                }
                                let age_days = stats
                                    .latest_modified
                                    .map(|m| ((now_secs - m).max(0)) / 86_400)
                                    .unwrap_or(0);
                                let item = Self::build_item(
                                    spec,
                                    stats.size_bytes,
                                    stats.file_count,
                                    age_days,
                                );
                                local.push(item);
                            }
                            local
                        })
                    })
                    .collect();

                for h in spec_handles {
                    if let Ok(mut list) = h.join() {
                        sweep_results.append(&mut list);
                    }
                }
            }

            let downloads = downloads_handle.join().unwrap_or_default();
            let leftovers = leftovers_handle.join().unwrap_or_default();
            (sweep_results, downloads, leftovers)
        });

        let mut items = measured_items;
        items.extend(download_items);

        for l in leftover_items {
            let path_obj = Path::new(&l.path);
            let exists_on_disk = path_obj.exists();
            let (is_cloud, cloud_warning) = safeguards::detect_cloud_sync(path_obj);
            let (is_critical, critical_reason) = safeguards::is_system_critical_path(path_obj);
            let is_blocked = is_critical;
            items.push(QuickSweepItem {
                id: format!("qs-leftover-{}", Uuid::new_v4()),
                path: l.path,
                name: format!("{} ({})", l.app_hint, l.kind.replace('_', " ")),
                size_bytes: l.size_bytes,
                file_count: l.file_count,
                sweep_group: "app_leftovers".to_string(),
                group_label: "Orphaned App Leftovers".to_string(),
                category: "temporary".to_string(),
                rec_group: if l.is_cache_safe {
                    RecommendationGroup::SafeToReview
                } else {
                    RecommendationGroup::ReviewRequired
                },
                risk_level: if l.is_cache_safe {
                    RiskLevel::Low
                } else {
                    RiskLevel::Moderate
                },
                reason: l.reason,
                expected_consequences:
                    "Residual cache/container for an app not found in /Applications. Moves safely to Trash."
                        .to_string(),
                recommended_action: "Verify you no longer use this app before moving its leftover data to Trash."
                    .to_string(),
                age_days: 0,
                is_cloud_synced: is_cloud,
                cloud_warning,
                is_blocked,
                block_reason: critical_reason.map(|s| s.to_string()),
                selected: false, // Never auto-check app leftovers without user consent
                exists_on_disk,
            });
        }

        items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        let mut total_bytes = 0u64;
        let mut safe_bytes = 0u64;
        let mut review_bytes = 0u64;
        let mut totals_map: HashMap<String, QuickSweepGroupTotal> = HashMap::new();

        for item in &items {
            total_bytes = total_bytes.saturating_add(item.size_bytes);
            if item.rec_group == RecommendationGroup::SafeToReview && !item.is_blocked {
                safe_bytes = safe_bytes.saturating_add(item.size_bytes);
            } else {
                review_bytes = review_bytes.saturating_add(item.size_bytes);
            }
            let entry = totals_map
                .entry(item.sweep_group.clone())
                .or_insert_with(|| QuickSweepGroupTotal {
                    sweep_group: item.sweep_group.clone(),
                    label: item.group_label.clone(),
                    item_count: 0,
                    total_bytes: 0,
                });
            entry.item_count += 1;
            entry.total_bytes = entry.total_bytes.saturating_add(item.size_bytes);
        }

        let mut category_totals: Vec<QuickSweepGroupTotal> = totals_map.into_values().collect();
        category_totals.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));

        let total_items = items.len();
        Ok(QuickSweepReport {
            scan_id: format!("qs-{}", Uuid::new_v4()),
            duration_millis: started.elapsed().as_millis() as u64,
            items,
            total_bytes,
            safe_bytes,
            review_bytes,
            total_items,
            category_totals,
        })
    }

    fn build_item(spec: &SweepTargetSpec, size_bytes: u64, file_count: u64, age_days: i64) -> QuickSweepItem {
        let (is_cloud, cloud_warning) = safeguards::detect_cloud_sync(&spec.path);
        let (is_critical, critical_reason) = safeguards::is_system_critical_path(&spec.path);
        let is_blocked = is_critical || spec.rec_group == RecommendationGroup::SystemProtected;

        QuickSweepItem {
            id: format!("qs-{}", Uuid::new_v4()),
            path: spec.path.to_string_lossy().to_string(),
            name: spec.name.clone(),
            size_bytes,
            file_count,
            sweep_group: spec.sweep_group.to_string(),
            group_label: spec.group_label.to_string(),
            category: spec.category.to_string(),
            rec_group: spec.rec_group,
            risk_level: spec.risk_level,
            reason: spec.reason.clone(),
            expected_consequences: spec.expected_consequences.clone(),
            recommended_action: spec.recommended_action.clone(),
            age_days,
            is_cloud_synced: is_cloud,
            cloud_warning,
            is_blocked,
            block_reason: critical_reason.map(|s| s.to_string()),
            selected: spec.default_selected && !is_blocked && !is_cloud,
            exists_on_disk: true,
        }
    }

    fn collect_dev_cache_targets(home_path: &Path, specs: &mut Vec<SweepTargetSpec>) {
        // Individual Xcode DerivedData projects
        #[cfg(target_os = "macos")]
        {
            let derived = home_path.join("Library/Developer/Xcode/DerivedData");
            if let Ok(entries) = fs::read_dir(&derived) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name == "ModuleCache.noindex" || !name.starts_with('.') {
                            specs.push(SweepTargetSpec {
                                path: p,
                                name: format!("Xcode DerivedData / {}", name),
                                sweep_group: "dev_caches",
                                group_label: "Developer & Build Caches",
                                category: "developer",
                                rec_group: RecommendationGroup::SafeToReview,
                                risk_level: RiskLevel::Low,
                                reason: "Regenerable Xcode compiler index and build intermediate cache."
                                    .to_string(),
                                expected_consequences:
                                    "Xcode rebuilds derived products automatically on next project build."
                                        .to_string(),
                                recommended_action: "Move to Trash to reclaim Xcode build cache space."
                                    .to_string(),
                                min_bytes: 10 * 1024 * 1024,
                                default_selected: true,
                            });
                        }
                    }
                }
            }
        }

        let static_dev_targets: Vec<(&str, PathBuf, &str, bool)> = vec![
            #[cfg(target_os = "macos")]
            (
                "Homebrew Downloads Cache",
                home_path.join("Library/Caches/Homebrew"),
                "Cached Homebrew bottle archives and source tarballs already installed.",
                true,
            ),
            #[cfg(target_os = "macos")]
            (
                "Xcode CoreSimulator Caches",
                home_path.join("Library/Developer/CoreSimulator/Caches"),
                "Simulator runtime dyld and image caches.",
                true,
            ),
            #[cfg(target_os = "macos")]
            (
                "CocoaPods Cache",
                home_path.join("Library/Caches/CocoaPods"),
                "Downloaded CocoaPods pod archives.",
                true,
            ),
            (
                "npm Global Cache",
                home_path.join(".npm"),
                "Cached npm registry tarballs; re-downloaded automatically when needed.",
                true,
            ),
            #[cfg(target_os = "macos")]
            (
                "pnpm Content-Addressable Store",
                home_path.join("Library/pnpm/store"),
                "Shared pnpm package store; unused packages can be reclaimed cleanly.",
                false,
            ),
            #[cfg(target_os = "macos")]
            (
                "Yarn Global Cache",
                home_path.join("Library/Caches/Yarn"),
                "Cached Yarn package archives.",
                true,
            ),
            (
                "Bun Install Cache",
                home_path.join(".bun/install/cache"),
                "Cached Bun package tarballs.",
                true,
            ),
            (
                "Cargo Registry Download Cache",
                home_path.join(".cargo/registry/cache"),
                "Compressed Rust crate `.crate` archives downloaded by Cargo.",
                true,
            ),
            (
                "pip Package Cache",
                home_path.join(".cache/pip"),
                "Python pip wheel and HTTP download cache.",
                true,
            ),
            (
                "uv Package Cache",
                home_path.join(".cache/uv"),
                "Python uv wheel and package download cache.",
                true,
            ),
            (
                "Gradle Build Cache",
                home_path.join(".gradle/caches"),
                "Cached Java/Android Gradle dependency artifacts and daemon caches.",
                false,
            ),
            (
                "Maven Local Repository",
                home_path.join(".m2/repository"),
                "Downloaded Java Maven dependency jars.",
                false,
            ),
        ];

        for (label, path, reason, safe_default) in static_dev_targets {
            specs.push(SweepTargetSpec {
                path,
                name: label.to_string(),
                sweep_group: "dev_caches",
                group_label: "Developer & Build Caches",
                category: "developer",
                rec_group: if safe_default {
                    RecommendationGroup::SafeToReview
                } else {
                    RecommendationGroup::ReviewRequired
                },
                risk_level: if safe_default {
                    RiskLevel::Low
                } else {
                    RiskLevel::Moderate
                },
                reason: reason.to_string(),
                expected_consequences:
                    "Package manager re-downloads dependencies if needed by future offline builds."
                        .to_string(),
                recommended_action: "Safe to move to Trash when not offline-developing."
                    .to_string(),
                min_bytes: 5 * 1024 * 1024,
                default_selected: safe_default,
            });
        }
    }

    fn collect_browser_cache_targets(home_path: &Path, specs: &mut Vec<SweepTargetSpec>) {
        // Strictly browser cache folders ONLY — never profiles, cookies, bookmarks, or saved passwords!
        #[cfg(target_os = "macos")]
        let browser_caches = vec![
            (
                "Google Chrome Cache",
                home_path.join("Library/Caches/Google/Chrome"),
            ),
            (
                "Microsoft Edge Cache",
                home_path.join("Library/Caches/Microsoft Edge"),
            ),
            (
                "Brave Browser Cache",
                home_path.join("Library/Caches/BraveSoftware/Brave-Browser"),
            ),
            (
                "Arc Browser Cache",
                home_path.join("Library/Caches/company.thebrowser.Browser"),
            ),
            (
                "Mozilla Firefox Cache",
                home_path.join("Library/Caches/Firefox"),
            ),
            (
                "Safari Cache",
                home_path.join("Library/Caches/com.apple.Safari"),
            ),
        ];

        #[cfg(not(target_os = "macos"))]
        let browser_caches: Vec<(&str, PathBuf)> = {
            let _ = home_path;
            Vec::new()
        };

        for (name, path) in browser_caches {
            specs.push(SweepTargetSpec {
                path,
                name: name.to_string(),
                sweep_group: "browser_caches",
                group_label: "Browser Caches",
                category: "browsers",
                rec_group: RecommendationGroup::SafeToReview,
                risk_level: RiskLevel::Low,
                reason: "Browser web cache only (bookmarks, history, and passwords stay untouched)."
                    .to_string(),
                expected_consequences: "Sites re-fetch cached assets cleanly on next load."
                    .to_string(),
                recommended_action: "Move browser cache folder to Trash.".to_string(),
                min_bytes: 2 * 1024 * 1024,
                default_selected: true,
            });
        }
    }

    fn collect_system_and_app_cache_targets(home_path: &Path, specs: &mut Vec<SweepTargetSpec>) {
        #[cfg(target_os = "macos")]
        {
            let caches_dir = home_path.join("Library/Caches");
            if let Ok(entries) = fs::read_dir(&caches_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if !p.is_dir() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    let lower = name.to_lowercase();
                    if lower.starts_with("com.apple.")
                        || lower == "homebrew"
                        || lower == "cloudkit"
                        || lower == "google"
                        || lower == "microsoft edge"
                        || lower == "firefox"
                        || lower == "bravesoftware"
                        || lower == "company.thebrowser.browser"
                        || lower == "cocoapods"
                        || lower == "yarn"
                        || lower.contains("diskatlas")
                    {
                        continue;
                    }
                    specs.push(SweepTargetSpec {
                        path: p,
                        name: format!("App Cache: {}", name),
                        sweep_group: "caches",
                        group_label: "App Caches & Logs",
                        category: "temporary",
                        rec_group: RecommendationGroup::SafeToReview,
                        risk_level: RiskLevel::Low,
                        reason: "Application cache directory in ~/Library/Caches.".to_string(),
                        expected_consequences:
                            "Applications regenerate temporary cache files as needed without losing user settings."
                                .to_string(),
                        recommended_action: "Move application cache to Trash.".to_string(),
                        min_bytes: 5 * 1024 * 1024,
                        default_selected: false,
                    });
                }
            }

            let logs_dir = home_path.join("Library/Logs");
            if let Ok(entries) = fs::read_dir(&logs_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if !p.is_dir() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    specs.push(SweepTargetSpec {
                        path: p,
                        name: format!("App Logs: {}", name),
                        sweep_group: "logs",
                        group_label: "Logs & Crash Reports",
                        category: "temporary",
                        rec_group: RecommendationGroup::SafeToReview,
                        risk_level: RiskLevel::Low,
                        reason: "Historical application log and diagnostic files.".to_string(),
                        expected_consequences:
                            "Diagnostic history is cleared while apps keep running normally."
                                .to_string(),
                        recommended_action: "Move old logs to Trash.".to_string(),
                        min_bytes: 2 * 1024 * 1024,
                        default_selected: true,
                    });
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let local_cache = home_path.join(".cache");
            if let Ok(entries) = fs::read_dir(&local_cache) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if !p.is_dir() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    specs.push(SweepTargetSpec {
                        path: p,
                        name: format!("User Cache: {}", name),
                        sweep_group: "caches",
                        group_label: "App Caches & Logs",
                        category: "temporary",
                        rec_group: RecommendationGroup::SafeToReview,
                        risk_level: RiskLevel::Low,
                        reason: "User cache folder in ~/.cache.".to_string(),
                        expected_consequences: "Apps recreate caches automatically.".to_string(),
                        recommended_action: "Move cache folder to Trash.".to_string(),
                        min_bytes: 5 * 1024 * 1024,
                        default_selected: false,
                    });
                }
            }
        }
    }

    fn collect_stale_project_build_targets(
        home_path: &Path,
        now_secs: i64,
        specs: &mut Vec<SweepTargetSpec>,
    ) {
        let roots = [
            home_path.join("Projects"),
            home_path.join("Developer"),
            home_path.join("Desktop"),
            home_path.join("Documents"),
            home_path.join("IdeaProjects"),
            home_path.join("AndroidStudioProjects"),
        ];

        let mut stack: Vec<(PathBuf, usize)> = roots
            .into_iter()
            .filter(|p| p.is_dir())
            .map(|p| (p, 0usize))
            .collect();

        while let Some((dir, depth)) = stack.pop() {
            if depth > 3 {
                continue;
            }
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let p = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') && name != ".venv" {
                    continue;
                }
                let Ok(meta) = fs::symlink_metadata(&p) else {
                    continue;
                };
                if meta.file_type().is_symlink() || !meta.is_dir() {
                    continue;
                }
                if matches!(name.as_str(), "node_modules" | "target" | ".venv" | "build" | "dist") {
                    let mod_secs = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs() as i64))
                        .unwrap_or(now_secs);
                    let age_days = ((now_secs - mod_secs).max(0)) / 86_400;
                    let project_name = dir
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| dir.to_string_lossy().to_string());
                    let is_stale = age_days >= 45;
                    specs.push(SweepTargetSpec {
                        path: p,
                        name: format!("{} / {}", project_name, name),
                        sweep_group: "dev_caches",
                        group_label: "Developer & Build Caches",
                        category: "developer",
                        rec_group: if is_stale {
                            RecommendationGroup::SafeToReview
                        } else {
                            RecommendationGroup::ReviewRequired
                        },
                        risk_level: if is_stale && name != "node_modules" {
                            RiskLevel::Low
                        } else {
                            RiskLevel::Moderate
                        },
                        reason: if is_stale {
                            format!("Project build/dependency folder untouched for {} days.", age_days)
                        } else {
                            format!("Active project build/dependency directory ({}).", name)
                        },
                        expected_consequences:
                            "Source code stays intact; rebuild or run package install before running that project."
                                .to_string(),
                        recommended_action: "Move stale build output or dependencies to Trash.".to_string(),
                        min_bytes: 50 * 1024 * 1024,
                        default_selected: false,
                    });
                    continue;
                }
                if depth < 3 {
                    stack.push((p, depth + 1));
                }
            }
        }
    }

    fn scan_large_downloads(home_path: &Path, now_secs: i64) -> Vec<QuickSweepItem> {
        let downloads_dir = home_path.join("Downloads");
        let Ok(entries) = fs::read_dir(&downloads_dir) else {
            return Vec::new();
        };

        let mut items = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            let Ok(meta) = fs::symlink_metadata(&p) else {
                continue;
            };
            if meta.file_type().is_symlink() || !meta.is_file() {
                continue;
            }
            let (sz, _) = effective_file_size(&meta);
            if sz < 25 * 1024 * 1024 {
                continue;
            }
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if !matches!(
                ext.as_str(),
                "dmg" | "pkg" | "iso" | "zip" | "tar" | "gz" | "tgz" | "7z" | "rar" | "exe" | "msi"
            ) {
                continue;
            }
            let mod_secs = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs() as i64))
                .unwrap_or(now_secs);
            let age_days = ((now_secs - mod_secs).max(0)) / 86_400;
            let name = entry.file_name().to_string_lossy().to_string();
            let is_old = age_days >= 30;
            let (is_cloud, cloud_warning) = safeguards::detect_cloud_sync(&p);
            let (is_critical, critical_reason) = safeguards::is_system_critical_path(&p);

            items.push(QuickSweepItem {
                id: format!("qs-dl-{}", Uuid::new_v4()),
                path: p.to_string_lossy().to_string(),
                name,
                size_bytes: sz,
                file_count: 1,
                sweep_group: "downloads".to_string(),
                group_label: "Installers & Large Downloads".to_string(),
                category: "archives".to_string(),
                rec_group: if is_old {
                    RecommendationGroup::SafeToReview
                } else {
                    RecommendationGroup::ReviewRequired
                },
                risk_level: RiskLevel::Low,
                reason: if is_old {
                    format!("Downloaded .{} installer/archive untouched for {} days.", ext, age_days)
                } else {
                    format!("Large downloaded .{} installer/archive in ~/Downloads.", ext)
                },
                expected_consequences:
                    "Installed applications keep working; installer/archive moves safely to Trash."
                        .to_string(),
                recommended_action: "Move unneeded installer or archive to Trash.".to_string(),
                age_days,
                is_cloud_synced: is_cloud,
                cloud_warning,
                is_blocked: is_critical,
                block_reason: critical_reason.map(|s| s.to_string()),
                selected: is_old && !is_critical && !is_cloud,
                exists_on_disk: true,
            });
        }
        items
    }
}
