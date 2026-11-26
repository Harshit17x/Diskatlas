use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use rusqlite::params;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::Result;
use crate::models::developer::{
    DeveloperAnalysisProfile, DeveloperEcosystemSummary, DeveloperProjectItem,
    DockerStorageInfo, GlobalToolCacheItem,
};
use crate::storage::{resolve_session_id, DbPool};

pub struct DeveloperService;

/// Checks for a path segment regardless of `/` vs `\` separator.
fn contains_segment_ci(path: &str, segment: &str) -> bool {
    let bytes = path.as_bytes();
    let seg = segment.as_bytes();
    if seg.is_empty() || bytes.len() < seg.len() {
        return false;
    }
    for i in 0..=bytes.len() - seg.len() {
        let prev_ok = i == 0 || bytes[i - 1] == b'/' || bytes[i - 1] == b'\\';
        if !prev_ok {
            continue;
        }
        let end = i + seg.len();
        let next_ok = end == bytes.len() || bytes[end] == b'/' || bytes[end] == b'\\';
        if next_ok && bytes[i..end].eq_ignore_ascii_case(seg) {
            return true;
        }
    }
    false
}

/// Checks if a two-segment relative sequence like `.next/cache` or `.gradle/caches` appears with either separator.
fn contains_subpath_ci(path: &str, part1: &str, part2: &str) -> bool {
    let bytes = path.as_bytes();
    let p1 = part1.as_bytes();
    let p2 = part2.as_bytes();
    let total = p1.len() + 1 + p2.len();
    if bytes.len() < total {
        return false;
    }
    for i in 0..=bytes.len() - total {
        let prev_ok = i == 0 || bytes[i - 1] == b'/' || bytes[i - 1] == b'\\';
        if !prev_ok {
            continue;
        }
        let sep_idx = i + p1.len();
        let sep_ok = bytes[sep_idx] == b'/' || bytes[sep_idx] == b'\\';
        let end = sep_idx + 1 + p2.len();
        let next_ok = end == bytes.len() || bytes[end] == b'/' || bytes[end] == b'\\';
        if sep_ok
            && next_ok
            && bytes[i..sep_idx].eq_ignore_ascii_case(p1)
            && bytes[sep_idx + 1..end].eq_ignore_ascii_case(p2)
        {
            return true;
        }
    }
    false
}

/// Returns true when a project marker resides inside vendored dependencies, tool caches,
/// or build output folders (e.g. `node_modules/react/package.json`), which must roll up
/// to their enclosing project rather than creating thousands of spurious sub-projects.
fn is_inside_dependency_or_cache_tree(path: &str) -> bool {
    contains_segment_ci(path, "node_modules")
        || contains_segment_ci(path, ".pnpm-store")
        || contains_segment_ci(path, ".cargo")
        || contains_segment_ci(path, ".rustup")
        || contains_segment_ci(path, ".venv")
        || contains_segment_ci(path, "venv")
        || contains_segment_ci(path, "site-packages")
        || contains_segment_ci(path, "Pods")
        || contains_segment_ci(path, ".gradle")
        || contains_segment_ci(path, ".m2")
        || contains_segment_ci(path, "target")
        || contains_segment_ci(path, "DerivedData")
        || contains_segment_ci(path, ".git")
}

impl DeveloperService {
    /// Returns developer storage profile for a session, serving cached DB results when already
    /// analyzed unless `force_refresh` is true.
    pub fn get_developer_storage(
        db_pool: &DbPool,
        session_id: Option<&str>,
        force_refresh: bool,
    ) -> Result<DeveloperAnalysisProfile> {
        let sid = db_pool.with_read_conn(|conn| resolve_session_id(conn, session_id))?;

        if !force_refresh {
            if let Some(cached) = Self::load_cached_profile(db_pool, &sid)? {
                let docker = Self::inspect_docker_storage();
                let global_caches = Self::inspect_global_caches();
                return Ok(DeveloperAnalysisProfile {
                    session_id: cached.0,
                    total_developer_bytes: cached.1,
                    total_projects: cached.2,
                    ecosystems: cached.3,
                    projects: cached.4,
                    docker,
                    global_caches,
                });
            }
        }

        let (projects, ecosystems, total_dev_bytes, total_projects) =
            Self::analyze_project_storage_db(db_pool, &sid)?;

        let docker = Self::inspect_docker_storage();
        let global_caches = Self::inspect_global_caches();

        Ok(DeveloperAnalysisProfile {
            session_id: sid,
            total_developer_bytes: total_dev_bytes,
            total_projects,
            ecosystems,
            projects,
            docker,
            global_caches,
        })
    }

