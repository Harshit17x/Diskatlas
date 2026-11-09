use std::path::{Path, PathBuf};
use std::process::Command;
use rusqlite::params;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::Result;
use crate::models::developer::{
    DeveloperAnalysisProfile, DeveloperEcosystemSummary, DeveloperProjectItem,
    DockerStorageInfo, GlobalToolCacheItem,
};
use crate::storage::DbPool;

pub struct DeveloperService;

/// Case-insensitive substring test that does not allocate a lowercased copy of the haystack.
///
/// The developer analysis classifies every file in a session, so allocating a lowercased
/// copy of each path (millions of times) was a significant cost.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() {
        return true;
    }
    if n.len() > h.len() {
        return false;
    }
    h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

impl DeveloperService {
    /// Discovers all software projects in a scan session, aggregates dependencies, caches, and build outputs.
    pub fn analyze_developer_storage(
        db_pool: &DbPool,
        session_id: &str,
    ) -> Result<DeveloperAnalysisProfile> {
        let conn_arc = db_pool.conn();
        let mut conn = conn_arc.lock().unwrap();

        // 1. Query all project markers in this session
        let mut marker_stmt = conn.prepare(
            "SELECT DISTINCT parent_path, name
             FROM files
             WHERE session_id = ?1
               AND name IN (
                 'package.json', 'Cargo.toml', 'pyproject.toml', 'setup.py', 'pom.xml',
                 'build.gradle', 'build.gradle.kts', 'CMakeLists.txt', '.git'
               )
             ORDER BY LENGTH(parent_path) ASC;",
        )?;

        struct Marker {
            parent_path: String,
            name: String,
        }

        let marker_rows = marker_stmt.query_map([session_id], |row| {
            Ok(Marker {
                parent_path: row.get(0)?,
                name: row.get(1)?,
            })
        })?;

        let mut markers = Vec::new();
        for m in marker_rows {
            markers.push(m?);
        }
        drop(marker_stmt);

        // Group markers by project root
        use std::collections::{HashMap, HashSet};
        let mut project_roots: HashMap<String, Vec<String>> = HashMap::new();
        for m in markers {
            project_roots
                .entry(m.parent_path.clone())
                .or_default()
                .push(m.name);
        }

        // Cap the number of projects analyzed in a single pass. A pathological filesystem
        // (e.g. a monorepo with thousands of vendored checkouts) can produce tens of
        // thousands of markers; analyzing them all would hold the database lock for a very
        // long time. The largest projects are kept.
        const MAX_PROJECTS: usize = 25_000;
        let mut root_list: Vec<(String, Vec<String>)> = project_roots.into_iter().collect();
        if root_list.len() > MAX_PROJECTS {
            root_list.sort_by(|a, b| a.0.cmp(&b.0));
            root_list.truncate(MAX_PROJECTS);
            warn!(
                "Developer analysis capped at {} projects for session '{}'",
                MAX_PROJECTS, session_id
            );
        }

        // Root lookup set, used to attribute each file to its owning project.
        let root_set: HashSet<String> = root_list.iter().map(|(r, _)| r.clone()).collect();

        // 2. Aggregate every file in the session in a SINGLE ordered pass.
        //
        // The previous implementation ran one `path LIKE 'root/%'` query per project root.
        // Because that predicate cannot use the (session_id, path) index, each query scanned
        // the whole session's file table — with ~19,000 roots and ~3.8M files that is
        // roughly 7.3e10 row visits (measured at ~72 seconds for a *single* root), so the
        // pipeline could never finish and the Developer Storage view stayed empty.
        //
        // Instead we stream the session's files once in path order and attribute each file
        // to the deepest project root that is an ancestor of its path. This is a single
        // sequential scan regardless of how many projects were discovered.
        #[derive(Default, Clone, Copy)]
        struct ProjectAgg {
            dep_bytes: u64,
            cache_bytes: u64,
            build_bytes: u64,
            total_bytes: u64,
            file_count: u64,
            max_modified: Option<i64>,
        }

        let mut aggregates: HashMap<String, ProjectAgg> = HashMap::new();

        {
            let mut file_stmt = conn.prepare(
                "SELECT path, size_bytes, modified_at
                 FROM files
                 WHERE session_id = ?1
                 ORDER BY path;",
            )?;

            let mut rows = file_stmt.query(params![session_id])?;
            // Reusable buffer for ancestor-path construction (avoids per-file allocation).
            let mut ancestor: String = String::with_capacity(512);

            while let Some(row) = rows.next()? {
                let fpath: String = row.get(0)?;
                let fsize: i64 = row.get(1)?;
                let fmod: Option<i64> = row.get(2)?;

                // Find the deepest project root that is an ancestor of this file.
                ancestor.clear();
                ancestor.push_str(&fpath);
                let mut owner: Option<String> = None;

                loop {
                    if root_set.contains(ancestor.as_str()) {
                        owner = Some(ancestor.clone());
                        break;
                    }
                    // Strip the last path component and try again.
                    match ancestor.rfind('/') {
                        Some(0) => {
                            // Reached the filesystem root.
                            if root_set.contains("/") {
                                owner = Some("/".to_string());
                            }
                            break;
                        }
                        Some(idx) => ancestor.truncate(idx),
                        None => break,
                    }
                }

                let Some(owner) = owner else { continue };

                let fsize = fsize.max(0) as u64;
                let agg = aggregates.entry(owner).or_default();
                agg.total_bytes += fsize;
                agg.file_count += 1;
                if let Some(m) = fmod {
                    agg.max_modified = Some(agg.max_modified.map_or(m, |curr| curr.max(m)));
                }

                // Classify the file's role. `contains_ci` performs a case-insensitive
                // substring test without allocating a lowercased copy of the path.
                if contains_ci(&fpath, "/node_modules/")
                    || contains_ci(&fpath, "/.venv/")
                    || contains_ci(&fpath, "/venv/")
                    || contains_ci(&fpath, "/pods/")
                    || contains_ci(&fpath, "/.pnpm-store/")
                {
                    agg.dep_bytes += fsize;
                } else if contains_ci(&fpath, "/.next/cache/")
                    || contains_ci(&fpath, "/.vite/")
                    || contains_ci(&fpath, "/.turbo/")
                    || contains_ci(&fpath, "/.parcel-cache/")
                    || contains_ci(&fpath, "/__pycache__/")
                    || contains_ci(&fpath, "/.pytest_cache/")
                    || contains_ci(&fpath, "/.mypy_cache/")
                    || contains_ci(&fpath, "/.ruff_cache/")
                    || contains_ci(&fpath, "/.gradle/caches/")
                {
                    agg.cache_bytes += fsize;
                } else if contains_ci(&fpath, "/target/")
                    || contains_ci(&fpath, "/dist/")
                    || contains_ci(&fpath, "/build/")
                    || contains_ci(&fpath, "/.next/")
                    || contains_ci(&fpath, "/.nuxt/")
                    || contains_ci(&fpath, "/.output/")
                    || contains_ci(&fpath, "/cmake-build-debug/")
                    || contains_ci(&fpath, "/cmake-build-release/")
                    || contains_ci(&fpath, "/debug/")
                    || contains_ci(&fpath, "/release/")
                    || contains_ci(&fpath, "/coverage/")
                    || contains_ci(&fpath, "/deriveddata/")
                {
                    agg.build_bytes += fsize;
                }
            }
        }

        // 3. Persist the per-project results.
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM developer_projects WHERE session_id = ?1;",
            [session_id],
        )?;
        tx.execute(
            "DELETE FROM developer_ecosystem_summaries WHERE session_id = ?1;",
            [session_id],
        )?;

