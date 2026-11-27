use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tracing::{debug, info, warn};

use crate::error::Result;
use crate::models::scan::{ScanProgress, ScanStatus, ScanSummary};
use crate::services::scanner::fast_walker::{effective_file_size, metadata_timestamps};
use crate::services::scanner::models::{RawDirItem, RawErrorItem, RawFileItem, ScanOptions};
use crate::storage::DbPool;

// State codes for atomic scan status
pub const STATUS_RUNNING: u8 = 0;
pub const STATUS_PAUSED: u8 = 1;
pub const STATUS_CANCELLED: u8 = 2;
pub const STATUS_COMPLETED: u8 = 3;
pub const STATUS_FAILED: u8 = 4;

struct DirTask {
    path: PathBuf,
    known_dev_ino: Option<(u64, u64)>,
    created_at: Option<i64>,
    modified_at: Option<i64>,
}

struct WorkQueueState {
    stack: Vec<DirTask>,
    active_workers: usize,
    closed: bool,
}

struct ProgressSyncState {
    recent_files: VecDeque<String>,
    current_file: Option<String>,
    current_path: String,
}

struct WriteBatchMsg {
    files: Vec<RawFileItem>,
    errors: Vec<RawErrorItem>,
}

pub struct ScanEngine;

impl ScanEngine {
    #[allow(clippy::too_many_arguments)]
    fn emit_shared_progress(
        options: &ScanOptions,
        status_flag: &AtomicU8,
        files_atom: &AtomicU64,
        dirs_atom: &AtomicU64,
        bytes_atom: &AtomicU64,
        errors_atom: &AtomicU64,
        latest_dir: &Path,
        latest_file: Option<&str>,
        push_recent: bool,
        start_time: Instant,
        next_emit_ms: &AtomicU64,
        progress_sync: &Mutex<ProgressSyncState>,
        progress: &Arc<Mutex<ScanProgress>>,
        app_handle: &Option<AppHandle>,
        force: bool,
    ) {
        let status_code = status_flag.load(Ordering::SeqCst);
        if status_code == STATUS_CANCELLED {
            return;
        }

        let elapsed_millis = start_time.elapsed().as_millis() as u64;
        if !force {
            let prev_ms = next_emit_ms.load(Ordering::Relaxed);
            if elapsed_millis < prev_ms.saturating_add(80) {
                return;
            }
            if next_emit_ms
                .compare_exchange(prev_ms, elapsed_millis, Ordering::Relaxed, Ordering::Relaxed)
                .is_err()
            {
                return;
            }
        } else {
            next_emit_ms.store(elapsed_millis, Ordering::Relaxed);
        }

        let live_status = match status_code {
            STATUS_PAUSED => ScanStatus::Paused,
            STATUS_FAILED => ScanStatus::Failed,
            _ => ScanStatus::Running,
        };

        let dir_str = latest_dir.to_string_lossy().to_string();
        let (cur_file_snap, recent_snap) = {
            let mut sync = progress_sync.lock().unwrap_or_else(|e| e.into_inner());
            sync.current_path = dir_str.clone();
            if let Some(f) = latest_file {
                sync.current_file = Some(f.to_string());
                if push_recent {
                    if sync.recent_files.len() >= 10 {
                        sync.recent_files.pop_front();
                    }
                    sync.recent_files.push_back(f.to_string());
                }
            }
            (
                sync.current_file.clone(),
                sync.recent_files.iter().cloned().collect::<Vec<String>>(),
            )
        };

        let files_scanned = files_atom.load(Ordering::Relaxed);
        let dirs_scanned = dirs_atom.load(Ordering::Relaxed);
        let bytes_scanned = bytes_atom.load(Ordering::Relaxed);
        let error_count = errors_atom.load(Ordering::Relaxed);

        let snap = ScanProgress {
            session_id: options.session_id.clone(),
            drive_id: options.drive_id.clone(),
            root_path: options.root_path.to_string_lossy().to_string(),
            status: live_status,
            files_scanned,
            dirs_scanned,
            bytes_scanned,
            error_count,
            current_path: dir_str,
            current_file: cur_file_snap,
            recent_files: recent_snap,
            elapsed_millis,
        };

        if let Ok(mut p) = progress.lock() {
            *p = snap.clone();
        }
        if let Some(ref handle) = app_handle {
            handle.emit("scan://progress", &snap).ok();
        }
    }

    fn ensure_session_registered(conn: &rusqlite::Connection, options: &ScanOptions) -> Result<()> {
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
        Ok(())
    }