    /// Discovers all software projects in a scan session and persists them in SQLite.
    pub fn analyze_developer_storage(
        db_pool: &DbPool,
        session_id: &str,
    ) -> Result<DeveloperAnalysisProfile> {
        Self::get_developer_storage(db_pool, Some(session_id), true)
    }

    /// DB-only project attribution pass (suitable for post-scan finalization without spawning
    /// Docker CLI or walking host global cache directories).
    pub fn analyze_project_storage_db(
        db_pool: &DbPool,
        session_id: &str,
    ) -> Result<(
        Vec<DeveloperProjectItem>,
        Vec<DeveloperEcosystemSummary>,
        u64,
        u64,
    )> {
        let sid = db_pool.with_read_conn(|conn| resolve_session_id(conn, Some(session_id)))?;

        // 1. Query file-based project markers AND `.git` directory markers on a read connection
        let project_roots = db_pool.with_read_conn(|conn| {
            let mut roots: HashMap<String, Vec<String>> = HashMap::new();

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

            let marker_rows = marker_stmt.query_map([&sid], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;

            for m in marker_rows {
                let (parent_path, name) = m?;
                if !is_inside_dependency_or_cache_tree(&parent_path) {
                    roots.entry(parent_path).or_default().push(name);
                }
            }

            // Detect standard Git repositories where `.git` is a directory
            let mut git_stmt = conn.prepare(
                "SELECT DISTINCT parent_path
                 FROM directories
                 WHERE session_id = ?1
                   AND name = '.git'
                   AND parent_path IS NOT NULL;",
            )?;
            let git_rows = git_stmt.query_map([&sid], |row| row.get::<_, String>(0))?;
            for g in git_rows {
                let repo_root = g?;
                if !is_inside_dependency_or_cache_tree(&repo_root) {
                    let list = roots.entry(repo_root).or_default();
                    if !list.iter().any(|m| m == ".git") {
                        list.push(".git".to_string());
                    }
                }
            }

            Ok(roots)
        })?;

        // Cap the number of projects analyzed in a single pass, preferring shallowest roots first.
        const MAX_PROJECTS: usize = 25_000;
        let mut root_list: Vec<(String, Vec<String>)> = project_roots.into_iter().collect();
        if root_list.len() > MAX_PROJECTS {
            root_list.sort_by(|a, b| a.0.len().cmp(&b.0.len()).then_with(|| a.0.cmp(&b.0)));
            root_list.truncate(MAX_PROJECTS);
            warn!(
                "Developer analysis capped at {} projects for session '{}'",
                MAX_PROJECTS, sid
            );
        }

        let root_set: HashSet<String> = root_list.iter().map(|(r, _)| r.clone()).collect();

        // 2. Single sequential scan over session files on a read connection
        #[derive(Default, Clone, Copy)]
        struct ProjectAgg {
            dep_bytes: u64,
            cache_bytes: u64,
            build_bytes: u64,
            total_bytes: u64,
            file_count: u64,
            max_modified: Option<i64>,
        }

        let aggregates: HashMap<String, ProjectAgg> = if root_set.is_empty() {
            HashMap::new()
        } else {
            db_pool.with_read_conn(|conn| {
                let mut aggs: HashMap<String, ProjectAgg> = HashMap::new();
                let mut file_stmt = conn.prepare(
                    "SELECT path, size_bytes, modified_at
                     FROM files
                     WHERE session_id = ?1;",
                )?;

                let mut rows = file_stmt.query(params![&sid])?;
                let mut ancestor: String = String::with_capacity(512);

                while let Some(row) = rows.next()? {
                    let fpath: String = row.get(0)?;
                    let fsize: i64 = row.get(1)?;
                    let fmod: Option<i64> = row.get(2)?;

                    ancestor.clear();
                    ancestor.push_str(&fpath);
                    let mut owner: Option<String> = None;

                    loop {
                        if root_set.contains(ancestor.as_str()) {
                            owner = Some(ancestor.clone());
                            break;
                        }
                        // Strip the last path component (supporting both `/` and `\`)
                        match ancestor.rfind(&['/', '\\'][..]) {
                            Some(0) => {
                                if root_set.contains("/") {
                                    owner = Some("/".to_string());
                                }
                                break;
                            }
                            Some(idx) => {
                                // Handle Windows drive root e.g. `C:\`
                                if idx == 2 && ancestor.as_bytes().get(1) == Some(&b':') {
                                    let drive_root = &ancestor[..3];
                                    if root_set.contains(drive_root) {
                                        owner = Some(drive_root.to_string());
                                    }
                                    break;
                                }
                                ancestor.truncate(idx);
                            }
                            None => break,
                        }
                    }

                    let Some(owner) = owner else { continue };

                    let fsize = fsize.max(0) as u64;
                    let agg = aggs.entry(owner).or_default();
                    agg.total_bytes += fsize;
                    agg.file_count += 1;
                    if let Some(m) = fmod {
                        agg.max_modified = Some(agg.max_modified.map_or(m, |curr| curr.max(m)));
                    }

                    if contains_segment_ci(&fpath, "node_modules")
                        || contains_segment_ci(&fpath, ".venv")
                        || contains_segment_ci(&fpath, "venv")
                        || contains_segment_ci(&fpath, "Pods")
                        || contains_segment_ci(&fpath, ".pnpm-store")
                    {
                        agg.dep_bytes += fsize;
                    } else if contains_subpath_ci(&fpath, ".next", "cache")
                        || contains_segment_ci(&fpath, ".vite")
                        || contains_segment_ci(&fpath, ".turbo")
                        || contains_segment_ci(&fpath, ".parcel-cache")
                        || contains_segment_ci(&fpath, "__pycache__")
                        || contains_segment_ci(&fpath, ".pytest_cache")
                        || contains_segment_ci(&fpath, ".mypy_cache")
                        || contains_segment_ci(&fpath, ".ruff_cache")
                        || contains_subpath_ci(&fpath, ".gradle", "caches")
                    {
                        agg.cache_bytes += fsize;
                    } else if contains_segment_ci(&fpath, "target")
                        || contains_segment_ci(&fpath, "dist")
                        || contains_segment_ci(&fpath, "build")
                        || contains_segment_ci(&fpath, ".next")
                        || contains_segment_ci(&fpath, ".nuxt")
                        || contains_segment_ci(&fpath, ".output")
                        || contains_segment_ci(&fpath, "cmake-build-debug")
                        || contains_segment_ci(&fpath, "cmake-build-release")
                        || contains_segment_ci(&fpath, "debug")
                        || contains_segment_ci(&fpath, "release")
                        || contains_segment_ci(&fpath, "coverage")
                        || contains_segment_ci(&fpath, "DerivedData")
                    {
                        agg.build_bytes += fsize;
                    }
                }
                Ok(aggs)
            })?
        };

        // 3. Build project records in memory before acquiring the write lock
        let mut projects = Vec::with_capacity(root_list.len());
        for (root_path, file_markers) in root_list {
            let ecosystem = Self::detect_ecosystem(&file_markers);
            let name = Self::extract_project_name(&root_path);
            let project_id = Uuid::new_v4().to_string();

            let agg = aggregates.get(&root_path).copied().unwrap_or_default();
            let confidence = if file_markers.len() > 1 { 0.98 } else { 0.90 };

            projects.push(DeveloperProjectItem {
                id: project_id,
                session_id: sid.clone(),
                name,
                root_path,
                ecosystem,
                dependency_bytes: agg.dep_bytes,
                cache_bytes: agg.cache_bytes,
                build_output_bytes: agg.build_bytes,
                total_bytes: agg.total_bytes,
                file_count: agg.file_count,
                last_modified_at: agg.max_modified,
                confidence,
            });
        }
        projects.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));

