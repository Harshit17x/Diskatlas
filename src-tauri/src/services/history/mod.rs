use rusqlite::params;
use tracing::info;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::history::{
    CategoryDeltaItem, CategorySummaryItem, DirectoryDeltaItem, DirectoryRecord,
    FileModifiedItem, ScanComparisonResult, ScanErrorItem, ScanSessionPage,
    ScanSessionRecord, SessionDetails, StorageSnapshotRecord,
};
use crate::models::scan::{ScanFileItem, ScanStatus};
use crate::storage::DbPool;

pub struct HistoryService;

impl HistoryService {
    /// Retrieves paginated scan sessions with optional filtering by drive and status.
    pub fn get_scan_sessions(
        db_pool: &DbPool,
        page: u32,
        page_size: u32,
        drive_id: Option<String>,
        status: Option<String>,
    ) -> Result<ScanSessionPage> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();

        let mut count_query = "SELECT COUNT(*) FROM scan_sessions s WHERE 1=1".to_string();
        let mut data_query = "SELECT
            s.id, s.drive_id, d.label, s.root_path, s.status,
            s.total_files, s.total_directories, s.total_bytes, s.error_count,
            s.started_at, s.completed_at
            FROM scan_sessions s
            LEFT JOIN drives d ON s.drive_id = d.id
            WHERE 1=1".to_string();

        let mut filter_params: Vec<String> = Vec::new();

        if let Some(did) = drive_id {
            if !did.trim().is_empty() {
                count_query.push_str(" AND s.drive_id = ?");
                data_query.push_str(" AND s.drive_id = ?");
                filter_params.push(did);
            }
        }

        if let Some(st) = status {
            if !st.trim().is_empty() {
                count_query.push_str(" AND s.status = ?");
                data_query.push_str(" AND s.status = ?");
                filter_params.push(st);
            }
        }

        // Count total matching records
        let mut count_stmt = conn.prepare(&count_query)?;
        let count_params = rusqlite::params_from_iter(filter_params.iter());
        let total_count: u64 = count_stmt.query_row(count_params, |r| r.get(0))?;

        // Order and paginate
        data_query.push_str(" ORDER BY s.started_at DESC LIMIT ? OFFSET ?;");
        let limit = page_size.clamp(1, 100);
        let offset = page.saturating_sub(1) * limit;

        let mut data_stmt = conn.prepare(&data_query)?;
        let mut query_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for p in &filter_params {
            query_params.push(Box::new(p.clone()));
        }
        query_params.push(Box::new(limit as i64));
        query_params.push(Box::new(offset as i64));