        // 4. Analyze each discovered project
        let mut projects = Vec::new();

        for (root_path, file_markers) in root_list {
            let ecosystem = Self::detect_ecosystem(&file_markers);
            let name = Self::extract_project_name(&root_path);
            let project_id = Uuid::new_v4().to_string();

            let agg = aggregates.get(&root_path).cloned().unwrap_or_default();
            let dep_bytes = agg.dep_bytes;
            let cache_bytes = agg.cache_bytes;
            let build_bytes = agg.build_bytes;
            let total_bytes = agg.total_bytes;
            let file_count = agg.file_count;
            let max_modified = agg.max_modified;

            let confidence = if file_markers.len() > 1 { 0.98 } else { 0.90 };

            tx.execute(
                "INSERT INTO developer_projects (
                    id, session_id, name, root_path, ecosystem,
                    dependency_bytes, cache_bytes, build_output_bytes, total_bytes,
                    file_count, last_modified_at, confidence
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12);",
                params![
                    project_id,
                    session_id,
                    name,
                    root_path,
                    ecosystem,
                    dep_bytes as i64,
                    cache_bytes as i64,
                    build_bytes as i64,
                    total_bytes as i64,
                    file_count as i64,
                    max_modified,
                    confidence,
                ],
            )?;

            projects.push(DeveloperProjectItem {
                id: project_id,
                session_id: session_id.to_string(),
                name,
                root_path,
                ecosystem,
                dependency_bytes: dep_bytes,
                cache_bytes,
                build_output_bytes: build_bytes,
                total_bytes,
                file_count,
                last_modified_at: max_modified,
                confidence,
            });
        }

        // Sort projects by total size descending
        projects.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));

        // 4. Aggregate ecosystem summaries
        tx.execute(
            "INSERT INTO developer_ecosystem_summaries (
                session_id, ecosystem, project_count, total_bytes, dependency_bytes, cache_bytes, build_bytes
             )
             SELECT
                session_id, ecosystem, COUNT(*),
                COALESCE(SUM(total_bytes), 0),
                COALESCE(SUM(dependency_bytes), 0),
                COALESCE(SUM(cache_bytes), 0),
                COALESCE(SUM(build_output_bytes), 0)
             FROM developer_projects
             WHERE session_id = ?1
             GROUP BY ecosystem;",
            [session_id],
        )?;

        let mut eco_stmt = tx.prepare(
            "SELECT ecosystem, project_count, total_bytes, dependency_bytes, cache_bytes, build_bytes
             FROM developer_ecosystem_summaries
             WHERE session_id = ?1
             ORDER BY total_bytes DESC;",
        )?;

        let eco_rows = eco_stmt.query_map([session_id], |r| {
            Ok(DeveloperEcosystemSummary {
                ecosystem: r.get(0)?,
                project_count: r.get::<_, i64>(1)? as u64,
                total_bytes: r.get::<_, i64>(2)? as u64,
                dependency_bytes: r.get::<_, i64>(3)? as u64,
                cache_bytes: r.get::<_, i64>(4)? as u64,
                build_bytes: r.get::<_, i64>(5)? as u64,
            })
        })?;

        let mut ecosystems = Vec::new();
        for e in eco_rows {
            ecosystems.push(e?);
        }
        drop(eco_stmt);

        tx.commit()?;
        drop(conn);

        let total_dev_bytes: u64 = projects.iter().map(|p| p.total_bytes).sum();
        let total_projects = projects.len() as u64;

        // 5. Query live Docker storage metrics
        let docker = Self::inspect_docker_storage();

        // 6. Query global tool and package manager caches
        let global_caches = Self::inspect_global_caches();

        info!(
            "Analyzed {} developer projects ({} total bytes) for session '{}'",
            total_projects, total_dev_bytes, session_id
        );

        Ok(DeveloperAnalysisProfile {
            session_id: session_id.to_string(),
            total_developer_bytes: total_dev_bytes,
            total_projects,
            ecosystems,
            projects,
            docker,
            global_caches,
        })
    }

    fn detect_ecosystem(markers: &[String]) -> String {
        let has_node = markers.iter().any(|m| m == "package.json");
        let has_rust = markers.iter().any(|m| m == "Cargo.toml");
        let has_python = markers.iter().any(|m| m == "pyproject.toml" || m == "setup.py");
        let has_java = markers.iter().any(|m| m == "pom.xml" || m.starts_with("build.gradle"));
        let has_cpp = markers.iter().any(|m| m == "CMakeLists.txt");

        let mut matches = Vec::new();
        if has_node { matches.push("Node.js"); }
        if has_rust { matches.push("Rust"); }
        if has_python { matches.push("Python"); }
        if has_java { matches.push("Java"); }
        if has_cpp { matches.push("C/C++"); }

        match matches.len() {
            0 => "Git Repository".to_string(),
            1 => matches[0].to_string(),
            _ => format!("Multi-stack ({})", matches.join(" + ")),
        }
    }

    fn extract_project_name(path_str: &str) -> String {
        let p = Path::new(path_str);
        p.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Developer Project".to_string())
    }

    /// Safely inspects Docker engine storage metrics via supported CLI command.
    /// Never touches Docker internal filesystem layers directly.
    pub fn inspect_docker_storage() -> DockerStorageInfo {
        let output = Command::new("docker")
            .args(["system", "df", "--format", "{{.Type}}\t{{.TotalCount}}\t{{.Size}}\t{{.Reclaimable}}"])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                Self::parse_docker_df_output(&stdout)
            }
            Ok(_) => DockerStorageInfo {
                is_available: false,
                images_bytes: 0,
                images_count: 0,
                containers_bytes: 0,
                containers_count: 0,
                volumes_bytes: 0,
                volumes_count: 0,
                build_cache_bytes: 0,
                total_bytes: 0,
                status_message: "Docker daemon is offline or inaccessible".to_string(),
            },
            Err(_) => DockerStorageInfo {
                is_available: false,
                images_bytes: 0,
                images_count: 0,
                containers_bytes: 0,
                containers_count: 0,
                volumes_bytes: 0,
                volumes_count: 0,
                build_cache_bytes: 0,
                total_bytes: 0,
                status_message: "Docker CLI is not installed on PATH".to_string(),
            },
        }
    }

    pub fn parse_docker_df_output(output_str: &str) -> DockerStorageInfo {
        let mut images_bytes: u64 = 0;
        let mut images_count: u64 = 0;
        let mut containers_bytes: u64 = 0;
        let mut containers_count: u64 = 0;
        let mut volumes_bytes: u64 = 0;
        let mut volumes_count: u64 = 0;
        let mut build_cache_bytes: u64 = 0;

        for line in output_str.lines() {
            let cols: Vec<&str> = line.split('\t').map(|s| s.trim()).collect();
            if cols.len() >= 3 {
                let item_type = cols[0].to_lowercase();
                let count: u64 = cols[1].parse().unwrap_or(0);
                let size = Self::parse_size_bytes(cols[2]);

                if item_type.contains("images") {
                    images_bytes = size;
                    images_count = count;
                } else if item_type.contains("containers") {
                    containers_bytes = size;
                    containers_count = count;
                } else if item_type.contains("local volumes") || item_type.contains("volumes") {
                    volumes_bytes = size;
                    volumes_count = count;
                } else if item_type.contains("build cache") {
                    build_cache_bytes = size;
                }
            }
        }

        let total = images_bytes + containers_bytes + volumes_bytes + build_cache_bytes;

        DockerStorageInfo {
            is_available: true,
            images_bytes,
            images_count,
            containers_bytes,
            containers_count,
            volumes_bytes,
            volumes_count,
            build_cache_bytes,
            total_bytes: total,
            status_message: "Docker engine connected".to_string(),
        }
    }

    pub fn parse_size_bytes(s: &str) -> u64 {
        let clean = s.trim().replace(',', "");
        let clean = clean.split_whitespace().next().unwrap_or("");

        let num_part: String = clean.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
        let unit_part: String = clean.chars().skip_while(|c| c.is_ascii_digit() || *c == '.').collect();

        let num: f64 = num_part.parse().unwrap_or(0.0);
        let unit = unit_part.to_uppercase();

        let multiplier: f64 = if unit.starts_with("TB") || unit.starts_with("TIB") {
            1024.0 * 1024.0 * 1024.0 * 1024.0
        } else if unit.starts_with("GB") || unit.starts_with("GIB") {
            1024.0 * 1024.0 * 1024.0
        } else if unit.starts_with("MB") || unit.starts_with("MIB") {
            1024.0 * 1024.0
        } else if unit.starts_with("KB") || unit.starts_with("KIB") || unit.starts_with('K') {
            1024.0
        } else {
            1.0
        };

        (num * multiplier) as u64
    }

    /// Inspects known global package manager and tooling caches on host disk.
    pub fn inspect_global_caches() -> Vec<GlobalToolCacheItem> {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".to_string());
        let home_path = PathBuf::from(&home);

        #[cfg(target_os = "windows")]
        let (pnpm_store, android_sdk) = (
            std::env::var("LOCALAPPDATA")
                .map(|l| PathBuf::from(l).join("pnpm/store"))
                .unwrap_or_else(|_| home_path.join("AppData/Local/pnpm/store")),
            std::env::var("LOCALAPPDATA")
                .map(|l| PathBuf::from(l).join("Android/Sdk"))
                .unwrap_or_else(|_| home_path.join("AppData/Local/Android/Sdk")),
        );

        #[cfg(target_os = "macos")]
        let (pnpm_store, android_sdk) = (
            home_path.join("Library/pnpm/store"),
            home_path.join("Library/Android/sdk"),
        );

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let (pnpm_store, android_sdk) = (
            home_path.join(".local/share/pnpm/store"),
            home_path.join("Android/Sdk"),
        );

        let targets = vec![
            ("Cargo Registry", home_path.join(".cargo/registry")),
            ("Cargo Git Caches", home_path.join(".cargo/git")),
            ("npm Global Cache", home_path.join(".npm")),
            ("pnpm Global Store", pnpm_store),
            ("Yarn Global Cache", home_path.join(".cache/yarn")),
            ("pip Global Cache", home_path.join(".cache/pip")),
            ("uv Global Cache", home_path.join(".cache/uv")),
            ("Maven .m2 Repository", home_path.join(".m2/repository")),
            ("Gradle Caches", home_path.join(".gradle/caches")),
            ("Android SDK & Emulators", android_sdk),
            ("Xcode DerivedData", home_path.join("Library/Developer/Xcode/DerivedData")),
        ];

        let mut items = Vec::new();

        for (name, path) in targets {
            let exists = path.exists();
            let (size, count) = if exists {
                Self::calculate_dir_size(&path)
            } else {
                (0, 0)
            };

            items.push(GlobalToolCacheItem {
                tool: name.to_string(),
                path: path.to_string_lossy().to_string(),
                size_bytes: size,
                file_count: count,
                exists,
            });
        }

        items
    }

    fn calculate_dir_size(path: &Path) -> (u64, u64) {
        let mut total_size: u64 = 0;
        let mut file_count: u64 = 0;
        let mut stack = vec![path.to_path_buf()];
        let mut visited: usize = 0;

        while let Some(current) = stack.pop() {
            visited += 1;
            if visited > 50_000 {
                // Safeguard against unbounded traversal
                break;
            }
            if let Ok(entries) = std::fs::read_dir(&current) {
                for entry in entries.flatten() {
                    if let Ok(meta) = entry.metadata() {
                        if meta.is_file() {
                            total_size += meta.len();
                            file_count += 1;
                        } else if meta.is_dir() && !meta.file_type().is_symlink() {
                            stack.push(entry.path());
                        }
                    }
                }
            }
        }

        (total_size, file_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrator::Migrator;

    #[test]
    fn test_parse_size_bytes() {
        assert_eq!(DeveloperService::parse_size_bytes("0B"), 0);
        assert_eq!(DeveloperService::parse_size_bytes("100B"), 100);
        assert_eq!(DeveloperService::parse_size_bytes("10KB"), 10240);
        assert_eq!(DeveloperService::parse_size_bytes("1.5MB"), 1572864);
        assert_eq!(DeveloperService::parse_size_bytes("2GB"), 2147483648);
    }

    #[test]
    fn test_parse_docker_df_output() {
        let sample = "Images\t12\t4.5GB\t1.2GB (26%)\nContainers\t3\t250MB\t50MB (20%)\nLocal Volumes\t5\t1.8GB\t800MB (44%)\nBuild Cache\t24\t3.2GB\t1.5GB (46%)\n";
        let info = DeveloperService::parse_docker_df_output(sample);

        assert!(info.is_available);
        assert_eq!(info.images_count, 12);
        assert!(info.images_bytes > 4_000_000_000);
        assert_eq!(info.containers_count, 3);
        assert!(info.containers_bytes > 200_000_000);
        assert_eq!(info.volumes_count, 5);
        assert!(info.volumes_bytes > 1_500_000_000);
        assert!(info.build_cache_bytes > 3_000_000_000);
        assert!(info.total_bytes > 9_000_000_000);
    }

    #[test]
    fn test_developer_project_detection_and_aggregation() {
        let temp_dir = std::env::temp_dir().join(format!("diskatlas_dev_test_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("dev_test.db");
        let pool = DbPool::new(&db_path).unwrap();

        {
            let conn_arc = pool.conn();
            let mut conn = conn_arc.lock().unwrap();
            Migrator::run(&mut conn).unwrap();

            conn.execute(
                "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
                 VALUES ('TEST_DEV_DRIVE', 'Test Drive', '/work', 'apfs', 100000000, 50000000);",
                (),
            ).unwrap();

            conn.execute(
                "INSERT INTO scan_sessions (id, drive_id, root_path, status, total_files, total_directories, total_bytes)
                 VALUES ('session_dev_1', 'TEST_DEV_DRIVE', '/work', 'completed', 6, 4, 150000);",
                (),
            ).unwrap();

            // Insert simulated project files (Node.js project with node_modules & .next/cache)
            conn.execute(
                "INSERT INTO files (session_id, path, name, extension, parent_path, size_bytes, modified_at)
                 VALUES
                 ('session_dev_1', '/work/web-app/package.json', 'package.json', 'json', '/work/web-app', 1000, 500),
                 ('session_dev_1', '/work/web-app/node_modules/react/index.js', 'index.js', 'js', '/work/web-app/node_modules/react', 50000, 500),
                 ('session_dev_1', '/work/web-app/.next/cache/webpack.pack', 'webpack.pack', 'pack', '/work/web-app/.next/cache', 30000, 500),
                 ('session_dev_1', '/work/web-app/dist/bundle.js', 'bundle.js', 'js', '/work/web-app/dist', 20000, 500),
                 ('session_dev_1', '/work/rust-cli/Cargo.toml', 'Cargo.toml', 'toml', '/work/rust-cli', 500, 600),
                 ('session_dev_1', '/work/rust-cli/target/debug/cli', 'cli', '', '/work/rust-cli/target/debug', 48500, 600);",
                (),
            ).unwrap();
        }

        let profile = DeveloperService::analyze_developer_storage(&pool, "session_dev_1").unwrap();

        assert_eq!(profile.total_projects, 2);
        assert_eq!(profile.projects.len(), 2);

        // Find web-app
        let web_app = profile.projects.iter().find(|p| p.name == "web-app").unwrap();
        assert_eq!(web_app.ecosystem, "Node.js");
        assert_eq!(web_app.dependency_bytes, 50000);
        assert_eq!(web_app.cache_bytes, 30000);
        assert_eq!(web_app.build_output_bytes, 20000);
        assert_eq!(web_app.total_bytes, 101000);

        // Find rust-cli
        let rust_cli = profile.projects.iter().find(|p| p.name == "rust-cli").unwrap();
        assert_eq!(rust_cli.ecosystem, "Rust");
        assert_eq!(rust_cli.build_output_bytes, 48500);
        assert_eq!(rust_cli.total_bytes, 49000);
    }
}
