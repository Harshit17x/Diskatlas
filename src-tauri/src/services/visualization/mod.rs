use rusqlite::{params, Connection};
use tracing::info;

use crate::error::Result;
use crate::models::developer::DeveloperEcosystemSummary;
use crate::models::history::StorageSnapshotRecord;
use crate::models::visualization::*;

pub struct VisualizationService;

impl VisualizationService {
    /// Recursively builds a bounded, database-aggregated hierarchical tree for Treemap & Sunburst charts
    pub fn get_hierarchical_tree(
        conn: &Connection,
        query: HierarchicalQuery,
    ) -> Result<HierarchicalNode> {
        let sid = Self::resolve_session_id(conn, query.session_id.as_deref())?;

        // 1. Resolve starting root path
        let root_path = match query.root_path {
            Some(ref p) if !p.is_empty() => p.clone(),
            _ => {
                let default_root: String = conn.query_row(
                    "SELECT root_path FROM scan_sessions WHERE id = ?1",
                    [&sid],
                    |row| row.get(0),
                )?;
                default_root
            }
        };

        // 2. Query total root metrics
        let (root_size, root_file_count): (i64, i64) = conn
            .query_row(
                "SELECT COALESCE(total_size_bytes, 0), COALESCE(file_count, 0)
                 FROM directories
                 WHERE session_id = ?1 AND path = ?2",
                params![&sid, &root_path],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or_else(|_| {
                // If directory record not found (e.g. root drive), query from scan_sessions
                let session_bytes: i64 = conn
                    .query_row(
                        "SELECT COALESCE(total_bytes, 0), COALESCE(total_files, 0) FROM scan_sessions WHERE id = ?1",
                        [&sid],
                        |row| Ok(row.get(0)?),
                    )
                    .unwrap_or(0);
                (session_bytes, 0)
            });

        let max_depth = query.max_depth.unwrap_or(3).clamp(1, 5);
        let threshold = query
            .min_bytes_threshold
            .unwrap_or_else(|| (root_size as f64 * 0.002) as i64) // default 0.2% of total size
            .max(1024); // at least 1 KB

        info!(
            session_id = %sid,
            root_path = %root_path,
            max_depth = max_depth,
            threshold_bytes = threshold,
            "Building aggregated hierarchical visualization tree"
        );

        let root_name = std::path::Path::new(&root_path)
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(&root_path)
            .to_string();

        let children = Self::build_children(conn, &sid, &root_path, 1, max_depth, threshold)?;

        Ok(HierarchicalNode {
            name: root_name,
            path: root_path,
            value: root_size,
            category: Some("drive_root".to_string()),
            is_directory: true,
            file_count: Some(root_file_count),
            children: if children.is_empty() { None } else { Some(children) },
        })
    }

    /// Recursively queries direct subdirectories and significant direct files up to max_depth
    fn build_children(
        conn: &Connection,
        session_id: &str,
        parent_path: &str,
        current_depth: u32,
        max_depth: u32,
        threshold: i64,
    ) -> Result<Vec<HierarchicalNode>> {
        if current_depth > max_depth {
            return Ok(Vec::new());
        }

        let mut child_nodes: Vec<HierarchicalNode> = Vec::new();
        let mut other_bytes = 0i64;
        let mut other_count = 0i64;

        // 1. Query direct subdirectories
        let mut dir_stmt = conn.prepare(
            "SELECT path, name, total_size_bytes, file_count
             FROM directories
             WHERE session_id = ?1 AND parent_path = ?2
             ORDER BY total_size_bytes DESC
             LIMIT 35"
        )?;

        let dir_rows = dir_stmt.query_map(params![session_id, parent_path], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;

        for r in dir_rows {
            let (path, name, size_bytes, file_count) = r?;
            if size_bytes < threshold {
                other_bytes += size_bytes;
                other_count += file_count;
                continue;
            }

            let sub_children = if current_depth < max_depth {
                let sub = Self::build_children(conn, session_id, &path, current_depth + 1, max_depth, threshold)?;
                if sub.is_empty() { None } else { Some(sub) }
            } else {
                None
            };

            let category = Self::infer_dir_category(&name, &path);

            child_nodes.push(HierarchicalNode {
                name,
                path,
                value: size_bytes,
                category: Some(category),
                is_directory: true,
                file_count: Some(file_count),
                children: sub_children,
            });
        }

        // 2. Query significant direct files in this directory
        let mut file_stmt = conn.prepare(
            "SELECT path, name, size_bytes, category
             FROM files
             WHERE session_id = ?1 AND parent_path = ?2 AND size_bytes >= ?3
             ORDER BY size_bytes DESC
             LIMIT 25"
        )?;

        let file_rows = file_stmt.query_map(params![session_id, parent_path, threshold], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;

        for r in file_rows {
            let (path, name, size_bytes, cat) = r?;
            child_nodes.push(HierarchicalNode {
                name,
                path,
                value: size_bytes,
                category: cat.or_else(|| Some("documents".to_string())),
                is_directory: false,
                file_count: Some(1),
                children: None,
            });
        }

        // 3. If there were smaller items below threshold, aggregate them into a synthetic node
        if other_bytes > 0 {
            child_nodes.push(HierarchicalNode {
                name: "Other items".to_string(),
                path: format!("{}/[other]", parent_path.trim_end_matches('/')),
                value: other_bytes,
                category: Some("unknown".to_string()),
                is_directory: false,
                file_count: Some(other_count),
                children: None,
            });
        }

        // Sort largest items first for optimal Treemap squarification
        child_nodes.sort_by(|a, b| b.value.cmp(&a.value));

        Ok(child_nodes)
    }

    /// Fetches complete aggregated analytics distributions from database
    pub fn get_storage_analytics(
        conn: &Connection,
        session_id: Option<&str>,
    ) -> Result<StorageAnalyticsData> {
        let sid = match Self::resolve_session_id(conn, session_id) {
            Ok(s) => s,
            Err(_) => {
                // If no session exists, return an empty analytics model immediately without failing or blocking
                return Ok(StorageAnalyticsData {
                    session_id: String::new(),
                    total_bytes: 0,
                    total_files: 0,
                    category_distribution: Vec::new(),
                    extension_distribution: Vec::new(),
                    largest_directories: Vec::new(),
                    application_storage: Vec::new(),
                    developer_storage: Vec::new(),
                    history_trends: Vec::new(),
                });
            }
        };

        let (total_bytes, total_files): (i64, i64) = conn.query_row(
            "SELECT COALESCE(total_bytes, 0), COALESCE(total_files, 0)
             FROM scan_sessions
             WHERE id = ?1",
            [&sid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // 1. Category Distribution
        let mut cat_stmt = conn.prepare(
            "SELECT category, file_count, total_bytes
             FROM classification_summaries
             WHERE session_id = ?1 AND subcategory IS NULL
             ORDER BY total_bytes DESC"
        )?;

        let mut category_distribution: Vec<CategoryChartItem> = cat_stmt
            .query_map([&sid], |row| {
                let category: String = row.get(0)?;
                let file_count: i64 = row.get(1)?;
                let bytes: i64 = row.get(2)?;
                let percentage = if total_bytes > 0 {
                    (bytes as f64 / total_bytes as f64) * 100.0
                } else {
                    0.0
                };
                Ok(CategoryChartItem {
                    category,
                    file_count,
                    total_bytes: bytes,
                    percentage,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Fallback: If classification_summaries is empty, aggregate directly from files table
        if category_distribution.is_empty() {
            let mut fallback_stmt = conn.prepare(
                "SELECT COALESCE(category, 'unknown') as cat, COUNT(*), SUM(size_bytes)
                 FROM files
                 WHERE session_id = ?1
                 GROUP BY cat
                 ORDER BY SUM(size_bytes) DESC"
            )?;

            category_distribution = fallback_stmt
                .query_map([&sid], |row| {
                    let category: String = row.get(0)?;
                    let file_count: i64 = row.get(1)?;
                    let bytes: i64 = row.get(2)?;
                    let percentage = if total_bytes > 0 {
                        (bytes as f64 / total_bytes as f64) * 100.0
                    } else {
                        0.0
                    };
                    Ok(CategoryChartItem {
                        category,
                        file_count,
                        total_bytes: bytes,
                        percentage,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();
        }

        // 2. Extension / File Type Distribution
        let mut ext_stmt = conn.prepare(
            "SELECT LOWER(COALESCE(NULLIF(extension, ''), 'unspecified')) as ext,
                    COUNT(*) as cnt,
                    COALESCE(SUM(size_bytes), 0) as bytes
             FROM files
             WHERE session_id = ?1
             GROUP BY ext
             ORDER BY bytes DESC
             LIMIT 15"
        )?;

        let extension_distribution: Vec<ExtensionDistItem> = ext_stmt
            .query_map([&sid], |row| {
                let extension: String = row.get(0)?;
                let file_count: i64 = row.get(1)?;
                let bytes: i64 = row.get(2)?;
                let percentage = if total_bytes > 0 {
                    (bytes as f64 / total_bytes as f64) * 100.0
                } else {
                    0.0
                };
                Ok(ExtensionDistItem {
                    extension,
                    file_count,
                    total_bytes: bytes,
                    percentage,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // 3. Top Largest Directories
        let mut dir_stmt = conn.prepare(
            "SELECT path, name, total_size_bytes, file_count, dir_count
             FROM directories
             WHERE session_id = ?1
             ORDER BY total_size_bytes DESC
             LIMIT 15"
        )?;

        let largest_directories: Vec<LargestDirItem> = dir_stmt
            .query_map([&sid], |row| {
                let path: String = row.get(0)?;
                let name: String = row.get(1)?;
                let bytes: i64 = row.get(2)?;
                let file_count: i64 = row.get(3)?;
                let dir_count: i64 = row.get(4)?;
                let percentage = if total_bytes > 0 {
                    (bytes as f64 / total_bytes as f64) * 100.0
                } else {
                    0.0
                };
                Ok(LargestDirItem {
                    path,
                    name,
                    total_bytes: bytes,
                    file_count,
                    dir_count,
                    percentage,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // 4. Application Storage Breakdown
        let mut app_stmt = conn.prepare(
            "SELECT app_name, COUNT(*) as cnt, COALESCE(SUM(size_bytes), 0) as bytes
             FROM files
             WHERE session_id = ?1 AND app_name IS NOT NULL AND app_name != ''
             GROUP BY app_name
             ORDER BY bytes DESC
             LIMIT 15"
        )?;

        let application_storage: Vec<AppStorageItem> = app_stmt
            .query_map([&sid], |row| {
                let app_name: String = row.get(0)?;
                let file_count: i64 = row.get(1)?;
                let bytes: i64 = row.get(2)?;
                let percentage = if total_bytes > 0 {
                    (bytes as f64 / total_bytes as f64) * 100.0
                } else {
                    0.0
                };
                Ok(AppStorageItem {
                    app_name,
                    file_count,
                    total_bytes: bytes,
                    percentage,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // 5. Developer Storage Summaries
        let mut dev_stmt = conn.prepare(
            "SELECT ecosystem, project_count, total_bytes, dependency_bytes, cache_bytes, build_bytes
             FROM developer_ecosystem_summaries
             WHERE session_id = ?1
             ORDER BY total_bytes DESC"
        )?;

        let developer_storage: Vec<DeveloperEcosystemSummary> = dev_stmt
            .query_map([&sid], |row| {
                Ok(DeveloperEcosystemSummary {
                    ecosystem: row.get(0)?,
                    project_count: row.get(1)?,
                    total_bytes: row.get(2)?,
                    dependency_bytes: row.get(3)?,
                    cache_bytes: row.get(4)?,
                    build_bytes: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // 6. Storage Growth History Snapshots
        let mut hist_stmt = conn.prepare(
            "SELECT id, session_id, drive_id, label, root_path, total_files, total_directories, total_bytes, created_at
             FROM storage_snapshots
             ORDER BY created_at ASC
             LIMIT 20"
        )?;

        let history_trends: Vec<StorageSnapshotRecord> = hist_stmt
            .query_map([], |row| {
                Ok(StorageSnapshotRecord {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    drive_id: row.get(2)?,
                    label: row.get(3)?,
                    root_path: row.get(4)?,
                    total_files: row.get(5)?,
                    total_directories: row.get(6)?,
                    total_bytes: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(StorageAnalyticsData {
            session_id: sid,
            total_bytes,
            total_files,
            category_distribution,
            extension_distribution,
            largest_directories,
            application_storage,
            developer_storage,
            history_trends,
        })
    }

    /// Resolves target session id, falling back to latest session
    fn resolve_session_id(conn: &Connection, requested: Option<&str>) -> Result<String> {
        if let Some(id) = requested {
            if !id.trim().is_empty() {
                return Ok(id.to_string());
            }
        }

        let latest: Option<String> = conn
            .query_row(
                "SELECT id FROM scan_sessions WHERE status IN ('completed', 'interrupted') ORDER BY started_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        match latest {
            Some(id) => Ok(id),
            None => Err(crate::error::AppError::NotFound(
                "No completed scan sessions found for visualization".to_string(),
            )),
        }
    }

    /// Helper to infer high-level category of directory from name and path
    fn infer_dir_category(name: &str, path: &str) -> String {
        let n_lower = name.to_lowercase();
        let p_lower = path.to_lowercase();

        if n_lower == "node_modules" || n_lower == "target" || n_lower == ".git" || n_lower == ".venv" {
            "developer".to_string()
        } else if p_lower.contains("windows") || p_lower.contains("system32") || p_lower.starts_with("/system") {
            "os".to_string()
        } else if p_lower.contains("program files") || p_lower.contains("applications") {
            "applications".to_string()
        } else if p_lower.contains("downloads") {
            "archives".to_string()
        } else if p_lower.contains("documents") {
            "documents".to_string()
        } else if p_lower.contains("pictures") || p_lower.contains("photos") {
            "images".to_string()
        } else if p_lower.contains("videos") || p_lower.contains("movies") {
            "videos".to_string()
        } else if p_lower.contains("music") {
            "audio".to_string()
        } else {
            "unknown".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrator::Migrator;

    fn setup_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrator::run(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
             VALUES ('DRIVE_VIS_TEST', 'Test Drive', 'C:\\', 'NTFS', 500000000000, 250000000000)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO scan_sessions (id, drive_id, root_path, started_at, status, total_bytes, total_files)
             VALUES ('sess-vis-1', 'DRIVE_VIS_TEST', 'C:\\', CURRENT_TIMESTAMP, 'completed', 100000000, 10)",
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_hierarchical_tree_aggregation_and_depth_bounding() {
        let conn = setup_test_db();

        // Insert directories hierarchy:
        // C:\ -> C:\Windows (40MB) -> C:\Windows\System32 (30MB)
        // C:\ -> C:\Users (50MB) -> C:\Users\Dev (45MB) -> C:\Users\Dev\project (40MB)
        conn.execute(
            "INSERT INTO directories (session_id, path, name, parent_path, total_size_bytes, file_count)
             VALUES
             ('sess-vis-1', 'C:\\', 'C:\\', NULL, 100000000, 10),
             ('sess-vis-1', 'C:\\Windows', 'Windows', 'C:\\', 40000000, 4),
             ('sess-vis-1', 'C:\\Windows\\System32', 'System32', 'C:\\Windows', 30000000, 3),
             ('sess-vis-1', 'C:\\Users', 'Users', 'C:\\', 50000000, 6),
             ('sess-vis-1', 'C:\\Users\\Dev', 'Dev', 'C:\\Users', 45000000, 5),
             ('sess-vis-1', 'C:\\Users\\Dev\\project', 'project', 'C:\\Users\\Dev', 40000000, 4)",
            [],
        )
        .unwrap();

        let query = HierarchicalQuery {
            session_id: Some("sess-vis-1".to_string()),
            root_path: Some("C:\\".to_string()),
            max_depth: Some(2),
            min_bytes_threshold: Some(1000000), // 1 MB
        };

        let tree = VisualizationService::get_hierarchical_tree(&conn, query).unwrap();
        assert_eq!(tree.name, "C:\\");
        assert!(tree.children.is_some());
        let children = tree.children.unwrap();
        assert_eq!(children.len(), 2, "Root C:\\ should have 2 direct children: Users and Windows");
        assert_eq!(children[0].name, "Users");
        assert_eq!(children[1].name, "Windows");

        // Verify Users has child Dev
        assert!(children[0].children.is_some());
        let dev_child = &children[0].children.as_ref().unwrap()[0];
        assert_eq!(dev_child.name, "Dev");
        // Because max_depth is 2, Dev should not recurse further
        assert!(dev_child.children.is_none());
    }

    #[test]
    fn test_storage_analytics_aggregation() {
        let conn = setup_test_db();

        conn.execute(
            "INSERT INTO files (session_id, path, name, extension, parent_path, size_bytes, category, app_name)
             VALUES
             ('sess-vis-1', 'C:\\app\\test.mp4', 'test.mp4', 'mp4', 'C:\\app', 50000000, 'videos', NULL),
             ('sess-vis-1', 'C:\\app\\code.rs', 'code.rs', 'rs', 'C:\\app', 10000000, 'developer', 'VS Code'),
             ('sess-vis-1', 'C:\\app\\doc.pdf', 'doc.pdf', 'pdf', 'C:\\app', 5000000, 'documents', NULL)",
            [],
        )
        .unwrap();

        let analytics = VisualizationService::get_storage_analytics(&conn, Some("sess-vis-1")).unwrap();
        assert_eq!(analytics.total_bytes, 100000000);
        assert!(!analytics.extension_distribution.is_empty());
        assert_eq!(analytics.extension_distribution[0].extension, "mp4");
        assert_eq!(analytics.extension_distribution[0].total_bytes, 50000000);

        assert!(!analytics.application_storage.is_empty());
        assert_eq!(analytics.application_storage[0].app_name, "VS Code");
    }
}
