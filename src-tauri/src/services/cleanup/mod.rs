pub mod safeguards;

use rusqlite::{params, Connection};
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
    pub fn get_cleanup_plan(conn: &Connection, session_id: Option<&str>) -> Result<CleanupPlan> {
        let sid = crate::storage::resolve_session_id(conn, session_id)?;

        // Ensure recommendations exist for this session when indexed files/dirs are present
        let existing_count: i64 = conn.query_row(
            "SELECT count(*) FROM storage_recommendations WHERE session_id = ?1",
            [&sid],
            |row| row.get(0),
        ).unwrap_or(0);

        if existing_count == 0 {
            let has_indexed_items: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM files WHERE session_id = ?1)
                     OR EXISTS(SELECT 1 FROM directories WHERE session_id = ?1)",
                    [&sid],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if has_indexed_items {
                info!("No existing recommendations for session {}, generating intelligence profile", sid);
                let _ = IntelligenceService::generate_intelligence_profile(conn, Some(&sid))?;
            }
        }

        let mut stmt = conn.prepare(
            "SELECT id, session_id, path, name, size_bytes, category, rec_group, risk_level,
                    reason, confidence, recoverable_bytes, expected_consequences, recommended_action
             FROM storage_recommendations
             WHERE session_id = ?1
             ORDER BY size_bytes DESC",
        )?;

        let rows = stmt.query_map([&sid], |row| {
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
            session_id: sid,
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

        let audit_session_id = Self::existing_session_id(conn, &request.session_id);
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
                    audit_session_id.as_deref(),
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
                    audit_session_id.as_deref(),
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

            // 3. Perform deletion (preferring reversible Recycle Bin / Trash)
            let op_res: std::result::Result<(&str, bool), (&str, bool, &str, String)> =
                if request.use_recycle_bin {
                    trash::delete(path_obj)
                        .map(|_| ("recycle_bin", true))
                        .map_err(|e| {
                            let err_str = e.to_string();
                            let lower = err_str.to_lowercase();
                            let status = if lower.contains("permission") || lower.contains("access") {
                                "permission_denied"
                            } else if lower.contains("in use")
                                || lower.contains("busy")
                                || lower.contains("sharing")
                            {
                                "in_use"
                            } else {
                                "failed"
                            };
                            ("recycle_bin", true, status, err_str)
                        })
                } else {
                    Self::permanently_delete_path(path_obj)
                        .map(|_| ("permanent_delete", false))
                        .map_err(|e| {
                            let status = match e.kind() {
                                std::io::ErrorKind::PermissionDenied => "permission_denied",
                                std::io::ErrorKind::NotFound => "not_found",
                                _ => "failed",
                            };
                            ("permanent_delete", false, status, e.to_string())
                        })
                };

            match op_res {
                Ok((method, is_reversible)) => {
                    info!(method = method, path = %path, "Cleanup deletion succeeded");
                    Self::log_audit_event(
                        conn,
                        &request.batch_id,
                        audit_session_id.as_deref(),
                        &path,
                        &name,
                        size_bytes,
                        &category,
                        &risk_level,
                        method,
                        "success",
                        is_reversible,
                        None,
                    )?;

                    results.push(CleanupOperationResult {
                        id: item_id.clone(),
                        path: path.clone(),
                        name,
                        size_bytes,
                        status: "success".to_string(),
                        cleanup_method: method.to_string(),
                        is_reversible,
                        error_message: None,
                    });
                    successful_count += 1;
                    bytes_reclaimed += size_bytes;

                    if let Err(e) = Self::record_deleted_path_in_db(
                        conn,
                        audit_session_id.as_deref(),
                        item_id,
                        &path,
                    ) {
                        warn!(path = %path, error = %e, "Failed to update DB rollups after cleanup");
                    }
                }
                Err((method, is_reversible, status, err_str)) => {
                    warn!(method = method, path = %path, error = %err_str, "Cleanup deletion failed");
                    Self::log_audit_event(
                        conn,
                        &request.batch_id,
                        audit_session_id.as_deref(),
                        &path,
                        &name,
                        size_bytes,
                        &category,
                        &risk_level,
                        method,
                        status,
                        is_reversible,
                        Some(&err_str),
                    )?;

                    results.push(CleanupOperationResult {
                        id: item_id.clone(),
                        path,
                        name,
                        size_bytes,
                        status: status.to_string(),
                        cleanup_method: method.to_string(),
                        is_reversible,
                        error_message: Some(err_str),
                    });
                    failed_count += 1;
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

    fn existing_session_id(conn: &Connection, session_id: &str) -> Option<String> {
        let trimmed = session_id.trim();
        if trimmed.is_empty() {
            return None;
        }
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM scan_sessions WHERE id = ?1)",
                [trimmed],
                |row| row.get(0),
            )
            .unwrap_or(false);
        if exists {
            Some(trimmed.to_string())
        } else {
            None
        }
    }

    fn permanently_delete_path(path_obj: &Path) -> std::io::Result<()> {
        let is_symlink = std::fs::symlink_metadata(path_obj)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);

        if is_symlink {
            #[cfg(target_os = "windows")]
            {
                if path_obj.is_dir() {
                    std::fs::remove_dir(path_obj)
                } else {
                    std::fs::remove_file(path_obj)
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                std::fs::remove_file(path_obj)
            }
        } else if path_obj.is_dir() {
            std::fs::remove_dir_all(path_obj)
        } else {
            std::fs::remove_file(path_obj)
        }
    }

    /// Atomically removes deleted items from recommendations/files/directories/file_hashes
    /// and rolls back counts on `scan_sessions`, ancestor `directories`, `category_summaries`,
    /// and `classification_summaries`.
    fn record_deleted_path_in_db(
        conn: &Connection,
        session_id: Option<&str>,
        item_id: &str,
        path: &str,
    ) -> Result<()> {
        let child_pattern = crate::storage::child_like_pattern(path);
        let escaped_path = crate::storage::escape_like_pattern(path);

        let tx = conn.unchecked_transaction()?;

        // Always remove recommendation and cached file hashes regardless of session presence.
        tx.execute(
            "DELETE FROM storage_recommendations WHERE id = ?1 OR path = ?2 OR path LIKE ?3 ESCAPE '\\'",
            params![item_id, path, &child_pattern],
        )?;
        tx.execute(
            "DELETE FROM file_hashes WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\'",
            params![path, &child_pattern],
        )?;

        if let Some(sid) = session_id {
            let (removed_files, removed_bytes): (i64, i64) = tx.query_row(
                "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0)
                 FROM files
                 WHERE session_id = ?1 AND (path = ?2 OR path LIKE ?3 ESCAPE '\\')",
                params![sid, path, &child_pattern],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

            let removed_dirs: i64 = tx.query_row(
                "SELECT COUNT(*)
                 FROM directories
                 WHERE session_id = ?1 AND (path = ?2 OR path LIKE ?3 ESCAPE '\\')",
                params![sid, path, &child_pattern],
                |row| row.get(0),
            )?;

            let deleted_dir_parent: Option<String> = tx
                .query_row(
                    "SELECT parent_path FROM directories WHERE session_id = ?1 AND path = ?2",
                    params![sid, path],
                    |row| row.get(0),
                )
                .ok()
                .flatten();

            // Capture per-category & per-classification rollups before deleting file rows
            let mut cat_deltas: Vec<(String, i64, i64)> = Vec::new();
            {
                let mut stmt = tx.prepare(
                    "SELECT COALESCE(NULLIF(category, ''), 'unknown'), COUNT(*), COALESCE(SUM(size_bytes), 0)
                     FROM files
                     WHERE session_id = ?1 AND (path = ?2 OR path LIKE ?3 ESCAPE '\\')
                     GROUP BY COALESCE(NULLIF(category, ''), 'unknown')",
                )?;
                let rows = stmt.query_map(params![sid, path, &child_pattern], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?;
                for r in rows {
                    cat_deltas.push(r?);
                }
            }

            let mut class_deltas: Vec<(String, Option<String>, Option<String>, i64, i64)> =
                Vec::new();
            {
                let mut stmt = tx.prepare(
                    "SELECT COALESCE(NULLIF(category, ''), 'unknown'), subcategory, app_name, COUNT(*), COALESCE(SUM(size_bytes), 0)
                     FROM files
                     WHERE session_id = ?1 AND (path = ?2 OR path LIKE ?3 ESCAPE '\\')
                     GROUP BY COALESCE(NULLIF(category, ''), 'unknown'), subcategory, app_name",
                )?;
                let rows = stmt.query_map(params![sid, path, &child_pattern], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
                })?;
                for r in rows {
                    class_deltas.push(r?);
                }
            }

            let mut parent_buckets: Vec<(String, i64, i64)> = Vec::new();
            {
                let mut stmt = tx.prepare(
                    "SELECT parent_path, COUNT(*), COALESCE(SUM(size_bytes), 0)
                     FROM files
                     WHERE session_id = ?1 AND (path = ?2 OR path LIKE ?3 ESCAPE '\\')
                     GROUP BY parent_path",
                )?;
                let rows = stmt.query_map(params![sid, path, &child_pattern], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?;
                for r in rows {
                    parent_buckets.push(r?);
                }
            }

            tx.execute(
                "DELETE FROM files WHERE session_id = ?1 AND (path = ?2 OR path LIKE ?3 ESCAPE '\\')",
                params![sid, path, &child_pattern],
            )?;
            tx.execute(
                "DELETE FROM directories WHERE session_id = ?1 AND (path = ?2 OR path LIKE ?3 ESCAPE '\\')",
                params![sid, path, &child_pattern],
            )?;

            if removed_files > 0 || removed_bytes > 0 || removed_dirs > 0 {
                tx.execute(
                    "UPDATE scan_sessions
                     SET total_files = MAX(0, total_files - ?1),
                         total_bytes = MAX(0, total_bytes - ?2),
                         total_directories = MAX(0, total_directories - ?3)
                     WHERE id = ?4",
                    params![removed_files, removed_bytes, removed_dirs, sid],
                )?;
            }

            for (cat, cnt, bytes) in cat_deltas {
                tx.execute(
                    "UPDATE category_summaries
                     SET file_count = MAX(0, file_count - ?1),
                         total_bytes = MAX(0, total_bytes - ?2)
                     WHERE session_id = ?3 AND category = ?4",
                    params![cnt, bytes, sid, cat],
                )?;
            }
            tx.execute(
                "DELETE FROM category_summaries WHERE session_id = ?1 AND file_count <= 0 AND total_bytes <= 0",
                [sid],
            )?;

            for (cat, subcat, app_name, cnt, bytes) in class_deltas {
                tx.execute(
                    "UPDATE classification_summaries
                     SET file_count = MAX(0, file_count - ?1),
                         total_bytes = MAX(0, total_bytes - ?2)
                     WHERE session_id = ?3 AND category = ?4 AND subcategory IS ?5 AND app_name IS ?6",
                    params![cnt, bytes, sid, cat, subcat, app_name],
                )?;
            }
            tx.execute(
                "DELETE FROM classification_summaries WHERE session_id = ?1 AND file_count <= 0 AND total_bytes <= 0",
                [sid],
            )?;

            if let Some(parent_dir) = deleted_dir_parent {
                tx.execute(
                    "UPDATE directories
                     SET dir_count = MAX(0, dir_count - 1)
                     WHERE session_id = ?1 AND path = ?2",
                    params![sid, parent_dir],
                )?;
            }

            // Walk upward from each deleted file's parent_path through surviving ancestor directories.
            for (mut cur_parent, cnt, bytes) in parent_buckets {
                let mut hops = 0usize;
                let mut prev = String::new();
                while hops < 128 && cur_parent != prev {
                    tx.execute(
                        "UPDATE directories
                         SET file_count = MAX(0, file_count - ?1),
                             total_size_bytes = MAX(0, total_size_bytes - ?2)
                         WHERE session_id = ?3 AND path = ?4",
                        params![cnt, bytes, sid, &cur_parent],
                    )?;

                    let next_parent: Option<String> = tx
                        .query_row(
                            "SELECT parent_path FROM directories WHERE session_id = ?1 AND path = ?2",
                            params![sid, &cur_parent],
                            |row| row.get(0),
                        )
                        .ok()
                        .flatten();

                    prev = cur_parent;
                    match next_parent {
                        Some(p) if !p.is_empty() => cur_parent = p,
                        _ => break,
                    }
                    hops += 1;
                }
            }
        }

        let _ = escaped_path;
        tx.commit()?;
        Ok(())
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

        let base_count = "SELECT count(*) FROM cleanup_audit_events";
        let base_select = "SELECT id, batch_id, session_id, path, name, size_bytes, category, risk_level,
                                   cleanup_method, status, is_reversible, error_message, created_at
                            FROM cleanup_audit_events";

        let (total_items, items) = match (&query.session_id, &query.status) {
            (Some(sid), Some(status)) => {
                let c_sql = format!("{base_count} WHERE session_id = ?1 AND status = ?2");
                let q_sql = format!("{base_select} WHERE session_id = ?1 AND status = ?2 ORDER BY created_at DESC LIMIT ?3 OFFSET ?4");
                let total: usize = conn.query_row(&c_sql, rusqlite::params![sid, status], |row| row.get(0)).unwrap_or(0);
                let mut stmt = conn.prepare(&q_sql)?;
                let rows = stmt.query_map(rusqlite::params![sid, status, page_size, offset], |row| {
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
                let list: std::result::Result<Vec<_>, _> = rows.collect();
                (total, list?)
            }
            (Some(sid), None) => {
                let c_sql = format!("{base_count} WHERE session_id = ?1");
                let q_sql = format!("{base_select} WHERE session_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3");
                let total: usize = conn.query_row(&c_sql, rusqlite::params![sid], |row| row.get(0)).unwrap_or(0);
                let mut stmt = conn.prepare(&q_sql)?;
                let rows = stmt.query_map(rusqlite::params![sid, page_size, offset], |row| {
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
                let list: std::result::Result<Vec<_>, _> = rows.collect();
                (total, list?)
            }
            (None, Some(status)) => {
                let c_sql = format!("{base_count} WHERE status = ?1");
                let q_sql = format!("{base_select} WHERE status = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3");
                let total: usize = conn.query_row(&c_sql, rusqlite::params![status], |row| row.get(0)).unwrap_or(0);
                let mut stmt = conn.prepare(&q_sql)?;
                let rows = stmt.query_map(rusqlite::params![status, page_size, offset], |row| {
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
                let list: std::result::Result<Vec<_>, _> = rows.collect();
                (total, list?)
            }
            (None, None) => {
                let c_sql = base_count;
                let q_sql = format!("{base_select} ORDER BY created_at DESC LIMIT ?1 OFFSET ?2");
                let total: usize = conn.query_row(c_sql, [], |row| row.get(0)).unwrap_or(0);
                let mut stmt = conn.prepare(&q_sql)?;
                let rows = stmt.query_map(rusqlite::params![page_size, offset], |row| {
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
                let list: std::result::Result<Vec<_>, _> = rows.collect();
                (total, list?)
            }
        };

        let total_pages = ((total_items as f64) / (page_size as f64)).ceil() as u32;

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
        let plan = CleanupService::get_cleanup_plan(&conn, Some(session_id)).unwrap();
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

    #[test]
    fn test_cleanup_updates_session_directory_and_summary_rollups() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrator::run(&mut conn).unwrap();

        let temp_dir = std::env::temp_dir().join(format!("diskatlas_rollup_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();
        let target_file = temp_dir.join("old_100%_archive.zip");
        {
            let mut f = File::create(&target_file).unwrap();
            f.write_all(b"rollup test payload").unwrap();
        }
        let f_size = fs::metadata(&target_file).unwrap().len() as i64;
        let temp_str = temp_dir.to_string_lossy().to_string();
        let file_str = target_file.to_string_lossy().to_string();

        conn.execute(
            "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
             VALUES ('D_ROLLUP', 'Test', '/tmp', 'apfs', 10000, 5000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO scan_sessions (id, drive_id, root_path, status, total_files, total_directories, total_bytes, started_at)
             VALUES ('sess-rollup', 'D_ROLLUP', ?1, 'completed', 2, 1, ?2, CURRENT_TIMESTAMP)",
            params![&temp_str, f_size + 50],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO directories (session_id, path, name, parent_path, total_size_bytes, file_count, dir_count)
             VALUES ('sess-rollup', ?1, 'temp', NULL, ?2, 2, 0)",
            params![&temp_str, f_size + 50],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (session_id, path, name, extension, parent_path, size_bytes, category, confidence)
             VALUES ('sess-rollup', ?1, 'old_100%_archive.zip', 'zip', ?2, ?3, 'archives', 0.9),
                    ('sess-rollup', ?4, 'keep.txt', 'txt', ?2, 50, 'documents', 0.9)",
            params![&file_str, &temp_str, f_size, format!("{}/keep.txt", temp_str)],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO category_summaries (session_id, category, file_count, total_bytes)
             VALUES ('sess-rollup', 'archives', 1, ?1), ('sess-rollup', 'documents', 1, 50)",
            [f_size],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO file_hashes (path, size_bytes, modified_at, sha256)
             VALUES (?1, ?2, 0, 'deadbeef')",
            params![&file_str, f_size],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO storage_recommendations
             (id, session_id, path, name, size_bytes, category, rec_group, risk_level,
              reason, confidence, recoverable_bytes, expected_consequences, recommended_action)
             VALUES ('rec-rollup-1', 'sess-rollup', ?1, 'old_100%_archive.zip', ?2, 'archives',
                     'safe_to_review', 'low', 'Old archive', 0.95, ?2, 'Reclaimable', 'Delete')",
            params![&file_str, f_size],
        )
        .unwrap();

        let res = CleanupService::execute_cleanup(
            &conn,
            ExecuteCleanupRequest {
                batch_id: "batch-rollup".to_string(),
                session_id: "sess-rollup".to_string(),
                item_ids: vec!["rec-rollup-1".to_string()],
                use_recycle_bin: false,
                confirmation_phrase: "CONFIRM_CLEANUP".to_string(),
            },
        )
        .unwrap();
        assert_eq!(res.successful_count, 1);

        let (rem_files, rem_bytes): (i64, i64) = conn
            .query_row(
                "SELECT total_files, total_bytes FROM scan_sessions WHERE id = 'sess-rollup'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(rem_files, 1);
        assert_eq!(rem_bytes, 50);

        let (dir_files, dir_bytes): (i64, i64) = conn
            .query_row(
                "SELECT file_count, total_size_bytes FROM directories WHERE session_id = 'sess-rollup' AND path = ?1",
                [&temp_str],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(dir_files, 1);
        assert_eq!(dir_bytes, 50);

        let arch_cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM category_summaries WHERE session_id = 'sess-rollup' AND category = 'archives'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(arch_cnt, 0);

        let hash_cnt: i64 = conn
            .query_row("SELECT COUNT(*) FROM file_hashes WHERE path = ?1", [&file_str], |r| r.get(0))
            .unwrap();
        assert_eq!(hash_cnt, 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