        // 4. Persist inside a quick atomic write transaction
        let ecosystems = {
            let mut conn = db_pool.lock_conn();
            let tx = conn.transaction()?;
            tx.execute(
                "DELETE FROM developer_projects WHERE session_id = ?1;",
                [&sid],
            )?;
            tx.execute(
                "DELETE FROM developer_ecosystem_summaries WHERE session_id = ?1;",
                [&sid],
            )?;

            {
                let mut insert_stmt = tx.prepare_cached(
                    "INSERT INTO developer_projects (
                        id, session_id, name, root_path, ecosystem,
                        dependency_bytes, cache_bytes, build_output_bytes, total_bytes,
                        file_count, last_modified_at, confidence
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12);",
                )?;

                for p in &projects {
                    insert_stmt.execute(params![
                        &p.id,
                        &sid,
                        &p.name,
                        &p.root_path,
                        &p.ecosystem,
                        p.dependency_bytes as i64,
                        p.cache_bytes as i64,
                        p.build_output_bytes as i64,
                        p.total_bytes as i64,
                        p.file_count as i64,
                        p.last_modified_at,
                        p.confidence,
                    ])?;
                }
            }

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
                [&sid],
            )?;

            let mut ecosystems = Vec::new();
            {
                let mut eco_stmt = tx.prepare(
                    "SELECT ecosystem, project_count, total_bytes, dependency_bytes, cache_bytes, build_bytes
                     FROM developer_ecosystem_summaries
                     WHERE session_id = ?1
                     ORDER BY total_bytes DESC;",
                )?;
                let eco_rows = eco_stmt.query_map([&sid], |r| {
                    Ok(DeveloperEcosystemSummary {
                        ecosystem: r.get(0)?,
                        project_count: r.get::<_, i64>(1)? as u64,
                        total_bytes: r.get::<_, i64>(2)? as u64,
                        dependency_bytes: r.get::<_, i64>(3)? as u64,
                        cache_bytes: r.get::<_, i64>(4)? as u64,
                        build_bytes: r.get::<_, i64>(5)? as u64,
                    })
                })?;
                for e in eco_rows {
                    ecosystems.push(e?);
                }
            }

            // Record completion marker so 0-project sessions don't endlessly recompute on view open
            tx.execute(
                "INSERT INTO app_settings (key, value, updated_at)
                 VALUES (?1, '1', CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = '1', updated_at = CURRENT_TIMESTAMP;",
                [format!("dev_analyzed:{}", sid)],
            )?;

            tx.commit()?;
            ecosystems
        };

        let total_dev_bytes: u64 = projects.iter().map(|p| p.total_bytes).sum();
        let total_projects = projects.len() as u64;

        info!(
            "Analyzed {} developer projects ({} total bytes) for session '{}'",
            total_projects, total_dev_bytes, sid
        );

        Ok((projects, ecosystems, total_dev_bytes, total_projects))
    }

    #[allow(clippy::type_complexity)]
    fn load_cached_profile(
        db_pool: &DbPool,
        session_id: &str,
    ) -> Result<
        Option<(
            String,
            u64,
            u64,
            Vec<DeveloperEcosystemSummary>,
            Vec<DeveloperProjectItem>,
        )>,
    > {
        db_pool.with_read_conn(|conn| {
            let marker_exists: bool = conn
                .query_row(
                    "SELECT count(*) FROM app_settings WHERE key = ?1",
                    [format!("dev_analyzed:{}", session_id)],
                    |r| r.get::<_, i64>(0),
                )
                .map(|c| c > 0)
                .unwrap_or(false);

            let project_count: i64 = conn
                .query_row(
                    "SELECT count(*) FROM developer_projects WHERE session_id = ?1",
                    [session_id],
                    |r| r.get(0),
                )
                .unwrap_or(0);

            if !marker_exists && project_count == 0 {
                return Ok(None);
            }

            let mut projects = Vec::new();
            {
                let mut stmt = conn.prepare(
                    "SELECT id, session_id, name, root_path, ecosystem,
                            dependency_bytes, cache_bytes, build_output_bytes, total_bytes,
                            file_count, last_modified_at, confidence
                     FROM developer_projects
                     WHERE session_id = ?1
                     ORDER BY total_bytes DESC;",
                )?;
                let rows = stmt.query_map([session_id], |r| {
                    Ok(DeveloperProjectItem {
                        id: r.get(0)?,
                        session_id: r.get(1)?,
                        name: r.get(2)?,
                        root_path: r.get(3)?,
                        ecosystem: r.get(4)?,
                        dependency_bytes: r.get::<_, i64>(5)? as u64,
                        cache_bytes: r.get::<_, i64>(6)? as u64,
                        build_output_bytes: r.get::<_, i64>(7)? as u64,
                        total_bytes: r.get::<_, i64>(8)? as u64,
                        file_count: r.get::<_, i64>(9)? as u64,
                        last_modified_at: r.get(10)?,
                        confidence: r.get(11)?,
                    })
                })?;
                for r in rows {
                    projects.push(r?);
                }
            }

            let mut ecosystems = Vec::new();
            {
                let mut eco_stmt = conn.prepare(
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
                for e in eco_rows {
                    ecosystems.push(e?);
                }
            }

            let total_dev_bytes: u64 = projects.iter().map(|p| p.total_bytes).sum();
            let total_projects = projects.len() as u64;

            Ok(Some((
                session_id.to_string(),
                total_dev_bytes,
                total_projects,
                ecosystems,
                projects,
            )))
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
        path_str
            .trim_end_matches(&['/', '\\'][..])
            .rsplit(&['/', '\\'][..])
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("Developer Project")
            .to_string()
    }

    /// Safely inspects Docker engine storage metrics via supported CLI command with a bounded timeout.
    pub fn inspect_docker_storage() -> DockerStorageInfo {
        let spawn_res = Command::new("docker")
            .args([
                "system",
                "df",
                "--format",
                "{{.Type}}\t{{.TotalCount}}\t{{.Size}}\t{{.Reclaimable}}",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        let mut child = match spawn_res {
            Ok(c) => c,
            Err(_) => {
                return DockerStorageInfo {
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
                };
            }
        };

        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        if let Some(mut stdout) = child.stdout.take() {
                            use std::io::Read;
                            let mut buf = String::new();
                            let _ = stdout.read_to_string(&mut buf);
                            return Self::parse_docker_df_output(&buf);
                        }
                    }
                    return DockerStorageInfo {
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
                    };
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return DockerStorageInfo {
                            is_available: false,
                            images_bytes: 0,
                            images_count: 0,
                            containers_bytes: 0,
                            containers_count: 0,
                            volumes_bytes: 0,
                            volumes_count: 0,
                            build_cache_bytes: 0,
                            total_bytes: 0,
                            status_message: "Docker CLI check timed out".to_string(),
                        };
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(_) => {
                    let _ = child.kill();
                    return DockerStorageInfo {
                        is_available: false,
                        images_bytes: 0,
                        images_count: 0,
                        containers_bytes: 0,
                        containers_count: 0,
                        volumes_bytes: 0,
                        volumes_count: 0,
                        build_cache_bytes: 0,
                        total_bytes: 0,
                        status_message: "Docker daemon query failed".to_string(),
                    };
                }
            }
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
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
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
                break;
            }
            if let Ok(entries) = std::fs::read_dir(&current) {
                for entry in entries.flatten() {
                    if let Ok(meta) = std::fs::symlink_metadata(entry.path()) {
                        if meta.file_type().is_symlink() {
                            continue;
                        }
                        if meta.is_file() {
                            total_size += meta.len();
                            file_count += 1;
                        } else if meta.is_dir() {
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
            let mut conn = pool.lock_conn();
            Migrator::run(&mut conn).unwrap();

            conn.execute(
                "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
                 VALUES ('TEST_DEV_DRIVE', 'Test Drive', '/work', 'apfs', 100000000, 50000000);",
                (),
            )
            .unwrap();

            conn.execute(
                "INSERT INTO scan_sessions (id, drive_id, root_path, status, total_files, total_directories, total_bytes)
                 VALUES ('session_dev_1', 'TEST_DEV_DRIVE', '/work', 'completed', 8, 5, 160000);",
                (),
            )
            .unwrap();

            // Include a vendored package.json inside node_modules/react (must NOT become a separate project!)
            // plus a plain Git repo whose .git is in directories.
            conn.execute(
                "INSERT INTO files (session_id, path, name, extension, parent_path, size_bytes, modified_at)
                 VALUES
                 ('session_dev_1', '/work/web-app/package.json', 'package.json', 'json', '/work/web-app', 1000, 500),
                 ('session_dev_1', '/work/web-app/node_modules/react/package.json', 'package.json', 'json', '/work/web-app/node_modules/react', 2000, 500),
                 ('session_dev_1', '/work/web-app/node_modules/react/index.js', 'index.js', 'js', '/work/web-app/node_modules/react', 48000, 500),
                 ('session_dev_1', '/work/web-app/.next/cache/webpack.pack', 'webpack.pack', 'pack', '/work/web-app/.next/cache', 30000, 500),
                 ('session_dev_1', '/work/web-app/dist/bundle.js', 'bundle.js', 'js', '/work/web-app/dist', 20000, 500),
                 ('session_dev_1', '/work/rust-cli/Cargo.toml', 'Cargo.toml', 'toml', '/work/rust-cli', 500, 600),
                 ('session_dev_1', '/work/rust-cli/target/debug/cli', 'cli', '', '/work/rust-cli/target/debug', 48500, 600),
                 ('session_dev_1', 'C:\\win-proj\\package.json', 'package.json', 'json', 'C:\\win-proj', 100, 700),
                 ('session_dev_1', 'C:\\win-proj\\node_modules\\pkg\\index.js', 'index.js', 'js', 'C:\\win-proj\\node_modules\\pkg', 9900, 700),
                 ('session_dev_1', '/work/git-only/README.md', 'README.md', 'md', '/work/git-only', 500, 800);",
                (),
            )
            .unwrap();

            conn.execute(
                "INSERT INTO directories (session_id, path, name, parent_path, file_count, total_size_bytes)
                 VALUES ('session_dev_1', '/work/git-only/.git', '.git', '/work/git-only', 0, 0);",
                (),
            )
            .unwrap();
        }

        let (projects, _, _, total_projects) =
            DeveloperService::analyze_project_storage_db(&pool, "session_dev_1").unwrap();

        assert_eq!(total_projects, 4);
        assert_eq!(projects.len(), 4);

        let web_app = projects.iter().find(|p| p.name == "web-app").unwrap();
        assert_eq!(web_app.ecosystem, "Node.js");
        assert_eq!(web_app.dependency_bytes, 50000);
        assert_eq!(web_app.cache_bytes, 30000);
        assert_eq!(web_app.build_output_bytes, 20000);
        assert_eq!(web_app.total_bytes, 101000);

        let rust_cli = projects.iter().find(|p| p.name == "rust-cli").unwrap();
        assert_eq!(rust_cli.ecosystem, "Rust");
        assert_eq!(rust_cli.build_output_bytes, 48500);
        assert_eq!(rust_cli.total_bytes, 49000);

        let win_proj = projects.iter().find(|p| p.name == "win-proj").unwrap();
        assert_eq!(win_proj.ecosystem, "Node.js");
        assert_eq!(win_proj.dependency_bytes, 9900);
        assert_eq!(win_proj.total_bytes, 10000);

        let git_only = projects.iter().find(|p| p.name == "git-only").unwrap();
        assert_eq!(git_only.ecosystem, "Git Repository");
        assert_eq!(git_only.total_bytes, 500);

        // Cached load works without recomputing
        let cached = DeveloperService::load_cached_profile(&pool, "session_dev_1")
            .unwrap()
            .unwrap();
        assert_eq!(cached.2, 4);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
