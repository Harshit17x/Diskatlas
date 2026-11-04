use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use rusqlite::{params, Connection};
use tracing::info;
use uuid::Uuid;

use crate::error::Result;
use crate::models::intelligence::*;

pub struct IntelligenceService;

impl IntelligenceService {
    /// Generates full Storage Intelligence profile: findings, consumers, and evidence-backed recommendations
    pub fn generate_intelligence_profile(
        conn: &Connection,
        session_id: Option<&str>,
    ) -> Result<StorageIntelligenceProfile> {
        let sid = Self::resolve_session_id(conn, session_id)?;

        // Fetch session metrics and drive ID
        let (drive_id, total_scanned_bytes): (String, i64) = conn.query_row(
            "SELECT drive_id, COALESCE(total_bytes, 0) FROM scan_sessions WHERE id = ?1",
            [&sid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        info!(session_id = %sid, "Generating Storage Intelligence recommendations and findings");

        // Clear existing generated recommendations for this session to ensure clean regeneration
        conn.execute(
            "DELETE FROM storage_recommendations WHERE session_id = ?1",
            [&sid],
        )?;

        let mut recommendations: Vec<StorageRecommendation> = Vec::new();
        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let ninety_days_ago = now_epoch - (90 * 86400);

        // -------------------------------------------------------------
        // RULE 1: Standalone Installers & OS Images (Safe to Review)
        // -------------------------------------------------------------
        {
            let mut stmt = conn.prepare(
                "SELECT id, path, name, size_bytes, modified_at, category
                 FROM files
                 WHERE session_id = ?1 AND size_bytes >= 20971520 -- >= 20 MB
                   AND (
                       LOWER(extension) IN ('exe', 'msi', 'dmg', 'pkg', 'iso', 'deb', 'rpm')
                       OR path LIKE '%Downloads%'
                   )
                   AND is_system = 0
                 ORDER BY size_bytes DESC
                 LIMIT 50"
            )?;

            let rows = stmt.query_map([&sid], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })?;

            for r in rows {
                let (_, path, name, size_bytes, modified_at, category) = r?;
                let mod_time = modified_at.unwrap_or(0);
                let is_old = mod_time < ninety_days_ago;
                let cat = category.unwrap_or_else(|| "downloads".to_string());

                let reason = if is_old {
                    format!("Standalone software installer untouched for over 90 days ({}).", name)
                } else {
                    format!("Large software package or disk image in downloads ({}).", name)
                };

                recommendations.push(StorageRecommendation {
                    id: format!("rec-inst-{}", Uuid::new_v4()),
                    session_id: sid.clone(),
                    path: path.clone(),
                    name: name.clone(),
                    size_bytes,
                    category: cat,
                    group: RecommendationGroup::SafeToReview,
                    risk_level: RiskLevel::Low,
                    reason,
                    confidence: 0.95,
                    recoverable_bytes: size_bytes,
                    expected_consequences: "Removing installer packages does not uninstall or harm currently installed software.".to_string(),
                    recommended_action: "Verify the corresponding software is installed or no longer needed, then remove the setup file.".to_string(),
                    created_at: chrono_now_iso(),
                });
            }
        }

        // -------------------------------------------------------------
        // RULE 2: Cryptographic Duplicate Secondary Copies (Safe to Review)
        // -------------------------------------------------------------
        {
            // Find duplicate files from file_hashes table
            let mut stmt = conn.prepare(
                "SELECT fh.path, f.name, f.size_bytes, f.category, fh.sha256
                 FROM file_hashes fh
                 JOIN files f ON f.path = fh.path AND f.session_id = ?1
                 WHERE fh.sha256 IN (
                     SELECT sha256 FROM file_hashes GROUP BY sha256 HAVING COUNT(*) > 1
                 )
                 ORDER BY fh.sha256, f.size_bytes DESC"
            )?;

            let dup_rows = stmt.query_map([&sid], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

            let mut hash_groups: HashMap<String, Vec<(String, String, i64, String)>> = HashMap::new();
            for r in dup_rows {
                let (path, name, size_bytes, cat, sha) = r?;
                let category = cat.unwrap_or_else(|| "duplicates".to_string());
                hash_groups.entry(sha).or_default().push((path, name, size_bytes, category));
            }

            for (_, mut copies) in hash_groups {
                if copies.len() >= 2 {
                    // Keep the first file as the canonical original (0 recoverable)
                    // Every additional copy is a safe reclaimable recommendation
                    copies.remove(0);

                    for (path, name, size_bytes, category) in copies {
                        recommendations.push(StorageRecommendation {
                            id: format!("rec-dup-{}", Uuid::new_v4()),
                            session_id: sid.clone(),
                            path,
                            name: name.clone(),
                            size_bytes,
                            category,
                            group: RecommendationGroup::SafeToReview,
                            risk_level: RiskLevel::Low,
                            reason: format!("Exact binary SHA-256 duplicate copy of '{}'.", name),
                            confidence: 1.0,
                            recoverable_bytes: size_bytes,
                            expected_consequences: "The primary file copy is retained; removing this redundant duplicate frees space with zero data loss.".to_string(),
                            recommended_action: "Remove redundant duplicate copy to reclaim space.".to_string(),
                            created_at: chrono_now_iso(),
                        });
                    }
                }
            }
        }

        // -------------------------------------------------------------
        // RULE 3: Developer Build Outputs & Dependencies
        // -------------------------------------------------------------
        {
            let mut stmt = conn.prepare(
                "SELECT path, name, total_size_bytes, modified_at
                 FROM directories
                 WHERE session_id = ?1 AND (
                     name IN ('target', 'node_modules', 'dist', 'build', '.output', 'cmake-build-debug', 'cmake-build-release')
                     OR path LIKE '%/target' OR path LIKE '%\\target'
                     OR path LIKE '%/node_modules' OR path LIKE '%\\node_modules'
                 ) AND total_size_bytes >= 52428800 -- >= 50 MB
                 ORDER BY total_size_bytes DESC
                 LIMIT 40"
            )?;

            let dev_rows = stmt.query_map([&sid], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            })?;

            for r in dev_rows {
                let (path, name, size_bytes, modified_at) = r?;
                let mod_time = modified_at.unwrap_or(0);
                let is_old = mod_time > 0 && mod_time < ninety_days_ago;

                let is_build_output = matches!(
                    name.as_str(),
                    "target" | "dist" | "build" | ".output" | "cmake-build-debug" | "cmake-build-release"
                );

                if is_build_output {
                    recommendations.push(StorageRecommendation {
                        id: format!("rec-dev-build-{}", Uuid::new_v4()),
                        session_id: sid.clone(),
                        path: path.clone(),
                        name: name.clone(),
                        size_bytes,
                        category: "developer".to_string(),
                        group: RecommendationGroup::SafeToReview,
                        risk_level: RiskLevel::Low,
                        reason: format!("Compiled project artifact directory ('{}') consuming significant disk space.", name),
                        confidence: 0.95,
                        recoverable_bytes: size_bytes,
                        expected_consequences: "Build artifacts can be regenerated from repository source code at any time via your compiler or build tool.".to_string(),
                        recommended_action: "Run 'cargo clean', 'npm run build', or delete the compilation directory to reclaim space.".to_string(),
                        created_at: chrono_now_iso(),
                    });
                } else if name == "node_modules" {
                    let group = if is_old {
                        RecommendationGroup::SafeToReview
                    } else {
                        RecommendationGroup::ReviewRequired
                    };

                    recommendations.push(StorageRecommendation {
                        id: format!("rec-dev-pkg-{}", Uuid::new_v4()),
                        session_id: sid.clone(),
                        path: path.clone(),
                        name: name.clone(),
                        size_bytes,
                        category: "developer".to_string(),
                        group,
                        risk_level: RiskLevel::Moderate,
                        reason: if is_old {
                            "Third-party package tree ('node_modules') in an inactive repository unmodified for over 90 days.".to_string()
                        } else {
                            "Substantial third-party dependency directory ('node_modules').".to_string()
                        },
                        confidence: 0.90,
                        recoverable_bytes: size_bytes,
                        expected_consequences: "Project requires executing 'npm install' or 'pnpm install' before next run.".to_string(),
                        recommended_action: "If the project is currently in cold storage, delete dependencies; otherwise preserve.".to_string(),
                        created_at: chrono_now_iso(),
                    });
                }
            }
        }

        // -------------------------------------------------------------
        // RULE 4: Operating System Protected Volumes & Files (System Protected)
        // -------------------------------------------------------------
        {
            let mut stmt = conn.prepare(
                "SELECT path, name, size_bytes, category
                 FROM files
                 WHERE session_id = ?1 AND (
                     is_system = 1
                     OR path LIKE '%Windows\\System32%'
                     OR path LIKE '%$Recycle.Bin%'
                     OR path LIKE '%System Volume Information%'
                     OR path LIKE '/System/%'
                     OR path LIKE '/private/%'
                     OR name IN ('pagefile.sys', 'hiberfil.sys', 'swapfile.sys')
                 ) AND size_bytes >= 10485760 -- >= 10 MB
                 ORDER BY size_bytes DESC
                 LIMIT 30"
            )?;

            let sys_rows = stmt.query_map([&sid], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?;

            for r in sys_rows {
                let (path, name, size_bytes, category) = r?;
                let cat = category.unwrap_or_else(|| "os".to_string());

                recommendations.push(StorageRecommendation {
                    id: format!("rec-sys-{}", Uuid::new_v4()),
                    session_id: sid.clone(),
                    path,
                    name: name.clone(),
                    size_bytes,
                    category: cat,
                    group: RecommendationGroup::SystemProtected,
                    risk_level: RiskLevel::Critical,
                    reason: format!("Operating system component or kernel memory structure ('{}').", name),
                    confidence: 1.0,
                    recoverable_bytes: 0, // 0 recoverable because user tools must NOT touch system files
                    expected_consequences: "Direct modification or deletion causes immediate operating system instability, BSOD, or boot corruption.".to_string(),
                    recommended_action: "Protected by system kernel. Non-destructive inspection only. Never modify or delete.".to_string(),
                    created_at: chrono_now_iso(),
                });
            }
        }

        // -------------------------------------------------------------
        // RULE 5: Large Cold Media Video Archives (Move or Archive)
        // -------------------------------------------------------------
        {
            let mut stmt = conn.prepare(
                "SELECT path, name, size_bytes, category, modified_at
                 FROM files
                 WHERE session_id = ?1 AND size_bytes >= 1073741824 -- >= 1 GB
                   AND LOWER(extension) IN ('mp4', 'mkv', 'mov', 'avi', 'wmv', 'm4v', 'flv')
                   AND is_system = 0
                 ORDER BY size_bytes DESC
                 LIMIT 25"
            )?;

            let media_rows = stmt.query_map([&sid], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                ))
            })?;

            for r in media_rows {
                let (path, name, size_bytes, category, modified_at) = r?;
                let mod_time = modified_at.unwrap_or(0);
                let is_old = mod_time > 0 && mod_time < ninety_days_ago;

                recommendations.push(StorageRecommendation {
                    id: format!("rec-media-{}", Uuid::new_v4()),
                    session_id: sid.clone(),
                    path,
                    name: name.clone(),
                    size_bytes,
                    category: category.unwrap_or_else(|| "videos".to_string()),
                    group: RecommendationGroup::MoveOrArchive,
                    risk_level: RiskLevel::Low,
                    reason: format!(
                        "Very large high-definition video archive ({}) occupying primary SSD space {}.",
                        name,
                        if is_old { "without recent modifications" } else { "" }
                    ),
                    confidence: 0.85,
                    recoverable_bytes: size_bytes,
                    expected_consequences: "Media content is preserved but offloaded from high-speed local drive.".to_string(),
                    recommended_action: "Migrate cold video archive to external storage, NAS, or archival storage.".to_string(),
                    created_at: chrono_now_iso(),
                });
            }
        }

