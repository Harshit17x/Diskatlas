pub mod safeguards;

use rusqlite::Connection;
use std::path::Path;
use uuid::Uuid;
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::models::cleanup::{
    CleanupAuditQuery, CleanupAuditRecord, CleanupAuditResponse, CleanupItemCandidate,
    CleanupOperationResult, CleanupPlan, ExecuteCleanupRequest, ExecuteCleanupResponse,
};
use crate::models::intelligence::{RecommendationGroup, RiskLevel};
use crate::services::intelligence::IntelligenceService;

pub struct CleanupService;

impl CleanupService {
    /// Constructs a transparent, user-controlled cleanup plan from active recommendations
    pub fn get_cleanup_plan(conn: &Connection, session_id: &str) -> Result<CleanupPlan> {
        // Ensure recommendations exist for this session
        let existing_count: i64 = conn.query_row(
            "SELECT count(*) FROM storage_recommendations WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        ).unwrap_or(0);

        if existing_count == 0 {
            info!("No existing recommendations for session {}, generating intelligence profile", session_id);
            let _ = IntelligenceService::generate_intelligence_profile(conn, Some(session_id))?;
        }

        let mut stmt = conn.prepare(
            "SELECT id, session_id, path, name, size_bytes, category, rec_group, risk_level,
                    reason, confidence, recoverable_bytes, expected_consequences, recommended_action
             FROM storage_recommendations
             WHERE session_id = ?1
             ORDER BY size_bytes DESC",
        )?;

        let rows = stmt.query_map([session_id], |row| {
            let id: String = row.get(0)?;
            let _sid: String = row.get(1)?;
            let path: String = row.get(2)?;
            let name: String = row.get(3)?;
            let size_bytes: i64 = row.get(4)?;
            let category: String = row.get(5)?;
            let group_str: String = row.get(6)?;
            let risk_str: String = row.get(7)?;
            let reason: String = row.get(8)?;
            let confidence: f64 = row.get(9)?;
            let recoverable_bytes: i64 = row.get(10)?;
            let expected_consequences: String = row.get(11)?;
            let recommended_action: String = row.get(12)?;

            let group = RecommendationGroup::from_str(&group_str);
            let risk_level = RiskLevel::from_str(&risk_str);

            Ok((
                id, path, name, size_bytes, category, group, risk_level,
                reason, confidence, recoverable_bytes, expected_consequences, recommended_action
            ))
        })?;

        let mut candidates = Vec::new();
        let mut total_bytes = 0i64;
        let mut selected_bytes = 0i64;
        let mut selected_items = 0usize;
        let mut safe_items = 0usize;
        let mut safe_bytes = 0i64;
        let mut review_required_items = 0usize;
        let mut review_required_bytes = 0i64;
        let mut blocked_items = 0usize;
        let mut has_cloud_synced = false;

        for item_res in rows {
            let (
                id, path, name, size_bytes, category, group, risk_level,
                reason, confidence, recoverable_bytes, expected_consequences, recommended_action
            ) = item_res?;

            let path_obj = Path::new(&path);
            let exists_on_disk = path_obj.exists();
            let (is_cloud, cloud_warning) = safeguards::detect_cloud_sync(path_obj);
            if is_cloud {
                has_cloud_synced = true;
            }

            let (is_critical, critical_reason) = safeguards::is_system_critical_path(path_obj);

            let is_blocked = is_critical || group == RecommendationGroup::SystemProtected;
            let block_reason = if is_critical {
                critical_reason.map(|r| r.to_string())
            } else if group == RecommendationGroup::SystemProtected {
                Some("Operating system protected item - manual modification prohibited".to_string())
            } else {
                None
            };

            // Pre-selection policy:
            // Safe to review items with low/none risk are selected by default.
            // Items requiring review or system protected are unselected.
            let selected = !is_blocked
                && group == RecommendationGroup::SafeToReview
                && (risk_level == RiskLevel::Low || risk_level == RiskLevel::None);

            if is_blocked {
                blocked_items += 1;
            } else if group == RecommendationGroup::SafeToReview {
                safe_items += 1;
                safe_bytes += size_bytes;
            } else {
                review_required_items += 1;
                review_required_bytes += size_bytes;
            }

            if selected {
                selected_items += 1;
                selected_bytes += size_bytes;
            }
            total_bytes += size_bytes;

            candidates.push(CleanupItemCandidate {
                id,
                path,
                name,
                size_bytes,
                category,
                group,
                risk_level,
                reason,
                confidence,
                recoverable_bytes,
                expected_consequences,
                recommended_action,
                is_cloud_synced: is_cloud,
                cloud_warning,
                is_blocked,
                block_reason,
                selected,
                exists_on_disk,
            });
        }

        let batch_id = Uuid::new_v4().to_string();
        let total_items = candidates.len();

        Ok(CleanupPlan {
            batch_id,
            session_id: session_id.to_string(),
            candidates,
            total_items,
            total_bytes,
            selected_items,
            selected_bytes,
            safe_items,
            safe_bytes,
            review_required_items,
            review_required_bytes,
            blocked_items,
            has_cloud_synced,
        })
    }

    /// Executes the user-confirmed cleanup operation.
    ///
    /// SAFEGUARDS:
    /// 1. Requires exact confirmation phrase "CONFIRM_CLEANUP".
    /// 2. Re-validates target paths immediately before deletion.
    /// 3. Strictly blocks critical OS/boot/driver components.
    /// 4. Prefers reversible system Recycle Bin / Trash.
    /// 5. Logs every action into the audit history table.
    pub fn execute_cleanup(conn: &Connection, request: ExecuteCleanupRequest) -> Result<ExecuteCleanupResponse> {
        // Enforce explicit user confirmation phrase
        if request.confirmation_phrase.trim() != "CONFIRM_CLEANUP" {
            return Err(AppError::ValidationFailed(
                "Cleanup aborted: Explicit confirmation phrase 'CONFIRM_CLEANUP' must be entered. Unconfirmed or automated execution is strictly disallowed.".to_string(),
            ));
        }

        if request.item_ids.is_empty() {
            return Err(AppError::ValidationFailed(
                "No items were selected for cleanup.".to_string(),
            ));
        }

        let mut results = Vec::new();
        let mut successful_count = 0usize;
        let mut failed_count = 0usize;
        let mut bytes_reclaimed = 0i64;

        for item_id in &request.item_ids {
            // Retrieve item details from storage_recommendations
            let item_opt = conn.query_row(
                "SELECT path, name, size_bytes, category, risk_level
                 FROM storage_recommendations
                 WHERE id = ?1",
                [item_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            );

            let (path, name, size_bytes, category, risk_level) = match item_opt {
                Ok(data) => data,
                Err(_) => {
                    warn!("Item {} not found in recommendations, skipping", item_id);
                    continue;
                }
            };

            let path_obj = Path::new(&path);

            // 1. Re-validate security safeguards immediately before deletion
            let (is_critical, critical_reason) = safeguards::is_system_critical_path(path_obj);
            if is_critical {
                let err_msg = format!(
                    "Security block: {}",
                    critical_reason.unwrap_or("Protected operating system or boot component")
                );
                warn!("Refusing to clean protected path: {} ({})", path, err_msg);

                Self::log_audit_event(
                    conn,
                    &request.batch_id,
                    Some(&request.session_id),
                    &path,
                    &name,
                    size_bytes,
                    &category,
                    &risk_level,
                    "skipped",
                    "blocked_security",
                    false,
                    Some(&err_msg),
                )?;

                results.push(CleanupOperationResult {
                    id: item_id.clone(),
                    path,
                    name,
                    size_bytes,
                    status: "blocked_security".to_string(),
                    cleanup_method: "skipped".to_string(),
                    is_reversible: false,
                    error_message: Some(err_msg),
                });
                failed_count += 1;
                continue;
            }

            // 2. Validate current disk existence
            if !path_obj.exists() {
                let err_msg = "Path does not exist on disk (may have been moved or previously deleted)".to_string();
                Self::log_audit_event(
                    conn,
                    &request.batch_id,
                    Some(&request.session_id),
                    &path,
                    &name,
                    size_bytes,
                    &category,
                    &risk_level,
                    "skipped",
                    "not_found",
                    false,
                    Some(&err_msg),
                )?;

                results.push(CleanupOperationResult {
                    id: item_id.clone(),
                    path,
                    name,
                    size_bytes,
                    status: "not_found".to_string(),
                    cleanup_method: "skipped".to_string(),
                    is_reversible: false,
                    error_message: Some(err_msg),
                });
                failed_count += 1;
                continue;
            }

            // 3. Perform cleanup operation (preferring reversible Recycle Bin / Trash)
            if request.use_recycle_bin {
                match trash::delete(path_obj) {
                    Ok(()) => {
                        info!("Successfully sent to Recycle Bin / Trash: {}", path);
                        Self::log_audit_event(
                            conn,
                            &request.batch_id,
                            Some(&request.session_id),
                            &path,
                            &name,
                            size_bytes,
                            &category,
                            &risk_level,
                            "recycle_bin",
                            "success",
                            true,
                            None,
                        )?;

                        results.push(CleanupOperationResult {
                            id: item_id.clone(),
                            path: path.clone(),
                            name,
                            size_bytes,
                            status: "success".to_string(),
                            cleanup_method: "recycle_bin".to_string(),
                            is_reversible: true,
                            error_message: None,
                        });
                        successful_count += 1;
                        bytes_reclaimed += size_bytes;

                        // Delete from active recommendations table
                        let _ = conn.execute("DELETE FROM storage_recommendations WHERE id = ?1", [item_id]);
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        let status = if err_str.to_lowercase().contains("permission") || err_str.to_lowercase().contains("access") {
                            "permission_denied"
                        } else if err_str.to_lowercase().contains("in use") || err_str.to_lowercase().contains("busy") || err_str.to_lowercase().contains("sharing") {
                            "in_use"
                        } else {
                            "failed"
                        };

                        warn!("Recycle bin error for {}: {}", path, err_str);
                        Self::log_audit_event(
                            conn,
                            &request.batch_id,
                            Some(&request.session_id),
                            &path,
                            &name,
                            size_bytes,
                            &category,
                            &risk_level,
                            "recycle_bin",
                            status,
                            true,
                            Some(&err_str),
                        )?;

                        results.push(CleanupOperationResult {
                            id: item_id.clone(),
                            path,
                            name,
                            size_bytes,
                            status: status.to_string(),
                            cleanup_method: "recycle_bin".to_string(),
                            is_reversible: true,
                            error_message: Some(err_str),
                        });
                        failed_count += 1;
                    }
                }
            } else {
                // Permanent deletion fallback (only when user explicitly opted out of recycle bin)
                let delete_res = if path_obj.is_dir() {
                    std::fs::remove_dir_all(path_obj)
                } else {
                    std::fs::remove_file(path_obj)
                };

                match delete_res {
                    Ok(()) => {
                        info!("Permanently deleted item: {}", path);
                        Self::log_audit_event(
                            conn,
                            &request.batch_id,
                            Some(&request.session_id),
                            &path,
                            &name,
                            size_bytes,
                            &category,
                            &risk_level,
                            "permanent_delete",
                            "success",
                            false,
                            None,
                        )?;

                        results.push(CleanupOperationResult {
                            id: item_id.clone(),
                            path: path.clone(),
                            name,
                            size_bytes,
                            status: "success".to_string(),
                            cleanup_method: "permanent_delete".to_string(),
                            is_reversible: false,
                            error_message: None,
                        });
                        successful_count += 1;
                        bytes_reclaimed += size_bytes;

                        let _ = conn.execute("DELETE FROM storage_recommendations WHERE id = ?1", [item_id]);
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        let status = match e.kind() {
                            std::io::ErrorKind::PermissionDenied => "permission_denied",
                            std::io::ErrorKind::NotFound => "not_found",
                            _ => "failed",
                        };

                        Self::log_audit_event(
                            conn,
                            &request.batch_id,
                            Some(&request.session_id),
                            &path,
                            &name,
                            size_bytes,
                            &category,
                            &risk_level,
                            "permanent_delete",
                            status,
                            false,
                            Some(&err_str),
                        )?;

                        results.push(CleanupOperationResult {
                            id: item_id.clone(),
                            path,
                            name,
                            size_bytes,
                            status: status.to_string(),
                            cleanup_method: "permanent_delete".to_string(),
                            is_reversible: false,
                            error_message: Some(err_str),
                        });
                        failed_count += 1;
                    }
                }
            }
        }

        Ok(ExecuteCleanupResponse {
            batch_id: request.batch_id,
            total_attempted: request.item_ids.len(),
            successful_count,
            failed_count,
            bytes_reclaimed,
            results,
        })
    }

    /// Records an individual cleanup action into the persistent audit history table
    fn log_audit_event(
        conn: &Connection,
        batch_id: &str,
        session_id: Option<&str>,
        path: &str,
        name: &str,
        size_bytes: i64,
        category: &str,
        risk_level: &str,
        cleanup_method: &str,
        status: &str,
        is_reversible: bool,
        error_message: Option<&str>,
    ) -> Result<()> {
        let event_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO cleanup_audit_events
             (id, batch_id, session_id, path, name, size_bytes, category, risk_level,
              cleanup_method, status, is_reversible, error_message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, CURRENT_TIMESTAMP)",
            (
                &event_id,
                batch_id,
                session_id,
                path,
                name,
                size_bytes,
                category,
                risk_level,
                cleanup_method,
                status,
                if is_reversible { 1 } else { 0 },
                error_message,
            ),
        )?;
        Ok(())
    }

    /// Queries paginated cleanup audit history
    pub fn get_cleanup_audit_history(
        conn: &Connection,
        query: CleanupAuditQuery,
    ) -> Result<CleanupAuditResponse> {
        let page = query.page.max(1);
        let page_size = query.page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;

        let (where_clause, count_sql, query_sql) = match (&query.session_id, &query.status) {
            (Some(sid), Some(status)) => (
                "WHERE session_id = ?1 AND status = ?2",
                format!("SELECT count(*) FROM cleanup_audit_events WHERE session_id = '{}' AND status = '{}'", sid, status),
                format!(
                    "SELECT id, batch_id, session_id, path, name, size_bytes, category, risk_level,
                            cleanup_method, status, is_reversible, error_message, created_at
                     FROM cleanup_audit_events
                     WHERE session_id = '{}' AND status = '{}'
                     ORDER BY created_at DESC LIMIT {} OFFSET {}",
                    sid, status, page_size, offset
                ),
            ),
            (Some(sid), None) => (
                "WHERE session_id = ?1",
                format!("SELECT count(*) FROM cleanup_audit_events WHERE session_id = '{}'", sid),
                format!(
                    "SELECT id, batch_id, session_id, path, name, size_bytes, category, risk_level,
                            cleanup_method, status, is_reversible, error_message, created_at
                     FROM cleanup_audit_events
                     WHERE session_id = '{}'
                     ORDER BY created_at DESC LIMIT {} OFFSET {}",
                    sid, page_size, offset
                ),
            ),
            (None, Some(status)) => (
                "WHERE status = ?1",
                format!("SELECT count(*) FROM cleanup_audit_events WHERE status = '{}'", status),
                format!(
                    "SELECT id, batch_id, session_id, path, name, size_bytes, category, risk_level,
                            cleanup_method, status, is_reversible, error_message, created_at
                     FROM cleanup_audit_events
                     WHERE status = '{}'
                     ORDER BY created_at DESC LIMIT {} OFFSET {}",
                    status, page_size, offset
                ),
            ),
            (None, None) => (
                "",
                "SELECT count(*) FROM cleanup_audit_events".to_string(),
                format!(
                    "SELECT id, batch_id, session_id, path, name, size_bytes, category, risk_level,
                            cleanup_method, status, is_reversible, error_message, created_at
                     FROM cleanup_audit_events
                     ORDER BY created_at DESC LIMIT {} OFFSET {}",
                    page_size, offset
                ),
            ),
        };
        let _ = where_clause;

        let total_items: usize = conn.query_row(&count_sql, [], |row| row.get(0)).unwrap_or(0);
        let total_pages = ((total_items as f64) / (page_size as f64)).ceil() as u32;

        let mut stmt = conn.prepare(&query_sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(CleanupAuditRecord {
                id: row.get(0)?,
                batch_id: row.get(1)?,
                session_id: row.get(2)?,
                path: row.get(3)?,
                name: row.get(4)?,
                size_bytes: row.get(5)?,
                category: row.get(6)?,
                risk_level: row.get(7)?,
                cleanup_method: row.get(8)?,
                status: row.get(9)?,
                is_reversible: row.get::<_, i64>(10)? != 0,
                error_message: row.get(11)?,
                created_at: row.get(12)?,
            })
        })?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }

        Ok(CleanupAuditResponse {
            items,
            total_items,
            page,
            page_size,
            total_pages: total_pages.max(1),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrator::Migrator;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn test_cleanup_plan_and_execution_with_synthetic_files() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrator::run(&mut conn).unwrap();

        // 1. Create temporary synthetic folder for safe testing
        let temp_dir = std::env::temp_dir().join(format!("diskatlas_clean_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let synth_file_path = temp_dir.join("test_stale_archive.zip");
        {
            let mut f = File::create(&synth_file_path).unwrap();
            f.write_all(b"synthetic mock content for safe cleanup center verification").unwrap();
        }
        let file_size = fs::metadata(&synth_file_path).unwrap().len() as i64;

        // 2. Insert dummy drive, session, and recommendations
        conn.execute(
            "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes, is_removable, is_system)
             VALUES ('DRIVE_C', 'Local Disk', 'C:', 'NTFS', 1000000000, 500000000, 0, 1)",
            [],
        ).unwrap();

        let session_id = "test-session-clean";
        conn.execute(
            "INSERT INTO scan_sessions (id, drive_id, root_path, status, started_at)
             VALUES (?1, 'DRIVE_C', '/test', 'completed', CURRENT_TIMESTAMP)",
            [session_id],
        ).unwrap();

        let rec_id = "rec-synth-1";
        conn.execute(
            "INSERT INTO storage_recommendations
             (id, session_id, path, name, size_bytes, category, rec_group, risk_level,
              reason, confidence, recoverable_bytes, expected_consequences, recommended_action)
             VALUES (?1, ?2, ?3, 'test_stale_archive.zip', ?4, 'Downloads & Archives',
                     'safe_to_review', 'low', 'Stale temporary archive', 0.95, ?4,
                     'File can be recovered from Recycle Bin', 'Delete to Recycle Bin')",
            (rec_id, session_id, synth_file_path.to_str().unwrap(), file_size),
        ).unwrap();

        // Also insert a critical system component recommendation to verify safety block
        let crit_rec_id = "rec-critical-sys";
        conn.execute(
            "INSERT INTO storage_recommendations
             (id, session_id, path, name, size_bytes, category, rec_group, risk_level,
              reason, confidence, recoverable_bytes, expected_consequences, recommended_action)
             VALUES (?1, ?2, 'C:\\Windows\\System32\\drivers\\synthetic_driver.sys', 'synthetic_driver.sys',
                     1024, 'Operating System', 'system_protected', 'critical',
                     'Critical system file', 1.0, 0,
                     'System failure if deleted', 'Do not delete')",
            (crit_rec_id, session_id),
        ).unwrap();

        // 3. Test get_cleanup_plan
        let plan = CleanupService::get_cleanup_plan(&conn, session_id).unwrap();
        assert_eq!(plan.total_items, 2);
        assert_eq!(plan.blocked_items, 1);

        let safe_candidate = plan.candidates.iter().find(|c| c.id == rec_id).unwrap();
        assert!(safe_candidate.selected);
        assert!(!safe_candidate.is_blocked);

        let crit_candidate = plan.candidates.iter().find(|c| c.id == crit_rec_id).unwrap();
        assert!(!crit_candidate.selected);
        assert!(crit_candidate.is_blocked);
        assert!(crit_candidate.block_reason.is_some());

        // 4. Test execution without confirmation phrase fails
        let invalid_req = ExecuteCleanupRequest {
            batch_id: plan.batch_id.clone(),
            session_id: session_id.to_string(),
            item_ids: vec![rec_id.to_string()],
            use_recycle_bin: true,
            confirmation_phrase: "WRONG_PHRASE".to_string(),
        };
        assert!(CleanupService::execute_cleanup(&conn, invalid_req).is_err());

        // 5. Test execution trying to clean critical path gets blocked by security safeguards
        let crit_req = ExecuteCleanupRequest {
            batch_id: plan.batch_id.clone(),
            session_id: session_id.to_string(),
            item_ids: vec![crit_rec_id.to_string()],
            use_recycle_bin: false,
            confirmation_phrase: "CONFIRM_CLEANUP".to_string(),
        };
        let crit_res = CleanupService::execute_cleanup(&conn, crit_req).unwrap();
        assert_eq!(crit_res.failed_count, 1);
        assert_eq!(crit_res.results[0].status, "blocked_security");

        // 6. Test valid execution for synthetic file
        let valid_req = ExecuteCleanupRequest {
            batch_id: plan.batch_id.clone(),
            session_id: session_id.to_string(),
            item_ids: vec![rec_id.to_string()],
            use_recycle_bin: false, // Use direct removal for CI tempfile test
            confirmation_phrase: "CONFIRM_CLEANUP".to_string(),
        };
        let valid_res = CleanupService::execute_cleanup(&conn, valid_req).unwrap();
        assert_eq!(valid_res.successful_count, 1);
        assert_eq!(valid_res.bytes_reclaimed, file_size);
        assert!(!synth_file_path.exists());

        // 7. Verify audit history records
        let audit = CleanupService::get_cleanup_audit_history(&conn, CleanupAuditQuery {
            session_id: None,
            status: None,
            page: 1,
            page_size: 10,
        }).unwrap();
        assert_eq!(audit.total_items, 2); // 1 blocked_security, 1 success

        // Clean up test dir
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
