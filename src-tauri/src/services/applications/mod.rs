use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::error::Result;
use crate::models::cleanup::{
    AppAssociatedPath, AppLeftoverItem, ApplicationsInventoryReport, InstalledAppInfo,
};
use crate::services::scanner::fast_walker::measure_dir_parallel;

pub struct ApplicationsService;

#[derive(Debug, Clone)]
struct RawBundle {
    name: String,
    bundle_id: Option<String>,
    version: Option<String>,
    app_path: PathBuf,
    is_system: bool,
}

impl ApplicationsService {
    /// Inspects installed macOS applications, measures bundles and associated Library data
    /// in parallel, and surfaces orphaned app leftovers (`mo uninstall` workflow).
    pub fn inspect_installed_applications() -> Result<ApplicationsInventoryReport> {
        let started = Instant::now();
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let home_path = PathBuf::from(&home);

        let all_bundles = Self::discover_app_bundles(&home_path);
        let installed_ids: HashSet<String> = all_bundles
            .iter()
            .filter_map(|b| b.bundle_id.as_ref().map(|s| s.to_lowercase()))
            .collect();
        let installed_names: HashSet<String> = all_bundles
            .iter()
            .map(|b| Self::normalize_app_token(&b.name))
            .collect();

        // Discover ~/Library candidates once
        let lib_candidates = Self::collect_library_entries(&home_path);

        // Measure all app bundles & library entries concurrently using bounded worker queue
        let user_bundles: Vec<RawBundle> = all_bundles
            .into_iter()
            .filter(|b| !b.is_system)
            .collect();

        let bundle_sizes: Vec<(u64, u64)> = std::thread::scope(|scope| {
            let mut results = vec![(0u64, 0u64); user_bundles.len()];
            let chunk_size = (user_bundles.len() / 6).max(1);
            let chunks: Vec<_> = user_bundles.chunks(chunk_size).collect();
            let handles: Vec<_> = chunks
                .into_iter()
                .map(|chunk| {
                    scope.spawn(move || {
                        chunk
                            .iter()
                            .map(|b| {
                                let st = measure_dir_parallel(&b.app_path);
                                (st.size_bytes, st.file_count)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            let mut idx = 0usize;
            for h in handles {
                if let Ok(vec_res) = h.join() {
                    for pair in vec_res {
                        if idx < results.len() {
                            results[idx] = pair;
                            idx += 1;
                        }
                    }
                }
            }
            results
        });

        let lib_measured: Vec<(LibCandidate, u64, u64)> = std::thread::scope(|scope| {
            if lib_candidates.is_empty() {
                return Vec::new();
            }
            let chunk_size = (lib_candidates.len() / 6).max(1);
            let handles: Vec<_> = lib_candidates
                .chunks(chunk_size)
                .map(|chunk| {
                    scope.spawn(move || {
                        chunk
                            .iter()
                            .cloned()
                            .map(|c| {
                                let st = measure_dir_parallel(&c.path);
                                (c, st.size_bytes, st.file_count)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            let mut out = Vec::with_capacity(lib_candidates.len());
            for h in handles {
                if let Ok(vec_res) = h.join() {
                    out.extend(vec_res);
                }
            }
            out
        });

        let mut matched_lib_indices: HashSet<usize> = HashSet::new();
        let mut apps: Vec<InstalledAppInfo> = Vec::with_capacity(user_bundles.len());

        for (b_idx, bundle) in user_bundles.iter().enumerate() {
            let (bundle_size, _) = bundle_sizes.get(b_idx).copied().unwrap_or((0, 0));
            let norm_name = Self::normalize_app_token(&bundle.name);
            let bid_lower = bundle.bundle_id.as_ref().map(|s| s.to_lowercase());

            let mut associated: Vec<AppAssociatedPath> = Vec::new();
            let mut support_bytes = 0u64;
            let mut cache_bytes = 0u64;

            for (l_idx, (cand, sz, _)) in lib_measured.iter().enumerate() {
                if *sz == 0 {
                    continue;
                }
                let entry_lower = cand.entry_name.to_lowercase();
                let entry_norm = Self::normalize_app_token(&cand.entry_name);

                let matches_bid = bid_lower
                    .as_ref()
                    .map(|bid| {
                        entry_lower == *bid
                            || entry_lower.starts_with(&format!("{}.", bid))
                            || entry_lower.contains(bid)
                    })
                    .unwrap_or(false);
                let matches_name = norm_name.len() >= 4
                    && (entry_norm == norm_name
                        || (cand.kind != "launch_agent"
                            && cand.kind != "preferences"
                            && entry_norm.contains(&norm_name)));

                if matches_bid || matches_name {
                    matched_lib_indices.insert(l_idx);
                    support_bytes = support_bytes.saturating_add(*sz);
                    if cand.is_cache_safe {
                        cache_bytes = cache_bytes.saturating_add(*sz);
                    }
                    associated.push(AppAssociatedPath {
                        path: cand.path.to_string_lossy().to_string(),
                        kind: cand.kind.to_string(),
                        size_bytes: *sz,
                        is_cache_safe: cand.is_cache_safe,
                    });
                }
            }

            associated.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
            let is_apple_protected = bid_lower
                .as_ref()
                .map(|s| s.starts_with("com.apple."))
                .unwrap_or(false);

            apps.push(InstalledAppInfo {
                id: format!("app-{}", b_idx),
                name: bundle.name.clone(),
                bundle_id: bundle.bundle_id.clone(),
                version: bundle.version.clone(),
                app_path: bundle.app_path.to_string_lossy().to_string(),
                bundle_size_bytes: bundle_size,
                support_size_bytes: support_bytes,
                cache_size_bytes: cache_bytes,
                associated_paths: associated,
                is_system_app: is_apple_protected,
            });
        }

        apps.sort_by(|a, b| {
            (b.bundle_size_bytes + b.support_size_bytes)
                .cmp(&(a.bundle_size_bytes + a.support_size_bytes))
        });

        let leftovers = Self::extract_orphaned_leftovers(
            &lib_measured,
            &matched_lib_indices,
            &installed_ids,
            &installed_names,
        );

        let total_apps_bytes = apps.iter().map(|a| a.bundle_size_bytes).sum();
        let total_support_bytes = apps.iter().map(|a| a.support_size_bytes).sum();
        let total_leftovers_bytes = leftovers.iter().map(|l| l.size_bytes).sum();

        Ok(ApplicationsInventoryReport {
            apps,
            leftovers,
            total_apps_bytes,
            total_support_bytes,
            total_leftovers_bytes,
            scan_duration_millis: started.elapsed().as_millis() as u64,
        })
    }

    /// Lightweight helper reused by Quick Sweep to surface orphaned app cache/container leftovers.
    pub fn find_app_leftovers() -> Vec<AppLeftoverItem> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let home_path = PathBuf::from(&home);

        let all_bundles = Self::discover_app_bundles(&home_path);
        let installed_ids: HashSet<String> = all_bundles
            .iter()
            .filter_map(|b| b.bundle_id.as_ref().map(|s| s.to_lowercase()))
            .collect();
        let installed_names: HashSet<String> = all_bundles
            .iter()
            .map(|b| Self::normalize_app_token(&b.name))
            .collect();

        let lib_candidates: Vec<LibCandidate> = Self::collect_library_entries(&home_path)
            .into_iter()
            .filter(|c| c.is_cache_safe || c.kind == "container")
            .filter(|c| !Self::is_system_or_active_runtime(&c.entry_name, &installed_ids, &installed_names))
            .collect();

        if lib_candidates.is_empty() {
            return Vec::new();
        }

        let measured: Vec<(LibCandidate, u64, u64)> = std::thread::scope(|scope| {
            let chunk_size = (lib_candidates.len() / 6).max(1);
            let handles: Vec<_> = lib_candidates
                .chunks(chunk_size)
                .map(|chunk| {
                    scope.spawn(move || {
                        chunk
                            .iter()
                            .cloned()
                            .map(|c| {
                                let st = measure_dir_parallel(&c.path);
                                (c, st.size_bytes, st.file_count)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            let mut out = Vec::with_capacity(lib_candidates.len());
            for h in handles {
                if let Ok(v) = h.join() {
                    out.extend(v);
                }
            }
            out
        });

        Self::extract_orphaned_leftovers(&measured, &HashSet::new(), &installed_ids, &installed_names)
    }

    fn is_system_or_active_runtime(
        entry_name: &str,
        installed_ids: &HashSet<String>,
        installed_names: &HashSet<String>,
    ) -> bool {
        let lower = entry_name.to_lowercase();
        let norm = Self::normalize_app_token(entry_name);
        if lower.starts_with("com.apple.")
            || lower.starts_with("apple.")
            || lower.contains("diskatlas")
            || matches!(
                lower.as_str(),
                "homebrew"
                    | "pip"
                    | "uv"
                    | "yarn"
                    | "typescript"
                    | "node-gyp"
                    | "cloudkit"
                    | "metadata"
                    | "coresimulator"
                    | "familycircled"
                    | "geoservices"
            )
        {
            return true;
        }
        if installed_ids
            .iter()
            .any(|bid| lower == *bid || lower.starts_with(&format!("{}.", bid)) || lower.contains(bid))
        {
            return true;
        }
        if norm.len() >= 4 && installed_names.iter().any(|n| n.len() >= 4 && (norm == *n || norm.contains(n))) {
            return true;
        }
        false
    }

    fn extract_orphaned_leftovers(
        lib_measured: &[(LibCandidate, u64, u64)],
        matched_indices: &HashSet<usize>,
        installed_ids: &HashSet<String>,
        installed_names: &HashSet<String>,
    ) -> Vec<AppLeftoverItem> {
        let mut leftovers = Vec::new();
        for (idx, (cand, size_bytes, file_count)) in lib_measured.iter().enumerate() {
            if matched_indices.contains(&idx) || *size_bytes < 1024 * 1024 {
                continue;
            }
            if !cand.is_cache_safe && cand.kind != "container" {
                continue;
            }
            if Self::is_system_or_active_runtime(&cand.entry_name, installed_ids, installed_names) {
                continue;
            }
            // Only flag reverse-DNS bundle identifiers or `.savedState` directories so generic folders
            // (like CLI tool caches) are never misclassified as orphaned app leftovers.
            let is_bundle_like = Self::looks_like_reverse_dns(&cand.entry_name)
                || cand.entry_name.ends_with(".savedState");
            if !is_bundle_like {
                continue;
            }

            let clean_hint = cand
                .entry_name
                .strip_suffix(".savedState")
                .unwrap_or(&cand.entry_name)
                .to_string();

            leftovers.push(AppLeftoverItem {
                id: format!("leftover-{}", idx),
                app_hint: clean_hint.clone(),
                bundle_id: if Self::looks_like_reverse_dns(&clean_hint) {
                    Some(clean_hint)
                } else {
                    None
                },
                path: cand.path.to_string_lossy().to_string(),
                kind: cand.kind.to_string(),
                size_bytes: *size_bytes,
                file_count: *file_count,
                is_cache_safe: cand.is_cache_safe,
                reason: format!(
                    "Residual {} directory matching no installed application bundle in /Applications or ~/Applications.",
                    cand.kind.replace('_', " ")
                ),
            });
        }
        leftovers.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        leftovers
    }

    fn looks_like_reverse_dns(name: &str) -> bool {
        let base = name.strip_suffix(".savedState").unwrap_or(name);
        let parts: Vec<&str> = base.split('.').collect();
        parts.len() >= 3
            && matches!(
                parts[0].to_ascii_lowercase().as_str(),
                "com" | "org" | "net" | "io" | "dev" | "app" | "ai" | "co" | "me" | "us" | "uk" | "de" | "fr" | "jp" | "cn"
            )
            && parts[1].len() >= 2
    }

    fn discover_app_bundles(home_path: &Path) -> Vec<RawBundle> {
        #[cfg(target_os = "macos")]
        let search_roots = vec![
            (PathBuf::from("/Applications"), false),
            (PathBuf::from("/Applications/Utilities"), false),
            (home_path.join("Applications"), false),
            (PathBuf::from("/System/Applications"), true),
            (PathBuf::from("/System/Applications/Utilities"), true),
        ];

        #[cfg(not(target_os = "macos"))]
        let search_roots: Vec<(PathBuf, bool)> = {
            let _ = home_path;
            Vec::new()
        };

        let mut seen_paths = HashSet::new();
        let mut bundles = Vec::new();

        for (root, is_sys) in search_roots {
            let Ok(entries) = fs::read_dir(&root) else {
                continue;
            };
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) != Some("app") || !p.is_dir() {
                    continue;
                }
                if !seen_paths.insert(p.clone()) {
                    continue;
                }
                let fallback_name = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Application")
                    .to_string();
                let plist_path = p.join("Contents/Info.plist");
                let meta = Self::parse_info_plist(&plist_path);
                bundles.push(RawBundle {
                    name: meta
                        .get("CFBundleDisplayName")
                        .or_else(|| meta.get("CFBundleName"))
                        .cloned()
                        .unwrap_or(fallback_name),
                    bundle_id: meta.get("CFBundleIdentifier").cloned(),
                    version: meta
                        .get("CFBundleShortVersionString")
                        .or_else(|| meta.get("CFBundleVersion"))
                        .cloned(),
                    app_path: p,
                    is_system: is_sys,
                });
            }
        }

        bundles
    }

    fn collect_library_entries(home_path: &Path) -> Vec<LibCandidate> {
        #[cfg(target_os = "macos")]
        {
            let lib = home_path.join("Library");
            let specs = [
                (lib.join("Caches"), "cache", true),
                (lib.join("Saved Application State"), "saved_state", true),
                (lib.join("HTTPStorages"), "http_storage", true),
                (lib.join("WebKit"), "webkit", true),
                (lib.join("Logs"), "logs", true),
                (lib.join("Containers"), "container", false),
                (lib.join("Application Support"), "support", false),
                (lib.join("Preferences"), "preferences", false),
                (lib.join("LaunchAgents"), "launch_agent", false),
            ];

            let mut out = Vec::new();
            for (dir, kind, is_cache_safe) in specs {
                let Ok(entries) = fs::read_dir(&dir) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let p = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    out.push(LibCandidate {
                        path: p,
                        entry_name: name,
                        kind,
                        is_cache_safe,
                    });
                }
            }
            out
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = home_path;
            Vec::new()
        }
    }

    fn normalize_app_token(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }

    fn parse_info_plist(plist_path: &Path) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let Ok(bytes) = fs::read(plist_path) else {
            return map;
        };

        let xml_owned: String;
        let xml_str: &str = if bytes.starts_with(b"bplist") {
            let output = Command::new("/usr/bin/plutil")
                .args(["-convert", "xml1", "-o", "-"])
                .arg(plist_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output();
            match output {
                Ok(out) if out.status.success() => {
                    xml_owned = String::from_utf8_lossy(&out.stdout).to_string();
                    &xml_owned
                }
                _ => return map,
            }
        } else {
            xml_owned = String::from_utf8_lossy(&bytes).to_string();
            &xml_owned
        };

        for key in [
            "CFBundleIdentifier",
            "CFBundleDisplayName",
            "CFBundleName",
            "CFBundleShortVersionString",
            "CFBundleVersion",
        ] {
            if let Some(val) = Self::extract_xml_plist_string(xml_str, key) {
                map.insert(key.to_string(), val);
            }
        }
        map
    }

    fn extract_xml_plist_string(xml: &str, key: &str) -> Option<String> {
        let needle = format!("<key>{}</key>", key);
        let pos = xml.find(&needle)?;
        let after_key = &xml[pos + needle.len()..];
        let str_start = after_key.find("<string>")?;
        // Ensure another <key> didn't appear before <string> (e.g. non-string value)
        if let Some(next_key) = after_key.find("<key>") {
            if next_key < str_start {
                return None;
            }
        }
        let val_start = str_start + "<string>".len();
        let rest = &after_key[val_start..];
        let val_end = rest.find("</string>")?;
        let raw = rest[..val_end].trim();
        if raw.is_empty() {
            None
        } else {
            Some(
                raw.replace("&amp;", "&")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">"),
            )
        }
    }
}

#[derive(Debug, Clone)]
struct LibCandidate {
    path: PathBuf,
    entry_name: String,
    kind: &'static str,
    is_cache_safe: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_plist_extraction_and_reverse_dns() {
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <plist version="1.0">
        <dict>
            <key>CFBundleDisplayName</key>
            <string>Acme &amp; Co</string>
            <key>CFBundleIdentifier</key>
            <string>com.acme.studio</string>
            <key>CFBundleShortVersionString</key>
            <string>2.4.1</string>
        </dict>
        </plist>
        "#;
        assert_eq!(
            ApplicationsService::extract_xml_plist_string(xml, "CFBundleDisplayName").as_deref(),
            Some("Acme & Co")
        );
        assert_eq!(
            ApplicationsService::extract_xml_plist_string(xml, "CFBundleIdentifier").as_deref(),
            Some("com.acme.studio")
        );
        assert!(ApplicationsService::looks_like_reverse_dns("com.acme.studio"));
        assert!(ApplicationsService::looks_like_reverse_dns("io.example.App.savedState"));
        assert!(!ApplicationsService::looks_like_reverse_dns("random_folder"));
    }
}