        // -------------------------------------------------------------
        // RULE 6: Installed Applications (Uninstall Official)
        // -------------------------------------------------------------
        {
            let mut stmt = conn.prepare(
                "SELECT path, name, size_bytes, app_name
                 FROM files
                 WHERE session_id = ?1 AND category = 'applications'
                   AND size_bytes >= 209715200 -- >= 200 MB
                   AND is_system = 0
                 ORDER BY size_bytes DESC
                 LIMIT 20"
            )?;

            let app_rows = stmt.query_map([&sid], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?;

            for r in app_rows {
                let (path, name, size_bytes, app_name) = r?;
                let target_app = app_name.unwrap_or_else(|| name.clone());

                recommendations.push(StorageRecommendation {
                    id: format!("rec-app-{}", Uuid::new_v4()),
                    session_id: sid.clone(),
                    path,
                    name: name.clone(),
                    size_bytes,
                    category: "applications".to_string(),
                    group: RecommendationGroup::UninstallOfficial,
                    risk_level: RiskLevel::High,
                    reason: format!("Installed application package executable ('{}').", target_app),
                    confidence: 0.95,
                    recoverable_bytes: size_bytes,
                    expected_consequences: "Raw deletion of application directories leaves dangling registry entries and broken system integrations.".to_string(),
                    recommended_action: "Uninstall using official operating system uninstaller (Settings > Apps or vendor uninstaller).".to_string(),
                    created_at: chrono_now_iso(),
                });
            }
        }

        // -------------------------------------------------------------
        // Persist all recommendations in batch into SQLite
        // -------------------------------------------------------------
        {
            let mut insert_stmt = conn.prepare(
                "INSERT INTO storage_recommendations (
                    id, session_id, path, name, size_bytes, category, rec_group,
                    risk_level, reason, confidence, recoverable_bytes,
                    expected_consequences, recommended_action
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"
            )?;

            for r in &recommendations {
                insert_stmt.execute(params![
                    &r.id,
                    &r.session_id,
                    &r.path,
                    &r.name,
                    r.size_bytes,
                    &r.category,
                    r.group.as_str(),
                    r.risk_level.as_str(),
                    &r.reason,
                    r.confidence,
                    r.recoverable_bytes,
                    &r.expected_consequences,
                    &r.recommended_action,
                ])?;
            }
        }

        // -------------------------------------------------------------
        // Calculate Totals and Group Summaries
        // -------------------------------------------------------------
        let mut potential_reclaimable_bytes = 0i64;
        let mut safe_reclaimable_bytes = 0i64;
        let mut review_required_bytes = 0i64;
        let mut system_protected_bytes = 0i64;
        let mut group_summaries: HashMap<String, GroupSummaryItem> = HashMap::new();

        for r in &recommendations {
            let entry = group_summaries.entry(r.group.as_str().to_string()).or_insert_with(|| {
                GroupSummaryItem {
                    group: r.group.as_str().to_string(),
                    count: 0,
                    total_bytes: 0,
                    recoverable_bytes: 0,
                }
            });

            entry.count += 1;
            entry.total_bytes += r.size_bytes;
            entry.recoverable_bytes += r.recoverable_bytes;

            match r.group {
                RecommendationGroup::SafeToReview => {
                    safe_reclaimable_bytes += r.recoverable_bytes;
                    potential_reclaimable_bytes += r.recoverable_bytes;
                }
                RecommendationGroup::ReviewRequired => {
                    review_required_bytes += r.recoverable_bytes;
                    potential_reclaimable_bytes += r.recoverable_bytes;
                }
                RecommendationGroup::MoveOrArchive => {
                    potential_reclaimable_bytes += r.recoverable_bytes;
                }
                RecommendationGroup::SystemProtected => {
                    system_protected_bytes += r.size_bytes;
                }
                RecommendationGroup::UninstallOfficial => {
                    // Application uninstallation is handled through external uninstaller
                }
            }
        }

        // -------------------------------------------------------------
        // Synthesize Macro Findings
        // -------------------------------------------------------------
        let mut findings: Vec<StorageFinding> = Vec::new();

        if let Some(dup_summary) = group_summaries.get("safe_to_review") {
            if dup_summary.recoverable_bytes > 0 {
                findings.push(StorageFinding {
                    id: "finding-safe-reclaim".to_string(),
                    title: "Immediate Reclaimable Storage Identified".to_string(),
                    description: format!(
                        "Discovered {} low-risk candidates (stale installers, compilation outputs, exact duplicate copies) taking {}.",
                        dup_summary.count,
                        format_bytes_internal(dup_summary.recoverable_bytes)
                    ),
                    severity: "info".to_string(),
                    impact_bytes: dup_summary.recoverable_bytes,
                    affected_items_count: dup_summary.count,
                    finding_type: "safe_reclaim".to_string(),
                });
            }
        }

        if let Some(sys_summary) = group_summaries.get("system_protected") {
            findings.push(StorageFinding {
                id: "finding-system-protected".to_string(),
                title: "Protected Operating System Footprint".to_string(),
                description: format!(
                    "Operating system kernel, pagefiles, and recovery stores consume {} across {} indexed items. Strictly quarantined.",
                    format_bytes_internal(sys_summary.total_bytes),
                    sys_summary.count
                ),
                severity: "advisory".to_string(),
                impact_bytes: sys_summary.total_bytes,
                affected_items_count: sys_summary.count,
                finding_type: "system_protected".to_string(),
            });
        }

        if let Some(media_summary) = group_summaries.get("move_or_archive") {
            if media_summary.total_bytes > 0 {
                findings.push(StorageFinding {
                    id: "finding-cold-media".to_string(),
                    title: "Cold Media & High-Definition Video Footprint".to_string(),
                    description: format!(
                        "Large video files occupying {} on primary storage. Candidates for offloading to external media.",
                        format_bytes_internal(media_summary.total_bytes)
                    ),
                    severity: "advisory".to_string(),
                    impact_bytes: media_summary.total_bytes,
                    affected_items_count: media_summary.count,
                    finding_type: "cold_media".to_string(),
                });
            }
        }

        // -------------------------------------------------------------
        // Query Top Storage Consumers with Explanations
        // -------------------------------------------------------------
        let mut largest_consumers: Vec<StorageConsumer> = Vec::new();
        {
            let mut stmt = conn.prepare(
                "SELECT path, name, size_bytes, category, is_system
                 FROM files
                 WHERE session_id = ?1
                 ORDER BY size_bytes DESC
                 LIMIT 15"
            )?;

            let consumer_rows = stmt.query_map([&sid], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)? != 0,
                ))
            })?;

            for r in consumer_rows {
                let (path, name, size_bytes, category, is_system) = r?;
                let cat = category.unwrap_or_else(|| "unknown".to_string());
                let percentage = if total_scanned_bytes > 0 {
                    (size_bytes as f64 / total_scanned_bytes as f64) * 100.0
                } else {
                    0.0
                };

                let explanation = if is_system {
                    "Critical operating system protected component.".to_string()
                } else if cat == "developer" {
                    "Developer tool or compilation output package.".to_string()
                } else if cat == "videos" {
                    "High-definition video recording or raw media.".to_string()
                } else if cat == "downloads" {
                    "Downloaded distribution image or setup installer.".to_string()
                } else if cat == "vm_emulators" {
                    "Virtual machine hard disk image or container snapshot.".to_string()
                } else {
                    format!("Large {} storage consumer on volume.", cat)
                };

                largest_consumers.push(StorageConsumer {
                    path,
                    name,
                    size_bytes,
                    category: cat,
                    percentage_of_used: percentage,
                    is_directory: false,
                    explanation,
                });
            }
        }

        Ok(StorageIntelligenceProfile {
            session_id: sid,
            drive_id,
            total_scanned_bytes,
            potential_reclaimable_bytes,
            safe_reclaimable_bytes,
            review_required_bytes,
            system_protected_bytes,
            findings,
            largest_consumers,
            group_summaries,
        })
    }

    /// Queries filtered and paginated recommendations from SQLite
    pub fn get_recommendations(
        conn: &Connection,
        query: RecommendationsQuery,
    ) -> Result<RecommendationsResponse> {
        let sid = Self::resolve_session_id(conn, query.session_id.as_deref())?;

        let page = query.page.max(1);
        let page_size = query.page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;

        let mut conditions = vec!["session_id = ?1".to_string()];
        let mut param_index = 2;
        let mut params_vec: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Text(sid.clone())];

        if let Some(grp) = &query.group {
            if grp != "all" && !grp.is_empty() {
                conditions.push(format!("rec_group = ?{}", param_index));
                params_vec.push(rusqlite::types::Value::Text(grp.clone()));
                param_index += 1;
            }
        }

        if let Some(risk) = &query.risk_level {
            if risk != "all" && !risk.is_empty() {
                conditions.push(format!("risk_level = ?{}", param_index));
                params_vec.push(rusqlite::types::Value::Text(risk.clone()));
                param_index += 1;
            }
        }

        if let Some(search) = &query.search {
            let term = search.trim();
            if !term.is_empty() {
                conditions.push(format!("(name LIKE ?{0} OR path LIKE ?{0} OR reason LIKE ?{0})", param_index));
                params_vec.push(rusqlite::types::Value::Text(format!("%{}%", term)));
            }
        }

        let where_clause = conditions.join(" AND ");

        // Total count query
        let count_sql = format!("SELECT COUNT(*) FROM storage_recommendations WHERE {}", where_clause);
        let mut count_stmt = conn.prepare(&count_sql)?;
        let total_items: i64 = count_stmt.query_row(
            rusqlite::params_from_iter(params_vec.iter()),
            |row| row.get(0),
        )?;

        // Paginated item query
        let query_sql = format!(
            "SELECT id, session_id, path, name, size_bytes, category, rec_group,
                    risk_level, reason, confidence, recoverable_bytes, expected_consequences,
                    recommended_action, created_at
             FROM storage_recommendations
             WHERE {}
             ORDER BY recoverable_bytes DESC, size_bytes DESC
             LIMIT {} OFFSET {}",
            where_clause, page_size, offset
        );

        let mut stmt = conn.prepare(&query_sql)?;
        let items_iter = stmt.query_map(
            rusqlite::params_from_iter(params_vec.iter()),
            |row| {
                let group_str: String = row.get(6)?;
                let risk_str: String = row.get(7)?;

                Ok(StorageRecommendation {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    path: row.get(2)?,
                    name: row.get(3)?,
                    size_bytes: row.get(4)?,
                    category: row.get(5)?,
                    group: RecommendationGroup::from_str(&group_str),
                    risk_level: RiskLevel::from_str(&risk_str),
                    reason: row.get(8)?,
                    confidence: row.get(9)?,
                    recoverable_bytes: row.get(10)?,
                    expected_consequences: row.get(11)?,
                    recommended_action: row.get(12)?,
                    created_at: row.get(13)?,
                })
            },
        )?;

        let mut items = Vec::new();
        for item in items_iter {
            items.push(item?);
        }

        let total_pages = if total_items == 0 {
            1
        } else {
            ((total_items as f64) / (page_size as f64)).ceil() as u32
        };

        Ok(RecommendationsResponse {
            items,
            total_items: total_items as usize,
            page,
            page_size,
            total_pages,
        })
    }

    /// Resolves target session id, falling back to latest session
    fn resolve_session_id(conn: &Connection, requested: Option<&str>) -> Result<String> {
        if let Some(id) = requested {
            return Ok(id.to_string());
        }

        let latest: Option<String> = conn
            .query_row(
                "SELECT id FROM scan_sessions ORDER BY started_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        match latest {
            Some(id) => Ok(id),
            None => Err(crate::error::AppError::NotFound(
                "No completed scan sessions found for storage intelligence analysis".to_string(),
            )),
        }
    }
}

