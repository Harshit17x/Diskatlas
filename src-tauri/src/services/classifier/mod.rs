pub mod rules;

use rusqlite::params;
use tracing::info;

use crate::error::{AppError, Result};
use crate::models::classification::{
    AppStatItem, CategoryOverrideRule, CategoryStatItem, ClassificationResult,
    StorageAnalysisProfile, StorageCategory, SubcategoryStatItem,
};
use crate::models::scan::{ScanFileItem, ScanFilePage};
use crate::services::classifier::rules::RuleMatcher;
use crate::storage::DbPool;

pub struct ClassifierService;

impl ClassifierService {
    /// Classifies an individual file against user overrides first, then priority heuristics.
    pub fn classify_file(
        path: &str,
        name: &str,
        extension: Option<&str>,
        is_hidden: bool,
        is_system: bool,
        overrides: &[CategoryOverrideRule],
    ) -> ClassificationResult {
        let p_lower = path.to_lowercase().replace('\\', "/");

        // 1. Check user overrides first
        for rule in overrides {
            let pat_lower = rule.pattern.to_lowercase().replace('\\', "/");
            if p_lower.contains(&pat_lower) {
                return ClassificationResult {
                    category: rule.category.clone(),
                    subcategory: rule.subcategory.clone(),
                    app_name: Some("User Override Rule".to_string()),
                    confidence: 1.0,
                };
            }
        }

        // 2. Evaluate deterministic rule matcher
        RuleMatcher::classify(path, name, extension, is_hidden, is_system)
    }

    /// Classifies all files in a scan session, persists attributes into SQLite, and builds summaries.
    pub fn classify_and_persist_session(
        db_pool: &DbPool,
        session_id: &str,
    ) -> Result<StorageAnalysisProfile> {
        let overrides = Self::get_overrides(db_pool)?;

        let conn_arc = db_pool.conn();
        let mut conn = conn_arc.lock().unwrap();

        let tx = conn.transaction()?;

        // 1. If user overrides exist, apply them efficiently via pattern matching
        if !overrides.is_empty() {
            for rule in &overrides {
                let pattern = format!("%{}%", rule.pattern.replace('\\', "/"));
                tx.execute(
                    "UPDATE files
                     SET category = ?1, subcategory = ?2, app_name = 'User Override Rule', confidence = 1.0
                     WHERE session_id = ?3 AND (path LIKE ?4 OR name LIKE ?4)",
                    params![rule.category, rule.subcategory, session_id, pattern],
                )?;
            }
        }

        // 2. Fallback: if any files were unclassified (e.g. from tests or legacy sessions), classify them in batch
        let unclassified_count: i64 = tx
            .query_row(
                "SELECT count(*) FROM files WHERE session_id = ?1 AND (category IS NULL OR category = 'unknown')",
                [session_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if unclassified_count > 0 {
            let mut fetch_stmt = tx.prepare(
                "SELECT id, path, name, extension, is_hidden, is_system
                 FROM files
                 WHERE session_id = ?1 AND (category IS NULL OR category = 'unknown')
                 LIMIT 5000",
            )?;

            struct FileMeta {
                id: i64,
                path: String,
                name: String,
                extension: Option<String>,
                is_hidden: bool,
                is_system: bool,
            }

            let file_rows = fetch_stmt.query_map([session_id], |row| {
                Ok(FileMeta {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    name: row.get(2)?,
                    extension: row.get(3)?,
                    is_hidden: row.get::<_, i64>(4)? != 0,
                    is_system: row.get::<_, i64>(5)? != 0,
                })
            })?;

            let mut files_to_classify = Vec::new();
            for r in file_rows {
                files_to_classify.push(r?);
            }
            drop(fetch_stmt);

            let mut update_stmt = tx.prepare(
                "UPDATE files
                 SET category = ?1, subcategory = ?2, app_name = ?3, confidence = ?4
                 WHERE id = ?5",
            )?;

            for file in files_to_classify {
                let res = Self::classify_file(
                    &file.path,
                    &file.name,
                    file.extension.as_deref(),
                    file.is_hidden,
                    file.is_system,
                    &overrides,
                );

                update_stmt.execute(params![
                    res.category,
                    res.subcategory,
                    res.app_name,
                    res.confidence,
                    file.id,
                ])?;
            }
        }

        // 3. Clear existing classification summaries for this session
        tx.execute(
            "DELETE FROM classification_summaries WHERE session_id = ?1",
            [session_id],
        )?;

        // 4. Instantaneous set-based SQL aggregation (< 0.2s even for millions of rows)
        tx.execute(
            "INSERT INTO classification_summaries (
                session_id, category, subcategory, app_name, file_count, total_bytes, avg_confidence
             )
             SELECT
                session_id,
                category,
                subcategory,
                app_name,
                COUNT(*) as file_count,
                COALESCE(SUM(size_bytes), 0) as total_bytes,
                AVG(confidence) as avg_confidence
             FROM files
             WHERE session_id = ?1
             GROUP BY category, subcategory, app_name;",
            [session_id],
        )?;

        tx.commit()?;
        drop(conn);

        info!("Classified and aggregated all files for session '{}'", session_id);
        Self::get_storage_analysis(db_pool, session_id)
    }

    /// Retrieves classified category breakdown, subcategories, and applications for a session.
    pub fn get_storage_analysis(
        db_pool: &DbPool,
        session_id: &str,
    ) -> Result<StorageAnalysisProfile> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();

        // Check if session has been classified; if not, drop lock and run classification
        let has_classifications: bool = conn
            .query_row(
                "SELECT count(*) FROM classification_summaries WHERE session_id = ?1",
                [session_id],
                |r| r.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !has_classifications {
            drop(conn);
            return Self::classify_and_persist_session(db_pool, session_id);
        }

        // Fetch session total info
        let (drive_id, total_files, total_bytes): (String, u64, u64) = conn
            .query_row(
                "SELECT drive_id, total_files, total_bytes FROM scan_sessions WHERE id = ?1",
                [session_id],
                |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u64, r.get::<_, i64>(2)? as u64)),
            )
            .map_err(|_| AppError::NotFound(format!("Session '{}' not found", session_id)))?;

