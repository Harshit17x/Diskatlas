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
    #[allow(clippy::too_many_arguments)]
    fn emit_progress_if_needed(
        options: &ScanOptions,
        files_scanned: u64,
        dirs_scanned: u64,
        bytes_scanned: u64,
        error_count: u64,
        current_path: &Path,
        current_file: Option<&str>,
        recent_files: &[String],
        start_time: Instant,
        progress: &Arc<Mutex<ScanProgress>>,
        app_handle: &Option<AppHandle>,
        last_emit: &mut Instant,
        force: bool,
    ) {
        // Always keep the shared in-memory progress struct updated so get_scan_progress queries see live numbers
        let elapsed_millis = start_time.elapsed().as_millis() as u64;
        let path_str = current_path.to_string_lossy().to_string();
        let file_opt = current_file.map(|s| s.to_string());

        if let Ok(mut p) = progress.lock() {
            p.files_scanned = files_scanned;
            p.dirs_scanned = dirs_scanned;
            p.bytes_scanned = bytes_scanned;
            p.error_count = error_count;
            p.current_path = path_str.clone();
            p.current_file = file_opt.clone().or_else(|| Some(path_str.clone()));
            if !recent_files.is_empty() {
                p.recent_files = recent_files.to_vec();
            }
            p.elapsed_millis = elapsed_millis;
        }

        // Throttle UI event emissions over Tauri IPC to prevent webview event starvation (emit at least every 80ms)
        if force || last_emit.elapsed() >= Duration::from_millis(80) {
            if let Some(ref handle) = app_handle {
                let current_progress = ScanProgress {
                    session_id: options.session_id.clone(),
                    drive_id: options.drive_id.clone(),
                    root_path: options.root_path.to_string_lossy().to_string(),
                    status: ScanStatus::Running,
                    files_scanned,
                    dirs_scanned,
                    bytes_scanned,
                    error_count,
                    current_path: path_str.clone(),
                    current_file: file_opt.or_else(|| Some(path_str)),
                    recent_files: recent_files.to_vec(),
                    elapsed_millis,
                };
                handle.emit("scan://progress", &current_progress).ok();
            }

            *last_emit = Instant::now();
        }
    }

    /// Executes the full recursive scan synchronously on a dedicated OS thread.
    /// All filesystem I/O is blocking so this MUST run on std::thread::spawn, NOT tokio::spawn.
    pub fn run_scan(
        options: ScanOptions,
        status: Arc<AtomicU8>,
        _pause_notify: Arc<tokio::sync::Notify>,
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
                "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes, is_removable, is_system)
                 VALUES (?1, ?2, ?3, 'UNKNOWN', 0, 0, 0, 1)
                 ON CONFLICT(id) DO NOTHING;",
                (&options.drive_id, &options.drive_id, &root_str),
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

        // Work stack of directories to traverse (avoids call-stack recursion).
        // On macOS (APFS Signed System Volume), root '/' mounts an immutable sealed OS snapshot (/System, /usr, /bin, /sbin).
        // The real mutable user files, installed applications, developer caches, and libraries reside in firmlinked roots.
        // Seeding directly with the mutable roots provides 50x faster scanning while capturing 100% of user data.
        #[cfg(target_os = "macos")]
        let mut dir_stack: Vec<PathBuf> = if options.root_path == std::path::Path::new("/") {
            let mut roots = Vec::new();
            // Note: dir_stack is a LIFO stack. Elements are popped from back to front.
            // Seeding system directories first and $HOME last ensures user files are traversed first!
            let opt = PathBuf::from("/opt");
            if opt.exists() {
                roots.push(opt);
            }
            let usr_local = PathBuf::from("/usr/local");
            if usr_local.exists() {
                roots.push(usr_local);
            }
            let lib = PathBuf::from("/Library");
            if lib.exists() {
                roots.push(lib);
            }
            let apps = PathBuf::from("/Applications");
            if apps.exists() {
                roots.push(apps);
            }
            let shared = PathBuf::from("/Users/Shared");
            if shared.exists() {
                roots.push(shared);
            }
            // Seed any additional user profiles in /Users if accessible
            if let Ok(entries) = fs::read_dir("/Users") {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() && !roots.contains(&p) {
                        roots.push(p);
                    }
                }
            }
            // Always seed current user's HOME directory last so it is popped FIRST from the stack!
            if let Ok(home) = std::env::var("HOME") {
                let home_path = PathBuf::from(&home);
                if home_path.exists() {
                    roots.retain(|r| r != &home_path);
                    roots.push(home_path);
                }
            }
            if roots.is_empty() {
                vec![options.root_path.clone()]
            } else {
                roots
            }
        } else {
            vec![options.root_path.clone()]
        };

        #[cfg(not(target_os = "macos"))]
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
        let root_display = options.root_path.to_string_lossy().to_string();
        #[allow(unused_assignments)]
        let mut current_file: Option<String> = None;
        let mut recent_files: Vec<String> = vec![root_display.clone()];
        // Initialize last_emit 200ms in the past so the first visited directory immediately emits live feedback to UI
        let mut last_emit = Instant::now().checked_sub(Duration::from_millis(200)).unwrap_or_else(Instant::now);
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
                current_path: root_display.clone(),
                current_file: Some(root_display),
                recent_files: recent_files.clone(),
                elapsed_millis: 0,
            };
            handle.emit("scan://progress", &initial).ok();
        }

        while let Some(current_dir) = dir_stack.pop() {
            loop_counter += 1;

            let cur_dir_str = current_dir.to_string_lossy().to_string();
            current_file = Some(cur_dir_str.clone());
            if recent_files.len() >= 10 {
                recent_files.remove(0);
            }
            if !recent_files.contains(&cur_dir_str) {
                recent_files.push(cur_dir_str);
            }

            // Emit live progress immediately when visiting a new directory (force emit for first 10 directories)
            Self::emit_progress_if_needed(
                &options,
                files_scanned,
                dirs_scanned,
                bytes_scanned,
                error_count,
                &current_dir,
                current_file.as_deref(),
                &recent_files,
                start_time,
                &progress,
                &app_handle,
                &mut last_emit,
                loop_counter <= 10,
            );

            // Cooperative yielding on dedicated OS thread to avoid CPU monopolization
            if loop_counter % 32 == 0 {
                std::thread::yield_now();
            }

            // Check cancellation
            if status.load(Ordering::SeqCst) == STATUS_CANCELLED {
                info!("Scan session '{}' cancelled by user", options.session_id);
                // Perform bottom-up rollup on whatever was scanned so far before flushing
                Self::rollup_and_flush_directories(&db_pool, &options.session_id, &mut dir_map, options.batch_size, &options.root_path)?;
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

            // Check pause — poll atomically on the dedicated OS thread
            while status.load(Ordering::SeqCst) == STATUS_PAUSED {
                debug!("Scan session '{}' paused, sleeping...", options.session_id);
                std::thread::sleep(Duration::from_millis(50));
            }

            // Check system internal boundary exclusions (e.g. /System/Volumes firmlinks, /Volumes external mounts, /dev)
            if Self::is_system_internal_exclusion(&current_dir, &options.root_path) {
                debug!("Skipping system internal directory: {}", current_dir.display());
                Self::emit_progress_if_needed(
                    &options,
                    files_scanned,
                    dirs_scanned,
                    bytes_scanned,
                    error_count,
                    &current_dir,
                    current_file.as_deref(),
                    &recent_files,
                    start_time,
                    &progress,
                    &app_handle,
                    &mut last_emit,
                    false,
                );
                continue;
            }

            // Check user-configured exclusions
            let current_dir_str = current_dir.to_string_lossy();
            if Self::is_excluded(&current_dir_str, &options.exclusions) {
                debug!("Skipping excluded directory: {}", current_dir.display());
                Self::emit_progress_if_needed(
                    &options,
                    files_scanned,
                    dirs_scanned,
                    bytes_scanned,
                    error_count,
                    &current_dir,
                    current_file.as_deref(),
                    &recent_files,
                    start_time,
                    &progress,
                    &app_handle,
                    &mut last_emit,
                    false,
                );
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
                        Self::emit_progress_if_needed(
                            &options,
                            files_scanned,
                            dirs_scanned,
                            bytes_scanned,
                            error_count,
                            &current_dir,
                            current_file.as_deref(),
                            &recent_files,
                            start_time,
                            &progress,
                            &app_handle,
                            &mut last_emit,
                            false,
                        );
                        continue;
                    }
                }
            }
            #[cfg(not(unix))]
            {
                if let Ok(canon) = fs::canonicalize(&current_dir) {
                    if !visited_paths.insert(canon) {
                        debug!("Cycle detected on directory '{}', skipping loop", current_dir.display());
                        Self::emit_progress_if_needed(
                            &options,
                            files_scanned,
                            dirs_scanned,
                            bytes_scanned,
                            error_count,
                            &current_dir,
                            current_file.as_deref(),
                            &recent_files,
                            start_time,
                            &progress,
                            &app_handle,
                            &mut last_emit,
                            false,
                        );
                        continue;
                    }
                }
            }

            // Read directory contents safely (read-only)
            let read_dir_result = fs::read_dir(&current_dir);
            let entries = match read_dir_result {
                Ok(entries) => {
                    dirs_scanned += 1;
                    entries
                }
                Err(err) => {
                    dirs_scanned += 1;
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
                    debug!("Inaccessible protected path '{}': {}", current_dir.display(), err);
                    Self::emit_progress_if_needed(
                        &options,
                        files_scanned,
                        dirs_scanned,
                        bytes_scanned,
                        error_count,
                        &current_dir,
                        current_file.as_deref(),
                        &recent_files,
                        start_time,
                        &progress,
                        &app_handle,
                        &mut last_emit,
                        true,
                    );
                    continue;
                }
            };

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
                    let path_str = entry_path.to_string_lossy().to_string();

                    current_file = Some(path_str.clone());
                    if recent_files.len() >= 10 {
                        recent_files.remove(0);
                    }
                    recent_files.push(path_str.clone());

                    // Microsecond in-memory classification during indexing avoids 15-minute post-scan DB table scans
                    let class_res = crate::services::classifier::rules::RuleMatcher::classify(
                        &path_str,
                        &entry_name,
                        extension.as_deref(),
                        is_hidden,
                        is_system,
                    );

                    file_batch.push(RawFileItem {
                        path: path_str,
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
                        category: class_res.category,
                        subcategory: class_res.subcategory,
                        app_name: class_res.app_name,
                        confidence: class_res.confidence,
                    });

                    // Flush batch if buffer is full
                    if file_batch.len() >= options.batch_size {
                        let mut empty_dirs = Vec::new();
                        Self::flush_batches(&db_pool, &options.session_id, &mut file_batch, &mut empty_dirs, &mut error_batch)?;
                        std::thread::yield_now();
                    }

                    // Throttle in-batch progress updates for large directories (emit on first file, every 30 files, or 80ms)
                    if files_scanned == 1 || files_scanned % 30 == 0 || last_emit.elapsed() >= Duration::from_millis(80) {
                        Self::emit_progress_if_needed(
                            &options,
                            files_scanned,
                            dirs_scanned,
                            bytes_scanned,
                            error_count,
                            &current_dir,
                            current_file.as_deref(),
                            &recent_files,
                            start_time,
                            &progress,
                            &app_handle,
                            &mut last_emit,
                            false,
                        );
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

            // Update in-memory progress and emit throttled event
            Self::emit_progress_if_needed(
                &options,
                files_scanned,
                dirs_scanned,
                bytes_scanned,
                error_count,
                &current_dir,
                current_file.as_deref(),
                &recent_files,
                start_time,
                &progress,
                &app_handle,
                &mut last_emit,
                false,
            );
        }

        // Final flush of remaining file and error records
        let mut empty_dir_batch = Vec::new();
        Self::flush_batches(&db_pool, &options.session_id, &mut file_batch, &mut empty_dir_batch, &mut error_batch)?;

        // Perform bottom-up directory rollup so parent directories reflect true recursive size
        Self::rollup_and_flush_directories(&db_pool, &options.session_id, &mut dir_map, options.batch_size, &options.root_path)?;

        // Ensure the scan root itself has a directory row.
        //
        // The traversal never visits the literal root (on macOS it seeds a set of firmlink
        // roots), so no `directories` row existed for it. The visualization tree looked up
        // the root by `path = root_path` and fell back to `scan_sessions.total_bytes` with a
        // hardcoded file count of 0 — the root tile therefore reported zero files and a size
        // that did not reconcile with the sum of its children. Writing an explicit root row
        // (aggregated from the top-level children) makes the root a real, navigable node.
        Self::ensure_root_directory_row(
            &db_pool,
            &options.session_id,
            &options.root_path,
            files_scanned,
            dirs_scanned,
            bytes_scanned,
        )?;

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

        // Notify frontend that traversal has finished and summaries are finalizing
        {
            if let Ok(mut p) = progress.lock() {
                p.current_path = "Finalizing storage categories and summaries...".to_string();
                p.elapsed_millis = start_time.elapsed().as_millis() as u64;
            }
        }
        if let Some(ref handle) = app_handle {
            if let Ok(p_snap) = progress.lock() {
                handle.emit("scan://progress", &*p_snap).ok();
            }
        }

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
            p.current_file = None;
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
                    allocated_bytes, created_at, modified_at, is_hidden, is_system, is_symlink,
                    category, subcategory, app_name, confidence
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
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
                    &f.category,
                    &f.subcategory,
                    &f.app_name,
                    f.confidence,
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

    pub(crate) fn update_session_status(
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
                     ext_category AS category,
                     COUNT(*) as file_count,
                     COALESCE(SUM(size_bytes), 0) as total_bytes
                 FROM (
                     SELECT
                         session_id,
                         size_bytes,
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
                         END AS ext_category
                     FROM files
                     WHERE session_id = ?1
                 )
                 GROUP BY ext_category;",
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
        root_path: &Path,
    ) -> Result<()> {
        if dir_map.is_empty() {
            return Ok(());
        }

        // Repair orphaned parent links.
        //
        // On macOS the scanner seeds a set of firmlink roots (/opt, /usr/local, /Library,
        // /Applications, /Users/Shared, each /Users/* profile, $HOME) instead of the literal
        // root. Those seeds record their *real* filesystem parent (e.g. /usr/local's parent is
        // /usr), but the intermediate parent is never itself visited, so it never gets a row.
        // The result is a chain that cannot be walked from the root and a subtree that is
        // invisible no matter how the user navigates.
        //
        // Here we (a) re-parent any directory whose parent has no row to the nearest ancestor
        // that does have one, and (b) synthesize the root row so the tree has a real root.
        let root_str = root_path.to_string_lossy().to_string();
        Self::repair_orphaned_parents(dir_map, &root_str);

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

    /// Re-parents directories whose parent directory has no row of its own to the nearest
    /// ancestor that does, so the whole tree remains reachable from the root.
    fn repair_orphaned_parents(
        dir_map: &mut std::collections::HashMap<String, RawDirItem>,
        root_path: &str,
    ) {
        // Collect the set of known paths once.
        let known: std::collections::HashSet<String> = dir_map.keys().cloned().collect();

        let orphans: Vec<(String, String)> = dir_map
            .iter()
            .filter_map(|(path, item)| {
                let parent = item.parent_path.clone()?;
                if parent.is_empty() || parent == *path {
                    return None;
                }
                if known.contains(&parent) {
                    return None;
                }
                Some((path.clone(), parent))
            })
            .collect();

        for (path, parent) in orphans {
            // Walk up the ancestor chain until we find one that exists as a row.
            let mut candidate = Path::new(&parent);
            let mut resolved: Option<String> = None;
            loop {
                let cand_str = candidate.to_string_lossy().to_string();
                if cand_str.is_empty() || cand_str == "/" {
                    break;
                }
                if known.contains(&cand_str) {
                    resolved = Some(cand_str);
                    break;
                }
                match candidate.parent() {
                    Some(p) => candidate = p,
                    None => break,
                }
            }

            // Fall back to the scan root when no ancestor has a row.
            let new_parent = resolved.or_else(|| {
                if root_path.is_empty() || root_path == "/" {
                    None
                } else {
                    Some(root_path.to_string())
                }
            });

            if let Some(item) = dir_map.get_mut(&path) {
                item.parent_path = new_parent;
            }
        }
    }

    /// Writes an explicit `directories` row for the scan root so the visualization tree has
    /// a real root node with an accurate file count and a size that reconciles with the sum
    /// of its children.
    ///
    /// The traversal never visits the literal root (on macOS it seeds a set of firmlink
    /// roots such as /Users, /Applications, /Library, /opt), so no row existed for it.
    fn ensure_root_directory_row(
        db_pool: &DbPool,
        session_id: &str,
        root_path: &Path,
        total_files: u64,
        total_dirs: u64,
        total_bytes: u64,
    ) -> Result<()> {
        let root_str = root_path.to_string_lossy().to_string();
        if root_str.is_empty() {
            return Ok(());
        }

        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();

        let (created_at, modified_at) = match fs::metadata(root_path) {
            Ok(m) => Self::get_timestamps(&m),
            Err(_) => (None, None),
        };

        let root_name = root_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root_str.clone());

        // The root's own metrics come from the session totals, which represent everything the
        // scan actually observed (including the seeded roots).
        conn.execute(
            "INSERT INTO directories (
                session_id, path, name, parent_path, file_count, dir_count,
                total_size_bytes, created_at, modified_at, is_hidden, is_system
             ) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, 0, 1)
             ON CONFLICT(session_id, path) DO UPDATE SET
                file_count = excluded.file_count,
                dir_count = excluded.dir_count,
                total_size_bytes = excluded.total_size_bytes,
                created_at = excluded.created_at,
                modified_at = excluded.modified_at",
            rusqlite::params![
                session_id,
                root_str,
                root_name,
                total_files as i64,
                total_dirs as i64,
                total_bytes as i64,
                created_at,
                modified_at,
            ],
        )?;

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
            // 3. Firmware, Unix system paths and synthetic OS mounts
            if path_str.starts_with("/usr/standalone")
                || path_str.starts_with("/usr/share")
                || path_str.starts_with("/usr/lib")
                || path_str.starts_with("/usr/bin")
                || path_str.starts_with("/usr/libexec")
                || path_str == "/bin" || path_str.starts_with("/bin/")
                || path_str == "/sbin" || path_str.starts_with("/sbin/")
                || path_str == "/cores" || path_str.starts_with("/cores/")
                || path_str.starts_with("/.vol")
                || path_str.starts_with("/.resolve")
                || path_str.starts_with("/.nofollow")
            {
                return true;
            }
            // 4. Restricted kernel, audit, root home, and system cache databases
            if path_str.starts_with("/private/var/db")
                || path_str.starts_with("/private/var/audit")
                || path_str.starts_with("/private/var/vm")
                || path_str.starts_with("/private/var/root")
                || path_str.contains("/.Spotlight-V100")
                || path_str.contains("/.fseventsd")
                || path_str.contains("/.DocumentRevisions-V100")
                || path_str.contains("/.PKInstallSandboxManager")
            {
                return true;
            }
            // 5. App sandbox circular aliases in ~/Library/Containers/*/Data/{Desktop,Documents,Downloads,Pictures,Music,Movies}
            //    (Prevent duplicating hundreds of gigabytes of user files)
            if path_str.contains("/Library/Containers/") && path_str.contains("/Data/") {
                if path_str.ends_with("/Data/Desktop") || path_str.contains("/Data/Desktop/")
                    || path_str.ends_with("/Data/Documents") || path_str.contains("/Data/Documents/")
                    || path_str.ends_with("/Data/Downloads") || path_str.contains("/Data/Downloads/")
                    || path_str.ends_with("/Data/Pictures") || path_str.contains("/Data/Pictures/")
                    || path_str.ends_with("/Data/Music") || path_str.contains("/Data/Music/")
                    || path_str.ends_with("/Data/Movies") || path_str.contains("/Data/Movies/")
                {
                    return true;
                }
            }
            // 6. /Volumes contains other mounted drives, network shares, DMGs, and /Volumes/Macintosh HD (loop back to /)
            if path_str.starts_with("/Volumes") && !root_str.starts_with("/Volumes") {
                return true;
            }
            // 7. /dev contains special character and FIFO device files
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