fn chrono_now_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}-01-01T00:00:00Z", 1970 + (now / 31536000))
}

fn format_bytes_internal(bytes: i64) -> String {
    if bytes >= 1073741824 {
        format!("{:.2} GB", bytes as f64 / 1073741824.0)
    } else if bytes >= 1048576 {
        format!("{:.1} MB", bytes as f64 / 1048576.0)
    } else {
        format!("{} KB", bytes / 1024)
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
             VALUES ('DRIVE_TEST_INT', 'Test Drive', 'C:\\', 'NTFS', 1000000000000, 500000000000)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO scan_sessions (id, drive_id, root_path, started_at, status, total_bytes)
             VALUES ('sess-intel-1', 'DRIVE_TEST_INT', 'C:\\', CURRENT_TIMESTAMP, 'completed', 50000000000)",
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_intelligence_profile_generation_and_recommendations() {
        let conn = setup_test_db();

        let past_time = 100000;
        conn.execute(
            "INSERT INTO files (session_id, path, name, extension, parent_path, size_bytes, modified_at, category, is_system)
             VALUES
             ('sess-intel-1', 'C:\\Users\\User\\Downloads\\setup_old.exe', 'setup_old.exe', 'exe', 'C:\\Users\\User\\Downloads', 3221225472, ?1, 'downloads', 0),
             ('sess-intel-1', 'C:\\Windows\\System32\\ntoskrnl.exe', 'ntoskrnl.exe', 'exe', 'C:\\Windows\\System32', 52428800, ?1, 'os', 1),
             ('sess-intel-1', 'C:\\Videos\\archive.mp4', 'archive.mp4', 'mp4', 'C:\\Videos', 2147483648, ?1, 'videos', 0),
             ('sess-intel-1', 'C:\\Program Files\\LargeApp\\app.exe', 'app.exe', 'exe', 'C:\\Program Files\\LargeApp', 524288000, ?1, 'applications', 0)",
            params![past_time],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO directories (session_id, path, name, parent_path, total_size_bytes, modified_at)
             VALUES ('sess-intel-1', 'C:\\dev\\project\\target', 'target', 'C:\\dev\\project', 1073741824, ?1)",
            params![past_time],
        )
        .unwrap();

        let profile = IntelligenceService::generate_intelligence_profile(&conn, Some("sess-intel-1")).unwrap();

        // 1. Verify findings exist
        assert!(!profile.findings.is_empty(), "Findings must be generated");

        // 2. Verify largest consumers
        assert!(!profile.largest_consumers.is_empty(), "Largest consumers must be identified");
        assert_eq!(profile.largest_consumers[0].name, "setup_old.exe");

        // 3. Verify recommendations query
        let q = RecommendationsQuery {
            session_id: Some("sess-intel-1".to_string()),
            group: None,
            risk_level: None,
            search: None,
            page: 1,
            page_size: 10,
        };
        let res = IntelligenceService::get_recommendations(&conn, q).unwrap();
        assert!(res.total_items >= 4, "Should have generated recommendations for installer, build, video, system, app");

        // 4. Verify system protected rule
        let sys_rec = res.items.iter().find(|r| r.name == "ntoskrnl.exe").unwrap();
        assert_eq!(sys_rec.group, RecommendationGroup::SystemProtected);
        assert_eq!(sys_rec.risk_level, RiskLevel::Critical);
        assert_eq!(sys_rec.recoverable_bytes, 0, "System files must never claim recoverable bytes");

        // 5. Verify safe installer rule
        let inst_rec = res.items.iter().find(|r| r.name == "setup_old.exe").unwrap();
        assert_eq!(inst_rec.group, RecommendationGroup::SafeToReview);
        assert_eq!(inst_rec.risk_level, RiskLevel::Low);
        assert_eq!(inst_rec.recoverable_bytes, 3221225472);

        // 6. Verify developer build output rule
        let build_rec = res.items.iter().find(|r| r.name == "target").unwrap();
        assert_eq!(build_rec.group, RecommendationGroup::SafeToReview);
    }
}