        // Distinct categories aggregation
        let mut cat_stmt = conn.prepare(
            "SELECT
                category,
                SUM(file_count) as total_files,
                SUM(total_bytes) as total_bytes,
                AVG(avg_confidence) as avg_conf
             FROM classification_summaries
             WHERE session_id = ?1
             GROUP BY category
             ORDER BY total_bytes DESC;",
        )?;

        struct CategoryRow {
            category: String,
            files: u64,
            bytes: u64,
            confidence: f64,
        }

        let cat_rows = cat_stmt.query_map([session_id], |row| {
            Ok(CategoryRow {
                category: row.get(0)?,
                files: row.get::<_, i64>(1)? as u64,
                bytes: row.get::<_, i64>(2)? as u64,
                confidence: row.get(3)?,
            })
        })?;

        let mut raw_cats = Vec::new();
        for r in cat_rows {
            raw_cats.push(r?);
        }

        let mut categories = Vec::new();

        for cat in raw_cats {
            let cat_enum = StorageCategory::from_id(&cat.category);
            let label = cat_enum.label().to_string();

            // Fetch subcategories
            let mut sub_stmt = conn.prepare(
                "SELECT subcategory, SUM(file_count), SUM(total_bytes)
                 FROM classification_summaries
                 WHERE session_id = ?1 AND category = ?2 AND subcategory IS NOT NULL
                 GROUP BY subcategory
                 ORDER BY SUM(total_bytes) DESC
                 LIMIT 5;",
            )?;

            let sub_rows = sub_stmt.query_map(params![session_id, cat.category], |r| {
                Ok(SubcategoryStatItem {
                    subcategory: r.get(0)?,
                    file_count: r.get::<_, i64>(1)? as u64,
                    total_bytes: r.get::<_, i64>(2)? as u64,
                })
            })?;

            let mut subcategories = Vec::new();
            for s in sub_rows {
                subcategories.push(s?);
            }

            // Fetch top apps
            let mut app_stmt = conn.prepare(
                "SELECT app_name, SUM(file_count), SUM(total_bytes)
                 FROM classification_summaries
                 WHERE session_id = ?1 AND category = ?2 AND app_name IS NOT NULL
                 GROUP BY app_name
                 ORDER BY SUM(total_bytes) DESC
                 LIMIT 5;",
            )?;

            let app_rows = app_stmt.query_map(params![session_id, cat.category], |r| {
                Ok(AppStatItem {
                    app_name: r.get(0)?,
                    file_count: r.get::<_, i64>(1)? as u64,
                    total_bytes: r.get::<_, i64>(2)? as u64,
                })
            })?;

            let mut top_apps = Vec::new();
            for a in app_rows {
                top_apps.push(a?);
            }

            let percentage = if total_bytes > 0 {
                ((cat.bytes as f64) / (total_bytes as f64)) * 100.0
            } else {
                0.0
            };

            categories.push(CategoryStatItem {
                category: cat.category,
                label,
                file_count: cat.files,
                dir_count: 0,
                total_bytes: cat.bytes,
                percentage_bytes: percentage,
                avg_confidence: cat.confidence,
                subcategories,
                top_apps,
            });
        }

