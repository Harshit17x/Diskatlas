use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tracing::{debug, info};

use crate::error::Result;
use crate::models::scan::{ScanProgress, ScanStatus, ScanSummary};
use crate::services::scanner::models::{RawDirItem, RawErrorItem, RawFileItem, ScanOptions};
use crate::storage::DbPool;

// State codes for atomic scan status
pub const STATUS_RUNNING: u8 = 0;
pub const STATUS_PAUSED: u8 = 1;
pub const STATUS_CANCELLED: u8 = 2;
pub const STATUS_COMPLETED: u8 = 3;
pub const STATUS_FAILED: u8 = 4;

pub struct ScanEngine;

impl ScanEngine {
    /// Executes the full recursive scan on a background thread.
    pub async fn run_scan(
        options: ScanOptions,
        status: Arc<AtomicU8>,
        pause_notify: Arc<tokio::sync::Notify>,
        progress: Arc<Mutex<ScanProgress>>,
        db_pool: DbPool,
        app_handle: Option<AppHandle>,
    ) -> Result<ScanSummary> {
        let start_time = Instant::now();
        info!(
            "Starting scan session '{}' on path: {}",
            options.session_id,
            options.root_path.display()
        );

        // Ensure drive record exists in drives table to satisfy foreign key constraint
        {
            let conn_arc = db_pool.conn();
            let conn = conn_arc.lock().unwrap();
            let root_str = options.root_path.to_string_lossy().to_string();
            let _ = conn.execute(
                "INSERT OR IGNORE INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
                 VALUES (?1, ?1, ?2, 'UNKNOWN', 0, 0);",
                (&options.drive_id, &root_str),
            );
            conn.execute(
                "INSERT INTO scan_sessions (id, drive_id, root_path, status, started_at)
                 VALUES (?1, ?2, ?3, 'running', CURRENT_TIMESTAMP)
                 ON CONFLICT(id) DO UPDATE SET status = 'running';",
                (&options.session_id, &options.drive_id, &root_str),
            )?;
        }

        // Cycle detection: track visited inodes or canonical paths
        #[cfg(unix)]
        let mut visited_inodes: HashSet<(u64, u64)> = HashSet::new();
        #[cfg(unix)]
        let mut visited_file_inodes: HashSet<(u64, u64)> = HashSet::new();
        #[cfg(not(unix))]
        let mut visited_paths: HashSet<PathBuf> = HashSet::new();

        // Work stack of directories to traverse (avoids call-stack recursion)
        let mut dir_stack: Vec<PathBuf> = vec![options.root_path.clone()];

        // In-memory directory aggregation map for bottom-up recursive sizing
        let mut dir_map: std::collections::HashMap<String, RawDirItem> = std::collections::HashMap::new();

        // Batch buffers for SQLite writes
        let mut file_batch: Vec<RawFileItem> = Vec::with_capacity(options.batch_size);
        let mut dir_batch: Vec<RawDirItem> = Vec::with_capacity(options.batch_size);
        let mut error_batch: Vec<RawErrorItem> = Vec::with_capacity(100);

        let mut files_scanned: u64 = 0;
        let mut dirs_scanned: u64 = 0;
        let mut bytes_scanned: u64 = 0;
        let mut error_count: u64 = 0;
        let mut last_emit = Instant::now();
        let mut loop_counter: usize = 0;

        // Immediately emit initial progress event so UI receives active scanning feedback
        if let Some(ref handle) = app_handle {
            let initial = ScanProgress {
                session_id: options.session_id.clone(),
                drive_id: options.drive_id.clone(),
                root_path: options.root_path.to_string_lossy().to_string(),
                status: ScanStatus::Running,
                files_scanned: 0,
                dirs_scanned: 0,
                bytes_scanned: 0,
                error_count: 0,
                current_path: options.root_path.to_string_lossy().to_string(),
                elapsed_millis: 0,
            };
            handle.emit("scan://progress", &initial).ok();
        }

        while let Some(current_dir) = dir_stack.pop() {
            loop_counter += 1;
            // Cooperative yielding to prevent monopolizing the Tokio executor and starving UI queries
            if loop_counter % 64 == 0 {
                tokio::task::yield_now().await;
            }

            // Check cancellation
            if status.load(Ordering::SeqCst) == STATUS_CANCELLED {
                info!("Scan session '{}' cancelled by user", options.session_id);
                // Perform bottom-up rollup on whatever was scanned so far before flushing
                Self::rollup_and_flush_directories(&db_pool, &options.session_id, &mut dir_map, options.batch_size)?;
                Self::flush_batches(&db_pool, &options.session_id, &mut file_batch, &mut dir_batch, &mut error_batch)?;
                Self::update_session_status(&db_pool, &options.session_id, ScanStatus::Cancelled, files_scanned, dirs_scanned, bytes_scanned, error_count)?;
                return Ok(ScanSummary {
                    session_id: options.session_id,
                    drive_id: options.drive_id,
                    root_path: options.root_path.to_string_lossy().to_string(),
                    status: ScanStatus::Cancelled,
                    total_files: files_scanned,
                    total_dirs: dirs_scanned,
                    total_bytes: bytes_scanned,
                    error_count,
                    duration_millis: start_time.elapsed().as_millis() as u64,
                });
            }

            // Check pause
            while status.load(Ordering::SeqCst) == STATUS_PAUSED {
                debug!("Scan session '{}' paused, waiting for resume notification...", options.session_id);
                pause_notify.notified().await;
            }

            // Check system internal boundary exclusions (e.g. /System/Volumes firmlinks, /Volumes external mounts, /dev)
            if Self::is_system_internal_exclusion(&current_dir, &options.root_path) {
                debug!("Skipping system internal directory: {}", current_dir.display());
                continue;
            }

            // Check user-configured exclusions
            let current_dir_str = current_dir.to_string_lossy();
            if Self::is_excluded(&current_dir_str, &options.exclusions) {
                debug!("Skipping excluded directory: {}", current_dir.display());
                continue;
            }

            // Loop and junction detection
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if let Ok(m) = fs::symlink_metadata(&current_dir) {
                    let key = (m.dev(), m.ino());
                    if !visited_inodes.insert(key) {
                        debug!("Cycle detected on directory '{}', skipping loop", current_dir.display());
                        continue;
                    }
                }
            }
            #[cfg(not(unix))]
            {
                if let Ok(canon) = fs::canonicalize(&current_dir) {
                    if !visited_paths.insert(canon) {
                        debug!("Cycle detected on directory '{}', skipping loop", current_dir.display());
                        continue;
                    }
                }
            }

            // Read directory contents safely (read-only)
            let read_dir_result = fs::read_dir(&current_dir);
            let entries = match read_dir_result {
                Ok(entries) => entries,
                Err(err) => {
                    error_count += 1;
                    let err_type = match err.kind() {
                        std::io::ErrorKind::PermissionDenied => "PermissionDenied",
                        std::io::ErrorKind::NotFound => "NotFound",
                        _ => "IoError",
                    };
                    error_batch.push(RawErrorItem {
                        path: current_dir.to_string_lossy().to_string(),
                        error_type: err_type.to_string(),
                        message: err.to_string(),
                    });
                    // Record permission-denied / inaccessible system paths safely as protected skips
                    debug!("Inaccessible protected path '{}': {}", current_dir.display(), err);
                    continue;
                }
            };

            dirs_scanned += 1;
            let mut dir_file_count: u64 = 0;
            let mut dir_total_bytes: u64 = 0;
            let mut dir_subdirs: u64 = 0;

            for entry_res in entries {
                let entry = match entry_res {
                    Ok(e) => e,
                    Err(err) => {
                        error_count += 1;
                        error_batch.push(RawErrorItem {
                            path: current_dir.to_string_lossy().to_string(),
                            error_type: "EntryReadError".to_string(),
                            message: err.to_string(),
                        });
                        continue;
                    }
                };

                let entry_path = entry.path();
                let entry_name = entry.file_name().to_string_lossy().to_string();

                // Safe symlink_metadata check
                let meta = match fs::symlink_metadata(&entry_path) {
                    Ok(m) => m,
                    Err(err) => {
                        error_count += 1;
                        error_batch.push(RawErrorItem {
                            path: entry_path.to_string_lossy().to_string(),
                            error_type: "MetadataError".to_string(),
                            message: err.to_string(),
                        });
                        continue;
                    }
                };

                let is_symlink = meta.file_type().is_symlink();

                if meta.is_dir() {
                    dir_subdirs += 1;
                    // Do not follow directory symlinks/reparse points to avoid double counting and loops
                    if !is_symlink {
                        if !Self::is_system_internal_exclusion(&entry_path, &options.root_path) {
                            dir_stack.push(entry_path.clone());
                        }
                    }
                } else if meta.is_file() || is_symlink {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt;
                        // For files with multiple hard links, track physical inode so each unique data block is only sized once
                        if meta.nlink() > 1 {
                            let fkey = (meta.dev(), meta.ino());
                            if !visited_file_inodes.insert(fkey) {
                                // Inode already sized; skip multi-counting
                                continue;
                            }
                        }
                    }

                    // Compute allocated size where available
                    let allocated_bytes = Self::get_allocated_bytes(&meta);

                    // For sparse files (e.g. VM images), bound by allocated physical blocks to prevent phantom gigabytes
                    let raw_len = meta.len();
                    let size = if let Some(alloc) = allocated_bytes {
                        if raw_len > 10 * 1024 * 1024 && alloc < raw_len {
                            alloc
                        } else {
                            raw_len
                        }
                    } else {
                        raw_len
                    };

                    files_scanned += 1;
                    bytes_scanned += size;
                    dir_file_count += 1;
                    dir_total_bytes += size;

                    // Extension extraction (normalized lowercase)
                    let extension = entry_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_lowercase());

                    let parent_str = current_dir.to_string_lossy().to_string();
                    let (created_at, modified_at) = Self::get_timestamps(&meta);
                    let (is_hidden, is_system) = Self::get_attributes(&entry_name, &meta, &entry_path);

                    file_batch.push(RawFileItem {
                        path: entry_path.to_string_lossy().to_string(),
                        name: entry_name,
                        extension,
                        parent_path: parent_str,
                        size_bytes: size,
                        allocated_bytes,
                        created_at,
                        modified_at,
                        is_hidden,
                        is_system,
                        is_symlink,
                    });

                    // Flush batch if buffer is full
                    if file_batch.len() >= options.batch_size {
                        let mut empty_dirs = Vec::new();
                        Self::flush_batches(&db_pool, &options.session_id, &mut file_batch, &mut empty_dirs, &mut error_batch)?;
                        tokio::task::yield_now().await;
                    }
                }
            }

            // Record directory aggregation in memory map for bottom-up rollup
            let (dir_created, dir_modified) = match fs::metadata(&current_dir) {
                Ok(m) => Self::get_timestamps(&m),
                Err(_) => (None, None),
            };

            let dir_parent = current_dir.parent().map(|p| p.to_string_lossy().to_string());
            let dir_name = current_dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| current_dir.to_string_lossy().to_string());
            let is_dir_hidden = dir_name.starts_with('.');

            dirs_scanned += 1;
            dir_map.insert(current_dir.to_string_lossy().to_string(), RawDirItem {
                path: current_dir.to_string_lossy().to_string(),
                name: dir_name,
                parent_path: dir_parent,
                file_count: dir_file_count,
                dir_count: dir_subdirs,
                total_size_bytes: dir_total_bytes,
                created_at: dir_created,
                modified_at: dir_modified,
                is_hidden: is_dir_hidden,
                is_system: false,
            });

            // Update in-memory progress and emit throttled event (every 150ms to prevent IPC congestion)
            if last_emit.elapsed() >= Duration::from_millis(150) {
                let current_progress = ScanProgress {
                    session_id: options.session_id.clone(),
                    drive_id: options.drive_id.clone(),
                    root_path: options.root_path.to_string_lossy().to_string(),
                    status: ScanStatus::Running,
                    files_scanned,
                    dirs_scanned,
                    bytes_scanned,
                    error_count,
                    current_path: current_dir.to_string_lossy().to_string(),
                    elapsed_millis: start_time.elapsed().as_millis() as u64,
                };

                {
                    let mut p = progress.lock().unwrap();
                    *p = current_progress.clone();
                }

                if let Some(ref handle) = app_handle {
                    handle.emit("scan://progress", &current_progress).ok();
                }

                last_emit = Instant::now();
            }
        }

        // Final flush of remaining file and error records
        let mut empty_dir_batch = Vec::new();
        Self::flush_batches(&db_pool, &options.session_id, &mut file_batch, &mut empty_dir_batch, &mut error_batch)?;

        // Perform bottom-up directory rollup so parent directories reflect true recursive size
        Self::rollup_and_flush_directories(&db_pool, &options.session_id, &mut dir_map, options.batch_size)?;

        // Determine final completion status
        let final_status = if error_count > 0 {
            ScanStatus::CompletedWithErrors
        } else {
            ScanStatus::Completed
        };

        Self::update_session_status(
            &db_pool,
            &options.session_id,
            final_status,
            files_scanned,
            dirs_scanned,
            bytes_scanned,
            error_count,
        )?;

        // Automated post-scan intelligence pipeline:
        // 1. Storage Classification Engine
        info!("Running automated post-scan classification for session '{}'", options.session_id);
        let _ = crate::services::classifier::ClassifierService::classify_and_persist_session(&db_pool, &options.session_id);

        // 2. Developer Storage Intelligence
        info!("Running automated developer storage discovery for session '{}'", options.session_id);
        let _ = crate::services::developer::DeveloperService::analyze_developer_storage(&db_pool, &options.session_id);

        // 3. Storage Intelligence Recommendations
        info!("Generating storage intelligence recommendations for session '{}'", options.session_id);
        {
            let conn_arc = db_pool.conn();
            let conn = conn_arc.lock().unwrap();
            let _ = crate::services::intelligence::IntelligenceService::generate_intelligence_profile(&conn, Some(&options.session_id));
        }

        let final_summary = ScanSummary {
            session_id: options.session_id.clone(),
            drive_id: options.drive_id,
            root_path: options.root_path.to_string_lossy().to_string(),
            status: final_status,
            total_files: files_scanned,
            total_dirs: dirs_scanned,
            total_bytes: bytes_scanned,
            error_count,
            duration_millis: start_time.elapsed().as_millis() as u64,
        };

        // Update progress state to completed
        {
            let mut p = progress.lock().unwrap();
            p.status = final_status;
            p.files_scanned = files_scanned;
            p.dirs_scanned = dirs_scanned;
            p.bytes_scanned = bytes_scanned;
            p.error_count = error_count;
            p.elapsed_millis = final_summary.duration_millis;
        }

        if let Some(ref handle) = app_handle {
            handle.emit("scan://completed", &final_summary).ok();
        }

        info!(
            "Scan session '{}' finished with status {:?}. Files: {}, Dirs: {}, Size: {}, Errors: {}",
            options.session_id, final_status, files_scanned, dirs_scanned, bytes_scanned, error_count
        );

        Ok(final_summary)
    }

    /// Flushes accumulated file, directory, and error buffers to SQLite inside a single transaction.
    fn flush_batches(
        db_pool: &DbPool,
        session_id: &str,
        file_batch: &mut Vec<RawFileItem>,
        dir_batch: &mut Vec<RawDirItem>,
        error_batch: &mut Vec<RawErrorItem>,
    ) -> Result<()> {
        if file_batch.is_empty() && dir_batch.is_empty() && error_batch.is_empty() {
            return Ok(());
        }

        let conn_arc = db_pool.conn();
        let mut conn = conn_arc.lock().unwrap();
        let tx = conn.transaction()?;

        // 1. Files
        if !file_batch.is_empty() {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO files (
                    session_id, path, name, extension, parent_path, size_bytes,
                    allocated_bytes, created_at, modified_at, is_hidden, is_system, is_symlink
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
            )?;

            for f in file_batch.drain(..) {
                stmt.execute((
                    session_id,
                    &f.path,
                    &f.name,
                    &f.extension,
                    &f.parent_path,
                    f.size_bytes as i64,
                    f.allocated_bytes.map(|b| b as i64),
                    f.created_at,
                    f.modified_at,
                    f.is_hidden as i32,
                    f.is_system as i32,
                    f.is_symlink as i32,
                ))?;
            }
        }

        // 2. Directories
        if !dir_batch.is_empty() {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO directories (
                    session_id, path, name, parent_path, file_count, dir_count,
                    total_size_bytes, created_at, modified_at, is_hidden, is_system
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
            )?;

            for d in dir_batch.drain(..) {
                stmt.execute((
                    session_id,
                    &d.path,
                    &d.name,
                    &d.parent_path,
                    d.file_count as i64,
                    d.dir_count as i64,
                    d.total_size_bytes as i64,
                    d.created_at,
                    d.modified_at,
                    d.is_hidden as i32,
                    d.is_system as i32,
                ))?;
            }
        }

        // 3. Errors
        if !error_batch.is_empty() {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO scan_errors (session_id, path, error_type, message)
                 VALUES (?1, ?2, ?3, ?4)"
            )?;

            for e in error_batch.drain(..) {
                stmt.execute((session_id, &e.path, &e.error_type, &e.message))?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    fn update_session_status(
        db_pool: &DbPool,
        session_id: &str,
        status: ScanStatus,
        total_files: u64,
        total_dirs: u64,
        total_bytes: u64,
        error_count: u64,
    ) -> Result<()> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();
        conn.execute(
            "UPDATE scan_sessions SET
                status = ?1,
                total_files = ?2,
                total_directories = ?3,
                total_bytes = ?4,
                error_count = ?5,
                completed_at = CURRENT_TIMESTAMP
             WHERE id = ?6",
            (
                status.as_str(),
                total_files as i64,
                total_dirs as i64,
                total_bytes as i64,
                error_count as i64,
                session_id,
            ),
        )?;

        // Automatically aggregate category summaries and create point-in-time snapshot on completion
        if status == ScanStatus::Completed || status == ScanStatus::CompletedWithErrors {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO category_summaries (session_id, category, file_count, total_bytes)
                 SELECT
                     session_id,
                     CASE
                         WHEN is_system = 1 THEN 'system'
                         WHEN extension IN ('rs', 'ts', 'tsx', 'js', 'jsx', 'py', 'go', 'c', 'cpp', 'h', 'hpp', 'java', 'cs', 'swift', 'kt', 'html', 'css', 'scss', 'json', 'yaml', 'yml', 'toml', 'xml', 'sql', 'sh', 'bash', 'zsh', 'bat', 'ps1') THEN 'code'
                         WHEN extension IN ('pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'txt', 'md', 'csv', 'tsv', 'rtf', 'epub', 'pages', 'numbers', 'key') THEN 'documents'
                         WHEN extension IN ('png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'ico', 'bmp', 'tiff', 'psd', 'ai', 'raw', 'heic') THEN 'images'
                         WHEN extension IN ('mp4', 'mkv', 'mov', 'avi', 'wmv', 'flv', 'webm', 'm4v', 'mpg', 'mpeg') THEN 'videos'
                         WHEN extension IN ('mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a', 'wma', 'aiff') THEN 'audio'
                         WHEN extension IN ('zip', 'tar', 'gz', '7z', 'rar', 'bz2', 'xz', 'iso', 'dmg', 'pkg', 'tgz') THEN 'archives'
                         WHEN extension IN ('exe', 'dll', 'msi', 'dylib', 'so', 'app', 'bin', 'elf') THEN 'executables'
                         ELSE 'other'
                     END AS category,
                     COUNT(*) as file_count,
                     COALESCE(SUM(size_bytes), 0) as total_bytes
                 FROM files
                 WHERE session_id = ?1
                 GROUP BY category;",
                [session_id],
            );

            let snapshot_id = uuid::Uuid::new_v4().to_string();
            let _ = conn.execute(
                "INSERT INTO storage_snapshots (id, session_id, drive_id, label, root_path, total_files, total_directories, total_bytes)
                 SELECT ?1, s.id, s.drive_id, 'Scan Completed', s.root_path, s.total_files, s.total_directories, s.total_bytes
                 FROM scan_sessions s
                 WHERE s.id = ?2;",
                (&snapshot_id, session_id),
            );
        }

        Ok(())
    }

    /// Performs a bottom-up directory rollup so parent folders accurately sum all their descendants,
    /// and flushes the aggregated directories to SQLite in batches.
    fn rollup_and_flush_directories(
        db_pool: &DbPool,
        session_id: &str,
        dir_map: &mut std::collections::HashMap<String, RawDirItem>,
        batch_size: usize,
    ) -> Result<()> {
        if dir_map.is_empty() {
            return Ok(());
        }

        // Sort descending by path length so deepest paths roll up to their parents first
        let mut dir_paths: Vec<String> = dir_map.keys().cloned().collect();
        dir_paths.sort_by(|a, b| b.len().cmp(&a.len()));

        for path in dir_paths {
            if let Some(child) = dir_map.get(&path) {
                let child_bytes = child.total_size_bytes;
                let child_files = child.file_count;
                let parent_opt = child.parent_path.clone();

                if let Some(parent_path) = parent_opt {
                    if let Some(parent) = dir_map.get_mut(&parent_path) {
                        parent.total_size_bytes += child_bytes;
                        parent.file_count += child_files;
                    }
                }
            }
        }

        // Flush rolled up directories into database in chunks
        let mut rolled_up: Vec<RawDirItem> = dir_map.drain().map(|(_, v)| v).collect();
        for chunk in rolled_up.chunks_mut(batch_size) {
            let mut chunk_vec = chunk.to_vec();
            let mut empty_files = Vec::new();
            let mut empty_errors = Vec::new();
            Self::flush_batches(db_pool, session_id, &mut empty_files, &mut chunk_vec, &mut empty_errors)?;
        }

        Ok(())
    }

    /// Automatically detects operating system boundary and virtual storage paths that must not be traversed
    /// to prevent infinite loops, duplicate scans of APFS firmlinks, or hanging on pseudo device files.
    fn is_system_internal_exclusion(path: &Path, root_path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let root_str = root_path.to_string_lossy();

        if path_str == root_str {
            return false;
        }

        #[cfg(target_os = "macos")]
        {
            // 1. /System/Volumes contains virtual sub-volumes (Data, VM, Preboot, Update).
            //    Scanning /System/Volumes/Data when root is / duplicates the entire drive!
            if path_str.starts_with("/System/Volumes") {
                return true;
            }
            // 2. Sealed immutable macOS System snapshot (/System).
            //    On macOS (APFS), /System is a cryptographically sealed, read-only snapshot (Signed System Volume).
            //    It contains 3.5+ million immutable OS files and dyld caches that cannot be modified or cleaned.
            if path_str == "/System" || path_str.starts_with("/System/") {
                return true;
            }
            // 3. Firmware and low-level hardware bundles (e.g. /usr/standalone/firmware)
            if path_str.starts_with("/usr/standalone") {
                return true;
            }
            // 4. Restricted kernel, audit, and system cache databases
            if path_str.starts_with("/private/var/db")
                || path_str.starts_with("/private/var/audit")
                || path_str.starts_with("/private/var/vm")
                || path_str.contains("/.Spotlight-V100")
                || path_str.contains("/.fseventsd")
                || path_str.contains("/.DocumentRevisions-V100")
                || path_str.contains("/.PKInstallSandboxManager")
            {
                return true;
            }
            // 5. /Volumes contains other mounted drives, network shares, DMGs, and /Volumes/Macintosh HD (loop back to /)
            if path_str.starts_with("/Volumes") && !root_str.starts_with("/Volumes") {
                return true;
            }
            // 6. /dev contains special character and FIFO device files
            if path_str.starts_with("/dev") || path_str.starts_with("/net") || path_str.starts_with("/home") {
                return true;
            }
        }

        #[cfg(target_os = "windows")]
        {
            let lower = path_str.to_lowercase();
            if lower.contains("system volume information")
                || lower.contains("$recycle.bin")
                || lower.ends_with("pagefile.sys")
                || lower.ends_with("hiberfil.sys")
                || lower.ends_with("swapfile.sys")
                || lower.contains("documents and settings")
            {
                return true;
            }
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            if path_str.starts_with("/proc")
                || path_str.starts_with("/sys")
                || path_str.starts_with("/dev")
                || path_str.starts_with("/run")
            {
                return true;
            }
        }

        false
    }

    fn is_excluded(path_str: &str, exclusions: &[String]) -> bool {
        for pattern in exclusions {
            if pattern.contains('*') {
                let clean_pat = pattern.replace("**", "").replace('*', "");
                if !clean_pat.is_empty() && path_str.contains(&clean_pat) {
                    return true;
                }
            } else if path_str.contains(pattern) {
                return true;
            }
        }
        false
    }

    fn get_allocated_bytes(meta: &fs::Metadata) -> Option<u64> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Some(meta.blocks() * 512)
        }
        #[cfg(not(unix))]
        {
            // On Windows align to 4096 byte cluster boundary
            let len = meta.len();
            let cluster_size = 4096;
            Some(((len + cluster_size - 1) / cluster_size) * cluster_size)
        }
    }

    fn get_timestamps(meta: &fs::Metadata) -> (Option<i64>, Option<i64>) {
        let created = meta
            .created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs() as i64));

        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs() as i64));

        (created, modified)
    }

    fn get_attributes(name: &str, _meta: &fs::Metadata, path: &Path) -> (bool, bool) {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::MetadataExt;
            let attrs = _meta.file_attributes();
            let is_hidden = (attrs & 0x00000002) != 0 || name.starts_with('.');
            let is_system = (attrs & 0x00000004) != 0;
            (is_hidden, is_system)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let is_hidden = name.starts_with('.');
            let path_str = path.to_string_lossy();
            let is_system = path_str.starts_with("/System")
                || path_str.starts_with("/Library")
                || path_str.starts_with("/dev")
                || path_str.starts_with("/proc");
            (is_hidden, is_system)
        }
    }
}