        let rows = data_stmt.query_map(
            rusqlite::params_from_iter(query_params.iter().map(|b| b.as_ref())),
            |row| {
                let status_str: String = row.get(4)?;
                let status = match status_str.as_str() {
                    "running" => ScanStatus::Running,
                    "paused" => ScanStatus::Paused,
                    "completed" => ScanStatus::Completed,
                    "completed_with_errors" => ScanStatus::CompletedWithErrors,
                    "cancelled" => ScanStatus::Cancelled,
                    "failed" => ScanStatus::Failed,
                    "interrupted" => ScanStatus::Interrupted,
                    _ => ScanStatus::Queued,
                };

                let started_at: String = row.get(9)?;
                let completed_at: Option<String> = row.get(10)?;

                Ok(ScanSessionRecord {
                    id: row.get(0)?,
                    drive_id: row.get(1)?,
                    drive_label: row.get(2)?,
                    root_path: row.get(3)?,
                    status,
                    total_files: row.get::<_, i64>(5)? as u64,
                    total_directories: row.get::<_, i64>(6)? as u64,
                    total_bytes: row.get::<_, i64>(7)? as u64,
                    error_count: row.get::<_, i64>(8)? as u64,
                    started_at,
                    completed_at,
                    duration_millis: None,
                })
            },
        )?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }

        let total_pages = ((total_count as f64) / (limit as f64)).ceil() as u32;

        Ok(ScanSessionPage {
            items,
            total_count,
            page,
            page_size: limit,
            total_pages: if total_pages == 0 { 1 } else { total_pages },
        })
    }

    /// Retrieves full breakdown for a specific scan session including category distribution and top directories.
    pub fn get_session_details(db_pool: &DbPool, session_id: &str) -> Result<SessionDetails> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();

        // 1. Session record
        let session = conn
            .query_row(
                "SELECT s.id, s.drive_id, d.label, s.root_path, s.status,
                        s.total_files, s.total_directories, s.total_bytes, s.error_count,
                        s.started_at, s.completed_at
                 FROM scan_sessions s
                 LEFT JOIN drives d ON s.drive_id = d.id
                 WHERE s.id = ?1",
                [session_id],
                |row| {
                    let status_str: String = row.get(4)?;
                    let status = match status_str.as_str() {
                        "running" => ScanStatus::Running,
                        "paused" => ScanStatus::Paused,
                        "completed" => ScanStatus::Completed,
                        "completed_with_errors" => ScanStatus::CompletedWithErrors,
                        "cancelled" => ScanStatus::Cancelled,
                        "failed" => ScanStatus::Failed,
                        "interrupted" => ScanStatus::Interrupted,
                        _ => ScanStatus::Queued,
                    };
                    Ok(ScanSessionRecord {
                        id: row.get(0)?,
                        drive_id: row.get(1)?,
                        drive_label: row.get(2)?,
                        root_path: row.get(3)?,
                        status,
                        total_files: row.get::<_, i64>(5)? as u64,
                        total_directories: row.get::<_, i64>(6)? as u64,
                        total_bytes: row.get::<_, i64>(7)? as u64,
                        error_count: row.get::<_, i64>(8)? as u64,
                        started_at: row.get(9)?,
                        completed_at: row.get(10)?,
                        duration_millis: None,
                    })
                },
            )
            .map_err(|_| AppError::NotFound(format!("Scan session '{}' not found", session_id)))?;

        // 2. Category summaries
        let mut cat_stmt = conn.prepare(
            "SELECT category, file_count, total_bytes
             FROM category_summaries
             WHERE session_id = ?1
             ORDER BY total_bytes DESC",
        )?;

        let cat_rows = cat_stmt.query_map([session_id], |row| {
            let cat: String = row.get(0)?;
            let file_count: u64 = row.get::<_, i64>(1)? as u64;
            let total_bytes: u64 = row.get::<_, i64>(2)? as u64;
            let percentage = if session.total_bytes > 0 {
                ((total_bytes as f64) / (session.total_bytes as f64)) * 100.0
            } else {
                0.0
            };
            Ok(CategorySummaryItem {
                category: cat,
                file_count,
                total_bytes,
                percentage_bytes: percentage,
            })
        })?;

        let mut categories = Vec::new();
        for c in cat_rows {
            categories.push(c?);
        }

        // 3. Top 15 largest directories
        let mut dir_stmt = conn.prepare(
            "SELECT path, name, total_size_bytes, file_count, dir_count
             FROM directories
             WHERE session_id = ?1
             ORDER BY total_size_bytes DESC
             LIMIT 15",
        )?;

        let dir_rows = dir_stmt.query_map([session_id], |row| {
            Ok(DirectoryRecord {
                path: row.get(0)?,
                name: row.get(1)?,
                total_size_bytes: row.get::<_, i64>(2)? as u64,
                file_count: row.get::<_, i64>(3)? as u64,
                dir_count: row.get::<_, i64>(4)? as u64,
            })
        })?;

        let mut top_directories = Vec::new();
        for d in dir_rows {
            top_directories.push(d?);
        }

        // 4. Sample scan errors (up to 20)
        let mut err_stmt = conn.prepare(
            "SELECT path, error_type, message, occurred_at
             FROM scan_errors
             WHERE session_id = ?1
             ORDER BY id ASC
             LIMIT 20",
        )?;

        let err_rows = err_stmt.query_map([session_id], |row| {
            Ok(ScanErrorItem {
                path: row.get(0)?,
                error_type: row.get(1)?,
                message: row.get(2)?,
                occurred_at: row.get(3)?,
            })
        })?;

        let mut error_samples = Vec::new();
        for e in err_rows {
            error_samples.push(e?);
        }

        Ok(SessionDetails {
            session,
            categories,
            top_directories,
            error_samples,
        })
    }

    /// Deletes a scan session and cascades deletion of files, directories, errors, and categories.
    pub fn delete_scan_session(db_pool: &DbPool, session_id: &str) -> Result<()> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();
        let deleted = conn.execute("DELETE FROM scan_sessions WHERE id = ?1", [session_id])?;
        if deleted == 0 {
            return Err(AppError::NotFound(format!("Session '{}' not found", session_id)));
        }
        info!("Deleted scan session '{}' and all associated records", session_id);
        Ok(())
    }

    /// Retrieves storage snapshot history for a drive.
    pub fn get_storage_snapshots(
        db_pool: &DbPool,
        drive_id: Option<String>,
        limit: u32,
    ) -> Result<Vec<StorageSnapshotRecord>> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();

        let l = limit.clamp(1, 100);

        let mut snapshots = Vec::new();
        if let Some(ref did) = drive_id {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, drive_id, label, root_path, total_files, total_directories, total_bytes, created_at
                 FROM storage_snapshots
                 WHERE drive_id = ?1
                 ORDER BY created_at DESC
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![did, l as i64], |row| {
                Ok(StorageSnapshotRecord {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    drive_id: row.get(2)?,
                    label: row.get(3)?,
                    root_path: row.get(4)?,
                    total_files: row.get::<_, i64>(5)? as u64,
                    total_directories: row.get::<_, i64>(6)? as u64,
                    total_bytes: row.get::<_, i64>(7)? as u64,
                    created_at: row.get(8)?,
                })
            })?;
            for r in rows {
                snapshots.push(r?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, drive_id, label, root_path, total_files, total_directories, total_bytes, created_at
                 FROM storage_snapshots
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![l as i64], |row| {
                Ok(StorageSnapshotRecord {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    drive_id: row.get(2)?,
                    label: row.get(3)?,
                    root_path: row.get(4)?,
                    total_files: row.get::<_, i64>(5)? as u64,
                    total_directories: row.get::<_, i64>(6)? as u64,
                    total_bytes: row.get::<_, i64>(7)? as u64,
                    created_at: row.get(8)?,
                })
            })?;
            for r in rows {
                snapshots.push(r?);
            }
        }
        Ok(snapshots)
    }

    /// Records a point-in-time snapshot for a drive.
    pub fn create_manual_snapshot(
        db_pool: &DbPool,
        drive_id: &str,
        label: &str,
    ) -> Result<StorageSnapshotRecord> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();

        // Lookup latest completed session for this drive or fallback to drive metrics
        let latest_session_opt: Option<(String, String, u64, u64, u64)> = conn
            .query_row(
                "SELECT id, root_path, total_files, total_directories, total_bytes
                 FROM scan_sessions
                 WHERE drive_id = ?1 AND status IN ('completed', 'completed_with_errors')
                 ORDER BY started_at DESC
                 LIMIT 1",
                [drive_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get::<_, i64>(2)? as u64,
                        row.get::<_, i64>(3)? as u64,
                        row.get::<_, i64>(4)? as u64,
                    ))
                },
            )
            .ok();

        let snapshot_id = Uuid::new_v4().to_string();

        let (session_id, root_path, total_files, total_dirs, total_bytes) = match latest_session_opt {
            Some((sid, rpath, tf, td, tb)) => (Some(sid), rpath, tf, td, tb),
            None => {
                // If no scan exists, use drive mount point
                let mount_point: String = conn
                    .query_row("SELECT mount_point FROM drives WHERE id = ?1", [drive_id], |r| r.get(0))
                    .unwrap_or_else(|_| "/".to_string());
                (None, mount_point, 0, 0, 0)
            }
        };

        conn.execute(
            "INSERT INTO storage_snapshots (id, session_id, drive_id, label, root_path, total_files, total_directories, total_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
            params![
                snapshot_id,
                session_id,
                drive_id,
                label,
                root_path,
                total_files as i64,
                total_dirs as i64,
                total_bytes as i64,
            ],
        )?;

        let created_at: String = conn.query_row(
            "SELECT created_at FROM storage_snapshots WHERE id = ?1",
            [&snapshot_id],
            |r| r.get(0),
        )?;

        Ok(StorageSnapshotRecord {
            id: snapshot_id,
            session_id,
            drive_id: drive_id.to_string(),
            label: label.to_string(),
            root_path,
            total_files,
            total_directories: total_dirs,
            total_bytes,
            created_at,
        })
    }

    /// Differential comparison between two scan sessions (baseline vs target).
    pub fn compare_scans(
        db_pool: &DbPool,
        baseline_session_id: &str,
        target_session_id: &str,
    ) -> Result<ScanComparisonResult> {
        let baseline = Self::get_session_details(db_pool, baseline_session_id)?.session;
        let target = Self::get_session_details(db_pool, target_session_id)?.session;

        let size_delta_bytes = (target.total_bytes as i64) - (baseline.total_bytes as i64);
        let files_delta = (target.total_files as i64) - (baseline.total_files as i64);
        let directories_delta = (target.total_directories as i64) - (baseline.total_directories as i64);

        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();

        // 1. Category deltas
        let mut cat_stmt = conn.prepare(
            "WITH all_cats AS (
                SELECT category FROM category_summaries WHERE session_id = ?1
                UNION
                SELECT category FROM category_summaries WHERE session_id = ?2
             )
             SELECT
                c.category,
                COALESCE(b.total_bytes, 0) as baseline_bytes,
                COALESCE(t.total_bytes, 0) as target_bytes,
                (COALESCE(t.total_bytes, 0) - COALESCE(b.total_bytes, 0)) as delta_bytes,
                COALESCE(b.file_count, 0) as baseline_files,
                COALESCE(t.file_count, 0) as target_files,
                (COALESCE(t.file_count, 0) - COALESCE(b.file_count, 0)) as delta_files
             FROM all_cats c
             LEFT JOIN category_summaries b ON b.session_id = ?1 AND b.category = c.category
             LEFT JOIN category_summaries t ON t.session_id = ?2 AND t.category = c.category
             ORDER BY ABS(delta_bytes) DESC;",
        )?;

        let cat_rows = cat_stmt.query_map(params![baseline_session_id, target_session_id], |row| {
            Ok(CategoryDeltaItem {
                category: row.get(0)?,
                baseline_bytes: row.get::<_, i64>(1)? as u64,
                target_bytes: row.get::<_, i64>(2)? as u64,
                delta_bytes: row.get(3)?,
                baseline_files: row.get::<_, i64>(4)? as u64,
                target_files: row.get::<_, i64>(5)? as u64,
                delta_files: row.get(6)?,
            })
        })?;

        let mut category_deltas = Vec::new();
        for c in cat_rows {
            category_deltas.push(c?);
        }

        // 2. Directory deltas (changes in largest directories)
        let mut dir_stmt = conn.prepare(
            "WITH all_dirs AS (
                SELECT path FROM directories WHERE session_id = ?1
                UNION
                SELECT path FROM directories WHERE session_id = ?2
             )
             SELECT
                c.path,
                COALESCE(b.total_size_bytes, 0) as baseline_size,
                COALESCE(t.total_size_bytes, 0) as target_size,
                (COALESCE(t.total_size_bytes, 0) - COALESCE(b.total_size_bytes, 0)) as delta_bytes
             FROM all_dirs c
             LEFT JOIN directories b ON b.session_id = ?1 AND b.path = c.path
             LEFT JOIN directories t ON t.session_id = ?2 AND t.path = c.path
             WHERE COALESCE(b.total_size_bytes, 0) != COALESCE(t.total_size_bytes, 0)
             ORDER BY ABS(delta_bytes) DESC
             LIMIT 25;",
        )?;

        let dir_rows = dir_stmt.query_map(params![baseline_session_id, target_session_id], |row| {
            Ok(DirectoryDeltaItem {
                path: row.get(0)?,
                baseline_size_bytes: row.get::<_, i64>(1)? as u64,
                target_size_bytes: row.get::<_, i64>(2)? as u64,
                delta_bytes: row.get(3)?,
            })
        })?;

        let mut directory_deltas = Vec::new();
        for d in dir_rows {
            directory_deltas.push(d?);
        }

        // 3. Newly discovered files (in target, not in baseline)
        let added_files_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM files t
             WHERE t.session_id = ?2
               AND NOT EXISTS (
                   SELECT 1 FROM files b WHERE b.session_id = ?1 AND b.path = t.path
               );",
            params![baseline_session_id, target_session_id],
            |r| r.get(0),
        )?;

        let mut added_stmt = conn.prepare(
            "SELECT t.id, t.session_id, t.path, t.name, t.extension, t.parent_path,
                    t.size_bytes, t.allocated_bytes, t.created_at, t.modified_at,
                    t.is_hidden, t.is_system, t.is_symlink
             FROM files t
             WHERE t.session_id = ?2
               AND NOT EXISTS (
                   SELECT 1 FROM files b WHERE b.session_id = ?1 AND b.path = t.path
               )
             ORDER BY t.size_bytes DESC
             LIMIT 50;",
        )?;

        let added_rows = added_stmt.query_map(params![baseline_session_id, target_session_id], |row| {
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

        let mut added_files_sample = Vec::new();
        for a in added_rows {
            added_files_sample.push(a?);
        }

        // 4. Removed files (in baseline, not in target)
        let removed_files_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM files b
             WHERE b.session_id = ?1
               AND NOT EXISTS (
                   SELECT 1 FROM files t WHERE t.session_id = ?2 AND t.path = b.path
               );",
            params![baseline_session_id, target_session_id],
            |r| r.get(0),
        )?;

        let mut removed_stmt = conn.prepare(
            "SELECT b.id, b.session_id, b.path, b.name, b.extension, b.parent_path,
                    b.size_bytes, b.allocated_bytes, b.created_at, b.modified_at,
                    b.is_hidden, b.is_system, b.is_symlink
             FROM files b
             WHERE b.session_id = ?1
               AND NOT EXISTS (
                   SELECT 1 FROM files t WHERE t.session_id = ?2 AND t.path = b.path
               )
             ORDER BY b.size_bytes DESC
             LIMIT 50;",
        )?;

        let removed_rows = removed_stmt.query_map(params![baseline_session_id, target_session_id], |row| {
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

        let mut removed_files_sample = Vec::new();
        for r in removed_rows {
            removed_files_sample.push(r?);
        }

        // 5. Modified files (in both, different size or modification timestamp)
        let modified_files_count: u64 = conn.query_row(
            "SELECT COUNT(*)
             FROM files b
             JOIN files t ON b.path = t.path AND t.session_id = ?2
             WHERE b.session_id = ?1
               AND (b.size_bytes != t.size_bytes OR b.modified_at != t.modified_at);",
            params![baseline_session_id, target_session_id],
            |r| r.get(0),
        )?;

        let mut mod_stmt = conn.prepare(
            "SELECT
                t.path, t.name, t.extension,
                b.size_bytes as baseline_size,
                t.size_bytes as target_size,
                (t.size_bytes - b.size_bytes) as size_delta,
                t.modified_at
             FROM files b
             JOIN files t ON b.path = t.path AND t.session_id = ?2
             WHERE b.session_id = ?1
               AND (b.size_bytes != t.size_bytes OR b.modified_at != t.modified_at)
             ORDER BY ABS(t.size_bytes - b.size_bytes) DESC
             LIMIT 50;",
        )?;

        let mod_rows = mod_stmt.query_map(params![baseline_session_id, target_session_id], |row| {
            Ok(FileModifiedItem {
                path: row.get(0)?,
                name: row.get(1)?,
                extension: row.get(2)?,
                baseline_size_bytes: row.get::<_, i64>(3)? as u64,
                target_size_bytes: row.get::<_, i64>(4)? as u64,
                size_delta_bytes: row.get(5)?,
                modified_at: row.get(6)?,
            })
        })?;

        let mut modified_files_sample = Vec::new();
        for m in mod_rows {
            modified_files_sample.push(m?);
        }

        Ok(ScanComparisonResult {
            baseline_session: baseline,
            target_session: target,
            size_delta_bytes,
            files_delta,
            directories_delta,
            duration_delta_millis: None,
            category_deltas,
            directory_deltas,
            added_files_count,
            removed_files_count,
            modified_files_count,
            added_files_sample,
            removed_files_sample,
            modified_files_sample,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::scanner::ScanManager;
    use crate::storage::migrator::Migrator;

    fn create_test_db() -> DbPool {
        let temp_dir = std::env::temp_dir().join(format!("diskatlas_hist_test_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("history_test.db");
        let pool = DbPool::new(&db_path).unwrap();

        {
            let conn_arc = pool.conn();
            let mut conn = conn_arc.lock().unwrap();
            Migrator::run(&mut conn).unwrap();

            conn.execute(
                "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
                 VALUES ('TEST_HIST_DRIVE', 'Test NVMe', '/mnt/test', 'ext4', 100000000, 50000000);",
                (),
            )
            .unwrap();
        }

        pool
    }

    #[test]
    fn test_interrupted_scan_recovery() {
        let pool = create_test_db();
        {
            let conn_arc = pool.conn();
            let conn = conn_arc.lock().unwrap();

            // Insert orphaned running and paused sessions
            conn.execute(
                "INSERT INTO scan_sessions (id, drive_id, root_path, status, started_at)
                 VALUES ('session_stalled_1', 'TEST_HIST_DRIVE', '/mnt/test', 'running', CURRENT_TIMESTAMP);",
                (),
            ).unwrap();

            conn.execute(
                "INSERT INTO scan_sessions (id, drive_id, root_path, status, started_at)
                 VALUES ('session_stalled_2', 'TEST_HIST_DRIVE', '/mnt/test', 'paused', CURRENT_TIMESTAMP);",
                (),
            ).unwrap();
        }

        let manager = ScanManager::new();
        let recovered = manager.recover_interrupted_scans(&pool).unwrap();
        assert_eq!(recovered, 2);

        // Verify status is now 'interrupted'
        let page = HistoryService::get_scan_sessions(&pool, 1, 10, None, Some("interrupted".into())).unwrap();
        assert_eq!(page.total_count, 2);
        assert!(page.items.iter().all(|s| s.status == ScanStatus::Interrupted));
    }

    #[test]
    fn test_compare_scans_differential_growth_and_file_deltas() {
        let pool = create_test_db();
        let conn_arc = pool.conn();
        let conn = conn_arc.lock().unwrap();

        // 1. Insert Baseline Session
        conn.execute(
            "INSERT INTO scan_sessions (id, drive_id, root_path, status, total_files, total_directories, total_bytes, started_at, completed_at)
             VALUES ('session_base', 'TEST_HIST_DRIVE', '/data', 'completed', 2, 1, 6000, '2026-11-01 10:00:00', '2026-11-01 10:02:00');",
            (),
        ).unwrap();

        conn.execute(
            "INSERT INTO category_summaries (session_id, category, file_count, total_bytes)
             VALUES ('session_base', 'documents', 1, 1000), ('session_base', 'code', 1, 5000);",
            (),
        ).unwrap();

        conn.execute(
            "INSERT INTO directories (session_id, path, name, total_size_bytes, file_count, dir_count)
             VALUES ('session_base', '/data', 'data', 6000, 2, 1);",
            (),
        ).unwrap();

        // Baseline files
        conn.execute(
            "INSERT INTO files (session_id, path, name, extension, parent_path, size_bytes, modified_at)
             VALUES ('session_base', '/data/file1.txt', 'file1.txt', 'txt', '/data', 1000, 100);",
            (),
        ).unwrap();

        conn.execute(
            "INSERT INTO files (session_id, path, name, extension, parent_path, size_bytes, modified_at)
             VALUES ('session_base', '/data/removed.rs', 'removed.rs', 'rs', '/data', 5000, 100);",
            (),
        ).unwrap();

        // 2. Insert Target Session (file1 modified +500, removed.rs deleted, added.mp4 +20000)
        conn.execute(
            "INSERT INTO scan_sessions (id, drive_id, root_path, status, total_files, total_directories, total_bytes, started_at, completed_at)
             VALUES ('session_target', 'TEST_HIST_DRIVE', '/data', 'completed', 2, 1, 21500, '2026-11-04 10:00:00', '2026-11-04 10:03:00');",
            (),
        ).unwrap();

        conn.execute(
            "INSERT INTO category_summaries (session_id, category, file_count, total_bytes)
             VALUES ('session_target', 'documents', 1, 1500), ('session_target', 'videos', 1, 20000);",
            (),
        ).unwrap();

        conn.execute(
            "INSERT INTO directories (session_id, path, name, total_size_bytes, file_count, dir_count)
             VALUES ('session_target', '/data', 'data', 21500, 2, 1);",
            (),
        ).unwrap();

        // Target files
        conn.execute(
            "INSERT INTO files (session_id, path, name, extension, parent_path, size_bytes, modified_at)
             VALUES ('session_target', '/data/file1.txt', 'file1.txt', 'txt', '/data', 1500, 200);",
            (),
        ).unwrap();

        conn.execute(
            "INSERT INTO files (session_id, path, name, extension, parent_path, size_bytes, modified_at)
             VALUES ('session_target', '/data/added.mp4', 'added.mp4', 'mp4', '/data', 20000, 200);",
            (),
        ).unwrap();

        drop(conn);

        // 3. Run comparison
        let diff = HistoryService::compare_scans(&pool, "session_base", "session_target").unwrap();

        assert_eq!(diff.size_delta_bytes, 15500); // 21500 - 6000
        assert_eq!(diff.files_delta, 0);          // 2 - 2
        assert_eq!(diff.added_files_count, 1);    // added.mp4
        assert_eq!(diff.removed_files_count, 1);  // removed.rs
        assert_eq!(diff.modified_files_count, 1); // file1.txt

        assert_eq!(diff.added_files_sample[0].name, "added.mp4");
        assert_eq!(diff.removed_files_sample[0].name, "removed.rs");
        assert_eq!(diff.modified_files_sample[0].name, "file1.txt");
        assert_eq!(diff.modified_files_sample[0].size_delta_bytes, 500);

        // Check directory delta
        assert_eq!(diff.directory_deltas.len(), 1);
        assert_eq!(diff.directory_deltas[0].path, "/data");
        assert_eq!(diff.directory_deltas[0].delta_bytes, 15500);

        // 4. Test snapshot creation and cascading deletion
        let snap = HistoryService::create_manual_snapshot(&pool, "TEST_HIST_DRIVE", "Manual Checkpoint").unwrap();
        assert_eq!(snap.label, "Manual Checkpoint");
        assert_eq!(snap.total_files, 2);

        let snaps = HistoryService::get_storage_snapshots(&pool, Some("TEST_HIST_DRIVE".into()), 10).unwrap();
        assert_eq!(snaps.len(), 1);

        // Test cascade delete
        HistoryService::delete_scan_session(&pool, "session_base").unwrap();
        let res = HistoryService::get_session_details(&pool, "session_base");
        assert!(res.is_err(), "Session should be deleted");
    }
}