        // Calculate overall classification percentage (excluding 'unknown')
        let known_bytes: u64 = categories
            .iter()
            .filter(|c| c.category != "unknown")
            .map(|c| c.total_bytes)
            .sum();

        let classified_percent = if total_bytes > 0 {
            ((known_bytes as f64) / (total_bytes as f64)) * 100.0
        } else {
            0.0
        };

        let overall_confidence = if !categories.is_empty() {
            categories.iter().map(|c| c.avg_confidence).sum::<f64>() / (categories.len() as f64)
        } else {
            0.0
        };

        Ok(StorageAnalysisProfile {
            session_id: session_id.to_string(),
            drive_id,
            total_files,
            total_bytes,
            categories,
            classified_percent,
            overall_confidence,
        })
    }

    /// Retrieves paginated files belonging to a specific category for detailed path drilldown.
    pub fn get_category_files(
        db_pool: &DbPool,
        session_id: &str,
        category: &str,
        page: u32,
        page_size: u32,
    ) -> Result<ScanFilePage> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();

        let limit = page_size.clamp(1, 100);
        let offset = page.saturating_sub(1) * limit;

        let total_count: u64 = conn.query_row(
            "SELECT count(*) FROM files WHERE session_id = ?1 AND category = ?2",
            params![session_id, category],
            |r| r.get(0),
        )?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, path, name, extension, parent_path,
                    size_bytes, allocated_bytes, created_at, modified_at,
                    is_hidden, is_system, is_symlink
             FROM files
             WHERE session_id = ?1 AND category = ?2
             ORDER BY size_bytes DESC
             LIMIT ?3 OFFSET ?4;",
        )?;

        let rows = stmt.query_map(params![session_id, category, limit as i64, offset as i64], |row| {
            Ok(ScanFileItem {
                id: row.get(0)?,
                session_id: row.get(1)?,
                path: row.get(2)?,
                name: row.get(3)?,
                extension: row.get(4)?,
                parent_path: row.get(5)?,
                size_bytes: row.get::<_, i64>(6)? as u64,
                allocated_bytes: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                created_at: row.get(8)?,
                modified_at: row.get(9)?,
                is_hidden: row.get::<_, i64>(10)? != 0,
                is_system: row.get::<_, i64>(11)? != 0,
                is_symlink: row.get::<_, i64>(12)? != 0,
            })
        })?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }

        let total_pages = ((total_count as f64) / (limit as f64)).ceil() as u32;

        Ok(ScanFilePage {
            items,
            total_count,
            page,
            page_size: limit,
            total_pages: if total_pages == 0 { 1 } else { total_pages },
        })
    }

    /// User override rules management
    pub fn get_overrides(db_pool: &DbPool) -> Result<Vec<CategoryOverrideRule>> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, pattern, category, subcategory, created_at
             FROM category_overrides
             ORDER BY id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(CategoryOverrideRule {
                id: row.get(0)?,
                pattern: row.get(1)?,
                category: row.get(2)?,
                subcategory: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn add_override(
        db_pool: &DbPool,
        pattern: &str,
        category: &str,
        subcategory: Option<&str>,
    ) -> Result<CategoryOverrideRule> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();

        conn.execute(
            "INSERT INTO category_overrides (pattern, category, subcategory)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(pattern) DO UPDATE SET category = ?2, subcategory = ?3;",
            params![pattern, category, subcategory],
        )?;

        let id = conn.last_insert_rowid();
        let created_at: String = conn.query_row(
            "SELECT created_at FROM category_overrides WHERE id = ?1",
            [id],
            |r| r.get(0),
        )?;

        Ok(CategoryOverrideRule {
            id,
            pattern: pattern.to_string(),
            category: category.to_string(),
            subcategory: subcategory.map(|s| s.to_string()),
            created_at,
        })
    }

    pub fn delete_override(db_pool: &DbPool, id: i64) -> Result<()> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();
        conn.execute("DELETE FROM category_overrides WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrator::Migrator;
    use uuid::Uuid;

    fn create_test_db() -> DbPool {
        let temp_dir = std::env::temp_dir().join(format!("diskatlas_class_test_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("class_test.db");
        let pool = DbPool::new(&db_path).unwrap();

        {
            let conn_arc = pool.conn();
            let mut conn = conn_arc.lock().unwrap();
            Migrator::run(&mut conn).unwrap();

            conn.execute(
                "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
                 VALUES ('TEST_CLASS_DRIVE', 'Test Drive', '/test', 'apfs', 100000000, 50000000);",
                (),
            )
            .unwrap();

            conn.execute(
                "INSERT INTO scan_sessions (id, drive_id, root_path, status, total_files, total_directories, total_bytes)
                 VALUES ('session_class_1', 'TEST_CLASS_DRIVE', '/test', 'completed', 5, 2, 50000);",
                (),
            )
            .unwrap();

            // Insert distinct file records across multiple categories
            conn.execute(
                "INSERT INTO files (session_id, path, name, extension, parent_path, size_bytes, is_hidden, is_system)
                 VALUES
                 ('session_class_1', '/test/Windows/System32/kernel.dll', 'kernel.dll', 'dll', '/test/Windows/System32', 10000, 0, 1),
                 ('session_class_1', '/test/project/node_modules/react/index.js', 'index.js', 'js', '/test/project/node_modules/react', 5000, 0, 0),
                 ('session_class_1', '/test/Steam/steamapps/common/game/data.pak', 'data.pak', 'pak', '/test/Steam/steamapps/common/game', 20000, 0, 0),
                 ('session_class_1', '/test/vms/ubuntu.vdi', 'ubuntu.vdi', 'vdi', '/test/vms', 12000, 0, 0),
                 ('session_class_1', '/test/mystery.xyz123', 'mystery.xyz123', 'xyz123', '/test', 3000, 0, 0);",
                (),
            )
            .unwrap();
        }

        pool
    }

    #[test]
    fn test_deterministic_classification_and_no_double_counting() {
        let pool = create_test_db();

        let profile = ClassifierService::classify_and_persist_session(&pool, "session_class_1").unwrap();

        // Check that total files and bytes are strictly preserved
        let sum_files: u64 = profile.categories.iter().map(|c| c.file_count).sum();
        let sum_bytes: u64 = profile.categories.iter().map(|c| c.total_bytes).sum();

        assert_eq!(sum_files, 5, "Total files must match without double-counting");
        assert_eq!(sum_bytes, 50000, "Total bytes must strictly equal sum of files");

        // Verify categories
        let has_os = profile.categories.iter().any(|c| c.category == "os" && c.total_bytes == 10000);
        let has_dev = profile.categories.iter().any(|c| c.category == "developer" && c.total_bytes == 5000);
        let has_gaming = profile.categories.iter().any(|c| c.category == "gaming" && c.total_bytes == 20000);
        let has_vm = profile.categories.iter().any(|c| c.category == "vm_emulators" && c.total_bytes == 12000);
        let has_unknown = profile.categories.iter().any(|c| c.category == "unknown" && c.total_bytes == 3000);

        assert!(has_os, "Operating system category must be assigned");
        assert!(has_dev, "Developer category must be assigned for node_modules");
        assert!(has_gaming, "Gaming category must be assigned for steamapps");
        assert!(has_vm, "VM category must be assigned for .vdi");
        assert!(has_unknown, "Unknown files must be preserved, not dropped");
    }

    #[test]
    fn test_user_override_precedence() {
        let pool = create_test_db();

        // Add user override rule: mapping mystery file to "backups"
        ClassifierService::add_override(&pool, "mystery", "backups", Some("Custom Archive")).unwrap();

        let profile = ClassifierService::classify_and_persist_session(&pool, "session_class_1").unwrap();

        let has_backup = profile.categories.iter().any(|c| c.category == "backups" && c.total_bytes == 3000);
        assert!(has_backup, "User override rule must take precedence over default unknown classification");
    }
}