    fn mark_session_registered(
        db_pool: &DbPool,
        options: &ScanOptions,
        session_registered: &mut bool,
    ) -> Result<()> {
        if !*session_registered {
            let conn = db_pool.lock_conn();
            Self::ensure_session_registered(&conn, options)?;
            *session_registered = true;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn finalize_cancelled_scan(
        db_pool: &DbPool,
        options: &ScanOptions,
        session_registered: &mut bool,
        dir_map: &mut HashMap<String, RawDirItem>,
        file_batch: &mut Vec<RawFileItem>,
        error_batch: &mut Vec<RawErrorItem>,
        files_scanned: u64,
        dirs_scanned: u64,
        bytes_scanned: u64,
        error_count: u64,
        start_time: Instant,
        progress: &Arc<Mutex<ScanProgress>>,
        app_handle: &Option<AppHandle>,
    ) -> Result<ScanSummary> {
        info!("Scan session '{}' cancelled by user", options.session_id);
        Self::mark_session_registered(db_pool, options, session_registered)?;

        // If this drive already has a completed baseline scan, discard partial rows from the
        // cancelled run so aborted scans never accumulate dead storage in SQLite.
        let has_completed_for_drive: bool = db_pool
            .with_read_conn(|conn| {
                Ok(conn
                    .query_row(
                        "SELECT EXISTS(
                            SELECT 1 FROM scan_sessions
                            WHERE drive_id = ?1
                              AND id != ?2
                              AND status IN ('completed', 'completed_with_errors', 'completedWithErrors')
                        )",
                        rusqlite::params![&options.drive_id, &options.session_id],
                        |r| r.get::<_, bool>(0),
                    )
                    .unwrap_or(false))
            })
            .unwrap_or(false);

        if has_completed_for_drive {
            file_batch.clear();
            error_batch.clear();
            dir_map.clear();
            let _ = db_pool.prune_session_heavy_data(&options.session_id);
        } else {
            if !file_batch.is_empty() || !error_batch.is_empty() {
                let mut empty_dirs = Vec::new();
                Self::flush_batches(db_pool, &options.session_id, file_batch, &mut empty_dirs, error_batch)?;
            }
            if !dir_map.is_empty() {
                Self::rollup_and_flush_directories(
                    db_pool,
                    &options.session_id,
                    dir_map,
                    options.batch_size,
                    &options.root_path,
                )?;
                Self::ensure_root_directory_row(
                    db_pool,
                    &options.session_id,
                    &options.root_path,
                    files_scanned,
                    dirs_scanned,
                    bytes_scanned,
                )?;
            }
        }

        Self::update_session_status(
            db_pool,
            &options.session_id,
            ScanStatus::Cancelled,
            files_scanned,
            dirs_scanned,
            bytes_scanned,
            error_count,
        )?;

        if let Err(err) = db_pool.reclaim_free_pages(false) {
            warn!("Cancelled scan WAL checkpoint / reclaim failed: {}", err);
        }

        let duration_millis = start_time.elapsed().as_millis() as u64;
        if let Ok(mut p) = progress.lock() {
            p.status = ScanStatus::Cancelled;
            p.files_scanned = files_scanned;
            p.dirs_scanned = dirs_scanned;
            p.bytes_scanned = bytes_scanned;
            p.error_count = error_count;
            p.current_file = None;
            p.elapsed_millis = duration_millis;
        }

        let summary = ScanSummary {
            session_id: options.session_id.clone(),
            drive_id: options.drive_id.clone(),
            root_path: options.root_path.to_string_lossy().to_string(),
            status: ScanStatus::Cancelled,
            total_files: files_scanned,
            total_dirs: dirs_scanned,
            total_bytes: bytes_scanned,
            error_count,
            duration_millis,
        };

        if let Some(ref handle) = app_handle {
            handle.emit("scan://cancelled", &summary).ok();
        }

        Ok(summary)
    }

    /// Executes the full recursive scan synchronously on a dedicated OS thread.
    /// All filesystem I/O is blocking so this MUST run on std::thread::spawn, NOT tokio::spawn.
    pub fn run_scan(
        options: ScanOptions,
        status: Arc<AtomicU8>,
        pause_signal: Arc<(Mutex<()>, Condvar)>,
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

        // Attempt non-blocking DB registration so traversal starts immediately even if background
        // maintenance or another write holds `lock_conn()`. Deferred cleanly until first DB flush if busy.
        let mut session_registered = false;
        if let Some(conn) = db_pool.try_lock_conn() {
            Self::ensure_session_registered(&conn, &options)?;
            session_registered = true;
        } else {
            debug!("SQLite write lock busy at scan start; starting filesystem traversal immediately");
        }

        // Cycle detection shared safely across bounded worker pool
        #[cfg(unix)]
        let visited_dirs: Mutex<HashSet<(u64, u64)>> = Mutex::new(HashSet::new());
        #[cfg(unix)]
        let visited_file_inodes: Mutex<HashSet<(u64, u64)>> = Mutex::new(HashSet::new());
        #[cfg(not(unix))]
        let visited_dirs: Mutex<HashSet<PathBuf>> = Mutex::new(HashSet::new());

        // Seed work queue with root targets (macOS skips sealed SSV `/System` snapshot).
        #[cfg(target_os = "macos")]
        let seed_paths: Vec<PathBuf> = if options.root_path == std::path::Path::new("/") {
            let mut roots = Vec::new();
            for candidate in ["/opt", "/usr/local", "/Library", "/Applications", "/Users/Shared"] {
                let p = PathBuf::from(candidate);
                if p.exists() {
                    roots.push(p);
                }
            }
            if let Ok(entries) = fs::read_dir("/Users") {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() && !roots.contains(&p) {
                        roots.push(p);
                    }
                }
            }
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
        let seed_paths: Vec<PathBuf> = vec![options.root_path.clone()];

        let initial_tasks: Vec<DirTask> = seed_paths
            .into_iter()
            .map(|p| DirTask {
                path: p,
                known_dev_ino: None,
                created_at: None,
                modified_at: None,
            })
            .collect();

        let root_display = options.root_path.to_string_lossy().to_string();
        if status.load(Ordering::SeqCst) != STATUS_CANCELLED {
            if let Some(ref handle) = app_handle {
                let initial = ScanProgress {
                    session_id: options.session_id.clone(),
                    drive_id: options.drive_id.clone(),
                    root_path: root_display.clone(),
                    status: ScanStatus::Running,
                    files_scanned: 0,
                    dirs_scanned: 0,
                    bytes_scanned: 0,
                    error_count: 0,
                    current_path: root_display.clone(),
                    current_file: None,
                    recent_files: Vec::new(),
                    elapsed_millis: 0,
                };
                handle.emit("scan://progress", &initial).ok();
            }
        }

        let work_queue = Mutex::new(WorkQueueState {
            stack: initial_tasks,
            active_workers: 0,
            closed: false,
        });
        let queue_cv = Condvar::new();

        let files_atom = AtomicU64::new(0);
        let dirs_atom = AtomicU64::new(0);
        let bytes_atom = AtomicU64::new(0);
        let errors_atom = AtomicU64::new(0);
        let next_emit_ms = AtomicU64::new(0);
        let progress_sync = Mutex::new(ProgressSyncState {
            recent_files: VecDeque::with_capacity(10),
            current_file: None,
            current_path: root_display,
        });

        let batch_limit = options.batch_size.max(1);
        let worker_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(4, 8);

        let (batch_tx, batch_rx) = mpsc::sync_channel::<WriteBatchMsg>(16);

        let mut dir_map: HashMap<String, RawDirItem> = HashMap::new();
        let mut writer_error: Option<crate::error::AppError> = None;

        std::thread::scope(|scope| {
            // Dedicated pipelined SQLite writer thread so scanner workers never block on DB commits.
            let db_ref = &db_pool;
            let opts_ref = &options;
            let writer_handle = scope.spawn(move || -> Result<bool> {
                let mut writer_registered = session_registered;
                while let Ok(mut msg) = batch_rx.recv() {
                    if msg.files.is_empty() && msg.errors.is_empty() {
                        continue;
                    }
                    if !writer_registered {
                        let conn = db_ref.lock_conn();
                        Self::ensure_session_registered(&conn, opts_ref)?;
                        writer_registered = true;
                    }
                    let mut empty_dirs = Vec::new();
                    Self::flush_batches(
                        db_ref,
                        &opts_ref.session_id,
                        &mut msg.files,
                        &mut empty_dirs,
                        &mut msg.errors,
                    )?;
                }
                Ok(writer_registered)
            });

            let mut worker_handles = Vec::with_capacity(worker_count);
            for _ in 0..worker_count {
                let w_tx: SyncSender<WriteBatchMsg> = batch_tx.clone();
                let options = &options;
                let status = &status;
                let pause_signal = &pause_signal;
                let progress = &progress;
                let app_handle = &app_handle;
                let work_queue = &work_queue;
                let queue_cv = &queue_cv;
                let visited_dirs = &visited_dirs;
                #[cfg(unix)]
                let visited_file_inodes = &visited_file_inodes;
                let files_atom = &files_atom;
                let dirs_atom = &dirs_atom;
                let bytes_atom = &bytes_atom;
                let errors_atom = &errors_atom;
                let next_emit_ms = &next_emit_ms;
                let progress_sync = &progress_sync;

                worker_handles.push(scope.spawn(move || -> HashMap<String, RawDirItem> {
                    let mut local_dir_map: HashMap<String, RawDirItem> = HashMap::new();
                    let mut local_files: Vec<RawFileItem> = Vec::with_capacity(batch_limit);
                    let mut local_errors: Vec<RawErrorItem> = Vec::with_capacity(64);

                    loop {
                        if status.load(Ordering::SeqCst) == STATUS_CANCELLED {
                            let mut q = work_queue.lock().unwrap_or_else(|e| e.into_inner());
                            q.closed = true;
                            queue_cv.notify_all();
                            break;
                        }

                        if status.load(Ordering::SeqCst) == STATUS_PAUSED {
                            let (lock, cvar) = &**pause_signal;
                            let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
                            while status.load(Ordering::SeqCst) == STATUS_PAUSED {
                                guard = cvar
                                    .wait_timeout(guard, Duration::from_millis(150))
                                    .unwrap_or_else(|e| e.into_inner())
                                    .0;
                            }
                            if status.load(Ordering::SeqCst) == STATUS_CANCELLED {
                                continue;
                            }
                        }

                        let maybe_task = {
                            let mut q = work_queue.lock().unwrap_or_else(|e| e.into_inner());
                            loop {
                                if status.load(Ordering::SeqCst) == STATUS_CANCELLED || q.closed {
                                    q.closed = true;
                                    queue_cv.notify_all();
                                    break None;
                                }
                                if let Some(task) = q.stack.pop() {
                                    q.active_workers += 1;
                                    break Some(task);
                                }
                                if q.active_workers == 0 {
                                    q.closed = true;
                                    queue_cv.notify_all();
                                    break None;
                                }
                                q = queue_cv.wait(q).unwrap_or_else(|e| e.into_inner());
                            }
                        };

                        let Some(task) = maybe_task else {
                            break;
                        };

                        let finish_task = |children: Vec<DirTask>| {
                            let mut q = work_queue.lock().unwrap_or_else(|e| e.into_inner());
                            let pushed = !children.is_empty();
                            if pushed {
                                q.stack.extend(children);
                            }
                            q.active_workers = q.active_workers.saturating_sub(1);
                            if pushed {
                                queue_cv.notify_all();
                            } else if q.stack.is_empty() && q.active_workers == 0 {
                                q.closed = true;
                                queue_cv.notify_all();
                            }
                        };

                        let current_dir = task.path;

                        // 1. Check exclusions before touching filesystem
                        if Self::is_system_internal_exclusion(&current_dir, &options.root_path) {
                            finish_task(Vec::new());
                            continue;
                        }
                        let current_dir_str = current_dir.to_string_lossy();
                        if Self::is_excluded(&current_dir_str, &options.exclusions) {
                            finish_task(Vec::new());
                            continue;
                        }

                        // 2. Deduplicate only if root seed (`known_dev_ino == None`); children are pre-verified!
                        let (dir_created, dir_modified) = if task.known_dev_ino.is_some() {
                            (task.created_at, task.modified_at)
                        } else {
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::MetadataExt;
                                if let Ok(m) = fs::symlink_metadata(&current_dir) {
                                    let key = (m.dev(), m.ino());
                                    if !visited_dirs.lock().unwrap_or_else(|e| e.into_inner()).insert(key) {
                                        finish_task(Vec::new());
                                        continue;
                                    }
                                    metadata_timestamps(&m)
                                } else {
                                    (None, None)
                                }
                            }
                            #[cfg(not(unix))]
                            {
                                if let Ok(canon) = fs::canonicalize(&current_dir) {
                                    if !visited_dirs.lock().unwrap_or_else(|e| e.into_inner()).insert(canon) {
                                        finish_task(Vec::new());
                                        continue;
                                    }
                                }
                                fs::symlink_metadata(&current_dir)
                                    .map(|m| metadata_timestamps(&m))
                                    .unwrap_or((None, None))
                            }
                        };

                        // 3. Read directory contents
                        let entries = match fs::read_dir(&current_dir) {
                            Ok(e) => {
                                dirs_atom.fetch_add(1, Ordering::Relaxed);
                                e
                            }
                            Err(err) => {
                                dirs_atom.fetch_add(1, Ordering::Relaxed);
                                errors_atom.fetch_add(1, Ordering::Relaxed);
                                let err_type = match err.kind() {
                                    std::io::ErrorKind::PermissionDenied => "PermissionDenied",
                                    std::io::ErrorKind::NotFound => "NotFound",
                                    _ => "IoError",
                                };
                                local_errors.push(RawErrorItem {
                                    path: current_dir_str.to_string(),
                                    error_type: err_type.to_string(),
                                    message: err.to_string(),
                                });
                                Self::emit_shared_progress(
                                    &options,
                                    &status,
                                    &files_atom,
                                    &dirs_atom,
                                    &bytes_atom,
                                    &errors_atom,
                                    &current_dir,
                                    None,
                                    false,
                                    start_time,
                                    &next_emit_ms,
                                    &progress_sync,
                                    &progress,
                                    &app_handle,
                                    false,
                                );
                                finish_task(Vec::new());
                                continue;
                            }
                        };

                        let mut dir_file_count: u64 = 0;
                        let mut dir_total_bytes: u64 = 0;
                        let mut dir_subdirs: u64 = 0;
                        let mut pending_children: Vec<DirTask> = Vec::new();
                        let mut entry_idx: usize = 0;
                        let parent_str_owned = current_dir_str.to_string();

                        for entry_res in entries {
                            entry_idx += 1;
                            if entry_idx & 63 == 0 {
                                if status.load(Ordering::SeqCst) == STATUS_CANCELLED {
                                    break;
                                }
                                if status.load(Ordering::SeqCst) == STATUS_PAUSED {
                                    let (lock, cvar) = &**pause_signal;
                                    let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
                                    while status.load(Ordering::SeqCst) == STATUS_PAUSED {
                                        guard = cvar
                                            .wait_timeout(guard, Duration::from_millis(150))
                                            .unwrap_or_else(|e| e.into_inner())
                                            .0;
                                    }
                                    if status.load(Ordering::SeqCst) == STATUS_CANCELLED {
                                        break;
                                    }
                                }
                            }

                            let entry = match entry_res {
                                Ok(e) => e,
                                Err(err) => {
                                    errors_atom.fetch_add(1, Ordering::Relaxed);
                                    local_errors.push(RawErrorItem {
                                        path: parent_str_owned.clone(),
                                        error_type: "EntryReadError".to_string(),
                                        message: err.to_string(),
                                    });
                                    continue;
                                }
                            };

                            let entry_path = entry.path();
                            let entry_name = entry.file_name().to_string_lossy().to_string();

                            let meta = match fs::symlink_metadata(&entry_path) {
                                Ok(m) => m,
                                Err(err) => {
                                    errors_atom.fetch_add(1, Ordering::Relaxed);
                                    local_errors.push(RawErrorItem {
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
                                if !is_symlink
                                    && !Self::is_system_internal_exclusion(&entry_path, &options.root_path)
                                {
                                    #[cfg(unix)]
                                    {
                                        use std::os::unix::fs::MetadataExt;
                                        let key = (meta.dev(), meta.ino());
                                        if visited_dirs.lock().unwrap_or_else(|e| e.into_inner()).insert(key) {
                                            let (c_at, m_at) = metadata_timestamps(&meta);
                                            pending_children.push(DirTask {
                                                path: entry_path,
                                                known_dev_ino: Some(key),
                                                created_at: c_at,
                                                modified_at: m_at,
                                            });
                                        }
                                    }
                                    #[cfg(not(unix))]
                                    {
                                        if let Ok(canon) = fs::canonicalize(&entry_path) {
                                            if visited_dirs.lock().unwrap_or_else(|e| e.into_inner()).insert(canon) {
                                                let (c_at, m_at) = metadata_timestamps(&meta);
                                                pending_children.push(DirTask {
                                                    path: entry_path,
                                                    known_dev_ino: Some((0, 0)),
                                                    created_at: c_at,
                                                    modified_at: m_at,
                                                });
                                            }
                                        }
                                    }
                                }
                            } else if meta.is_file() || is_symlink {
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::MetadataExt;
                                    if meta.nlink() > 1 {
                                        let fkey = (meta.dev(), meta.ino());
                                        if !visited_file_inodes
                                            .lock()
                                            .unwrap_or_else(|e| e.into_inner())
                                            .insert(fkey)
                                        {
                                            continue;
                                        }
                                    }
                                }

                                let (size, allocated_bytes) = effective_file_size(&meta);
                                let new_files = files_atom.fetch_add(1, Ordering::Relaxed) + 1;
                                bytes_atom.fetch_add(size, Ordering::Relaxed);
                                dir_file_count += 1;
                                dir_total_bytes += size;

                                let extension = entry_path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .map(|s| s.to_ascii_lowercase());

                                let (created_at, modified_at) = metadata_timestamps(&meta);
                                let (is_hidden, is_system) =
                                    Self::get_attributes(&entry_name, &meta, &entry_path);
                                let path_str = entry_path.to_string_lossy().to_string();

                                if new_files <= 2 || new_files % 32 == 0 {
                                    Self::emit_shared_progress(
                                        &options,
                                        &status,
                                        &files_atom,
                                        &dirs_atom,
                                        &bytes_atom,
                                        &errors_atom,
                                        &current_dir,
                                        Some(&path_str),
                                        true,
                                        start_time,
                                        &next_emit_ms,
                                        &progress_sync,
                                        &progress,
                                        &app_handle,
                                        new_files <= 2,
                                    );
                                }

                                let class_res = crate::services::classifier::rules::RuleMatcher::classify(
                                    &path_str,
                                    &entry_name,
                                    extension.as_deref(),
                                    is_hidden,
                                    is_system,
                                );

                                local_files.push(RawFileItem {
                                    path: path_str,
                                    name: entry_name,
                                    extension,
                                    parent_path: parent_str_owned.clone(),
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

                                if local_files.len() >= batch_limit {
                                    let send_files = std::mem::replace(
                                        &mut local_files,
                                        Vec::with_capacity(batch_limit),
                                    );
                                    let send_errors = std::mem::take(&mut local_errors);
                                    let _ = w_tx.send(WriteBatchMsg {
                                        files: send_files,
                                        errors: send_errors,
                                    });
                                }
                            }
                        }

                        let dir_name = current_dir
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| parent_str_owned.clone());
                        let is_dir_hidden = dir_name.starts_with('.');
                        let dir_parent = current_dir
                            .parent()
                            .map(|p| p.to_string_lossy().to_string());

                        local_dir_map.insert(
                            parent_str_owned.clone(),
                            RawDirItem {
                                path: parent_str_owned,
                                name: dir_name,
                                parent_path: dir_parent,
                                file_count: dir_file_count,
                                dir_count: dir_subdirs,
                                total_size_bytes: dir_total_bytes,
                                created_at: dir_created,
                                modified_at: dir_modified,
                                is_hidden: is_dir_hidden,
                                is_system: false,
                            },
                        );

                        Self::emit_shared_progress(
                            &options,
                            &status,
                            &files_atom,
                            &dirs_atom,
                            &bytes_atom,
                            &errors_atom,
                            &current_dir,
                            None,
                            false,
                            start_time,
                            &next_emit_ms,
                            &progress_sync,
                            &progress,
                            &app_handle,
                            false,
                        );

                        finish_task(pending_children);
                    }

                    if !local_files.is_empty() || !local_errors.is_empty() {
                        let _ = w_tx.send(WriteBatchMsg {
                            files: local_files,
                            errors: local_errors,
                        });
                    }

                    local_dir_map
                }));
            }

            // Drop the original sender so writer_handle drains remaining queued batches and terminates.
            drop(batch_tx);

            for handle in worker_handles {
                if let Ok(local_map) = handle.join() {
                    dir_map.extend(local_map);
                }
            }

            match writer_handle.join() {
                Ok(Ok(registered)) => {
                    session_registered = registered;
                }
                Ok(Err(e)) => {
                    writer_error = Some(e);
                }
                Err(_) => {
                    writer_error = Some(crate::error::AppError::System(
                        "Scan DB writer thread panicked".to_string(),
                    ));
                }
            }
        });

        if let Some(err) = writer_error {
            return Err(err);
        }

        let files_scanned = files_atom.load(Ordering::Relaxed);
        let dirs_scanned = dirs_atom.load(Ordering::Relaxed);
        let bytes_scanned = bytes_atom.load(Ordering::Relaxed);
        let error_count = errors_atom.load(Ordering::Relaxed);

        let mut file_batch: Vec<RawFileItem> = Vec::new();
        let mut error_batch: Vec<RawErrorItem> = Vec::new();

        if status.load(Ordering::SeqCst) == STATUS_CANCELLED {
            return Self::finalize_cancelled_scan(
                &db_pool,
                &options,
                &mut session_registered,
                &mut dir_map,
                &mut file_batch,
                &mut error_batch,
                files_scanned,
                dirs_scanned,
                bytes_scanned,
                error_count,
                start_time,
                &progress,
                &app_handle,
            );
        }

        Self::mark_session_registered(&db_pool, &options, &mut session_registered)?;

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

        // Helper closure to emit stage progress updates during post-scan enrichment
        let emit_stage = |msg: &str| {
            if let Ok(mut p) = progress.lock() {
                p.current_path = msg.to_string();
                p.current_file = None;
                p.elapsed_millis = start_time.elapsed().as_millis() as u64;
            }
            if let Some(ref handle) = app_handle {
                if let Ok(p_snap) = progress.lock() {
                    handle.emit("scan://progress", &*p_snap).ok();
                }
            }
        };

        // Automated post-scan intelligence pipeline (with cancel checks & error logging):
        // 1. Storage Classification Summaries
        if status.load(Ordering::SeqCst) == STATUS_CANCELLED {
            return Self::finalize_cancelled_scan(
                &db_pool,
                &options,
                &mut session_registered,
                &mut dir_map,
                &mut file_batch,
                &mut error_batch,
                files_scanned,
                dirs_scanned,
                bytes_scanned,
                error_count,
                start_time,
                &progress,
                &app_handle,
            );
        }

        emit_stage("Finalizing storage classification summaries...");
        info!("Running automated post-scan classification for session '{}'", options.session_id);
        if let Err(err) = crate::services::classifier::ClassifierService::ensure_session_summaries(&db_pool, &options.session_id) {
            warn!("Post-scan classification summary failed for session '{}': {}", options.session_id, err);
        }

        // 2. Developer Storage Project Attribution (DB-only; skips Docker CLI / host cache crawls)
        if status.load(Ordering::SeqCst) == STATUS_CANCELLED {
            return Self::finalize_cancelled_scan(
                &db_pool,
                &options,
                &mut session_registered,
                &mut dir_map,
                &mut file_batch,
                &mut error_batch,
                files_scanned,
                dirs_scanned,
                bytes_scanned,
                error_count,
                start_time,
                &progress,
                &app_handle,
            );
        }

        emit_stage("Profiling developer project storage...");
        info!("Running automated developer storage discovery for session '{}'", options.session_id);
        if let Err(err) = crate::services::developer::DeveloperService::analyze_project_storage_db(&db_pool, &options.session_id) {
            warn!("Post-scan developer analysis failed for session '{}': {}", options.session_id, err);
        }

        // 3. Storage Intelligence Recommendations
        if status.load(Ordering::SeqCst) == STATUS_CANCELLED {
            return Self::finalize_cancelled_scan(
                &db_pool,
                &options,
                &mut session_registered,
                &mut dir_map,
                &mut file_batch,
                &mut error_batch,
                files_scanned,
                dirs_scanned,
                bytes_scanned,
                error_count,
                start_time,
                &progress,
                &app_handle,
            );
        }

        emit_stage("Generating storage intelligence recommendations...");
        info!("Generating storage intelligence recommendations for session '{}'", options.session_id);
        {
            let conn = db_pool.lock_conn();
            if let Err(err) = crate::services::intelligence::IntelligenceService::generate_intelligence_profile(&conn, Some(&options.session_id)) {
                warn!("Post-scan intelligence generation failed for session '{}': {}", options.session_id, err);
            }
        }

        // Persist final completion status, category summaries, and snapshot AFTER all stages finish
        Self::update_session_status(
            &db_pool,
            &options.session_id,
            final_status,
            files_scanned,
            dirs_scanned,
            bytes_scanned,
            error_count,
        )?;

        // Prune superseded raw file/directory rows from prior scans of this drive
        // (preserving lightweight scan_sessions, category_summaries, and storage_snapshots)
        // and reclaim freed SQLite pages + truncate the WAL file.
        match db_pool.prune_superseded_scans_and_reclaim(Some(&options.drive_id), false, false) {
            Ok((pruned, reclaimed)) => {
                if pruned > 0 || reclaimed > 0 {
                    info!(
                        "Post-scan retention pruned {} superseded session tree(s) on '{}' and reclaimed {} bytes",
                        pruned, options.drive_id, reclaimed
                    );
                }
            }
            Err(err) => {
                warn!("Post-scan retention/reclaim failed: {}", err);
                let _ = db_pool.checkpoint_wal();
            }
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
            let mut p = progress.lock().unwrap_or_else(|e| e.into_inner());
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

        let mut conn = db_pool.lock_conn();
        if !dir_batch.is_empty() {
            DbPool::ensure_directories_unique_index(&conn)?;
        }
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

        // 2. Directories (idempotent upsert via unique index on (session_id, path))
        if !dir_batch.is_empty() {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO directories (
                    session_id, path, name, parent_path, file_count, dir_count,
                    total_size_bytes, created_at, modified_at, is_hidden, is_system
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(session_id, path) DO UPDATE SET
                    parent_path = excluded.parent_path,
                    file_count = excluded.file_count,
                    dir_count = excluded.dir_count,
                    total_size_bytes = excluded.total_size_bytes,
                    created_at = excluded.created_at,
                    modified_at = excluded.modified_at"
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
        let conn = db_pool.lock_conn();
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

        // Automatically aggregate canonical category summaries and create point-in-time snapshot on completion
        if status == ScanStatus::Completed || status == ScanStatus::CompletedWithErrors {
            conn.execute("DELETE FROM category_summaries WHERE session_id = ?1", [session_id])?;
            if let Err(err) = conn.execute(
                "INSERT INTO category_summaries (session_id, category, file_count, total_bytes)
                 SELECT
                     session_id,
                     COALESCE(NULLIF(category, ''), 'unknown') AS cat,
                     COUNT(*) as file_count,
                     COALESCE(SUM(size_bytes), 0) as total_bytes
                 FROM files
                 WHERE session_id = ?1
                 GROUP BY session_id, COALESCE(NULLIF(category, ''), 'unknown');",
                [session_id],
            ) {
                warn!("Failed aggregating category_summaries for '{}': {}", session_id, err);
            }

            let snapshot_id = uuid::Uuid::new_v4().to_string();
            if let Err(err) = conn.execute(
                "INSERT INTO storage_snapshots (id, session_id, drive_id, label, root_path, total_files, total_directories, total_bytes)
                 SELECT ?1, s.id, s.drive_id, 'Scan Completed', s.root_path, s.total_files, s.total_directories, s.total_bytes
                 FROM scan_sessions s
                 WHERE s.id = ?2;",
                (&snapshot_id, session_id),
            ) {
                warn!("Failed inserting completion snapshot for '{}': {}", session_id, err);
            }
        }

        Ok(())
    }

    /// Performs a bottom-up directory rollup so parent folders accurately sum all their descendants,
    /// and flushes the aggregated directories to SQLite in batches.
    pub(crate) fn rollup_and_flush_directories(
        db_pool: &DbPool,
        session_id: &str,
        dir_map: &mut HashMap<String, RawDirItem>,
        batch_size: usize,
        root_path: &Path,
    ) -> Result<()> {
        if dir_map.is_empty() {
            return Ok(());
        }

        // 1. Synthesize missing intermediate directories between seeded roots (e.g. `/Users`,
        //    `/usr` on macOS `/` scans) and `root_path` so the directory hierarchy and
        //    breadcrumbs stay connected without dropping branches.
        let root_str = root_path.to_string_lossy().to_string();
        let synthesized = Self::synthesize_missing_ancestors(dir_map, &root_str);

        // 2. Repair any remaining orphaned parent links to nearest known ancestor or `root_str`.
        Self::repair_orphaned_parents(dir_map, &root_str);

        // 3. Sort descending by path length so deepest paths roll up to their parents first
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
                        if synthesized.contains(&parent_path) {
                            parent.dir_count += 1;
                        }
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

    fn is_strict_descendant(candidate: &str, root: &str) -> bool {
        if candidate.is_empty() || candidate == root {
            return false;
        }
        if root == "/" {
            return candidate.starts_with('/');
        }
        if root.ends_with('\\') || root.ends_with('/') {
            return candidate.starts_with(root) && candidate.len() > root.len();
        }
        candidate.starts_with(&format!("{}/", root)) || candidate.starts_with(&format!("{}\\", root))
    }

    fn synthesize_missing_ancestors(
        dir_map: &mut HashMap<String, RawDirItem>,
        root_str: &str,
    ) -> HashSet<String> {
        let mut synthesized = HashSet::new();
        let initial_paths: Vec<String> = dir_map.keys().cloned().collect();

        for path_str in initial_paths {
            let mut cur = PathBuf::from(&path_str);
            while let Some(parent) = cur.parent() {
                let p_str = parent.to_string_lossy().to_string();
                if p_str.is_empty() || p_str == root_str || !Self::is_strict_descendant(&p_str, root_str) {
                    break;
                }
                if !dir_map.contains_key(&p_str) {
                    let parent_of_parent = parent.parent().map(|pp| pp.to_string_lossy().to_string());
                    let name = parent
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| p_str.clone());
                    let (created_at, modified_at) = match fs::metadata(parent) {
                        Ok(m) => Self::get_timestamps(&m),
                        Err(_) => (None, None),
                    };
                    let is_hidden = name.starts_with('.');
                    dir_map.insert(
                        p_str.clone(),
                        RawDirItem {
                            path: p_str.clone(),
                            name,
                            parent_path: parent_of_parent,
                            file_count: 0,
                            dir_count: 0,
                            total_size_bytes: 0,
                            created_at,
                            modified_at,
                            is_hidden,
                            is_system: false,
                        },
                    );
                    synthesized.insert(p_str.clone());
                }
                cur = parent.to_path_buf();
            }
        }

        synthesized
    }

    /// Re-parents directories whose parent directory has no row of its own to the nearest
    /// ancestor that does (or `root_path`), so the whole tree remains reachable from the root.
    fn repair_orphaned_parents(
        dir_map: &mut HashMap<String, RawDirItem>,
        root_path: &str,
    ) {
        let known: HashSet<String> = dir_map.keys().cloned().collect();

        let orphans: Vec<(String, String)> = dir_map
            .iter()
            .filter_map(|(path, item)| {
                if path == root_path {
                    return None;
                }
                let parent = item.parent_path.clone()?;
                if parent.is_empty() || parent == *path {
                    return None;
                }
                // If parent equals the scan root, it will link directly to the root row created by ensure_root_directory_row
                if parent == root_path || known.contains(&parent) {
                    return None;
                }
                Some((path.clone(), parent))
            })
            .collect();

        for (path, parent) in orphans {
            let mut candidate = Path::new(&parent);
            let mut resolved: Option<String> = None;
            loop {
                let cand_str = candidate.to_string_lossy().to_string();
                if cand_str.is_empty() || cand_str == root_path {
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

            let new_parent = resolved.or_else(|| {
                if root_path.is_empty() {
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
    pub(crate) fn ensure_root_directory_row(
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

        let conn = db_pool.lock_conn();
        DbPool::ensure_directories_unique_index(&conn)?;

        let (created_at, modified_at) = match fs::metadata(root_path) {
            Ok(m) => Self::get_timestamps(&m),
            Err(_) => (None, None),
        };

        let root_name = root_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root_str.clone());

        conn.execute(
            "INSERT INTO directories (
                session_id, path, name, parent_path, file_count, dir_count,
                total_size_bytes, created_at, modified_at, is_hidden, is_system
             ) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, 0, 1)
             ON CONFLICT(session_id, path) DO UPDATE SET
                parent_path = NULL,
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
                || path_str.contains("/.timemachine")
                || path_str.contains("/.MobileBackups")
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
        let norm_path = path_str.replace('\\', "/");
        for raw in exclusions {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let norm_pat = trimmed.replace('\\', "/");
            let clean_pat = norm_pat.trim_matches('*').trim_matches('/');
            if clean_pat.is_empty() {
                continue;
            }

            if !clean_pat.contains('/') {
                // Single segment name (e.g. "node_modules", ".git", "**/.git/**")
                if norm_path.split('/').any(|seg| seg.eq_ignore_ascii_case(clean_pat)) {
                    return true;
                }
            } else if norm_path.to_lowercase().contains(&clean_pat.to_lowercase()) {
                return true;
            }
        }
        false
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
                || path_str.starts_with("/Library/Apple")
                || path_str.starts_with("/Library/Extensions")
                || path_str.starts_with("/usr/bin")
                || path_str.starts_with("/usr/lib")
                || path_str.starts_with("/usr/sbin")
                || path_str.starts_with("/usr/libexec")
                || path_str.starts_with("/usr/standalone")
                || path_str.starts_with("/usr/share")
                || path_str.starts_with("/bin/")
                || path_str.starts_with("/sbin/")
                || path_str.starts_with("/dev")
                || path_str.starts_with("/proc");
            (is_hidden, is_system)
        }
    }
}
