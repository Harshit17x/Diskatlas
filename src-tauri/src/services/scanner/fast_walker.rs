use std::collections::HashSet;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};

#[derive(Debug, Clone, Copy, Default)]
pub struct FastDirStats {
    pub size_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub latest_modified: Option<i64>,
}

/// Computes physical allocated size where APFS/NTFS sparse or cloud-evicted stubs report
/// smaller allocated blocks than logical `len()`.
pub fn effective_file_size(meta: &Metadata) -> (u64, Option<u64>) {
    let raw_len = meta.len();

    #[cfg(unix)]
    let allocated = {
        use std::os::unix::fs::MetadataExt;
        Some(meta.blocks().saturating_mul(512))
    };

    #[cfg(not(unix))]
    let allocated = {
        let cluster_size = 4096u64;
        Some(((raw_len + cluster_size - 1) / cluster_size) * cluster_size)
    };

    let size = if let Some(alloc) = allocated {
        if raw_len > 0 && alloc < raw_len && (alloc == 0 || raw_len > 10 * 1024 * 1024) {
            alloc
        } else {
            raw_len
        }
    } else {
        raw_len
    };

    (size, allocated)
}

pub fn metadata_timestamps(meta: &Metadata) -> (Option<i64>, Option<i64>) {
    let created = meta
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs() as i64));
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs() as i64));
    (created, modified)
}

struct DirQueueState {
    stack: Vec<PathBuf>,
    active_workers: usize,
    done: bool,
}

/// Measures a directory tree's physical size, file count, directory count, and newest mtime
/// using a bounded multi-worker traversal pool (inspired by Mole's bounded concurrency model)
/// and `(dev, ino)` deduplication for directories & multi-hardlinked files.
pub fn measure_dir_parallel(root: &Path) -> FastDirStats {
    measure_dir_parallel_with_cancel(root, None)
}

pub fn measure_dir_parallel_with_cancel(root: &Path, cancel_flag: Option<&AtomicBool>) -> FastDirStats {
    let meta = match fs::symlink_metadata(root) {
        Ok(m) => m,
        Err(_) => return FastDirStats::default(),
    };
    if meta.file_type().is_symlink() {
        return FastDirStats::default();
    }
    if meta.is_file() {
        let (size, _) = effective_file_size(&meta);
        let (_, modified) = metadata_timestamps(&meta);
        return FastDirStats {
            size_bytes: size,
            file_count: 1,
            dir_count: 0,
            latest_modified: modified,
        };
    }
    if !meta.is_dir() {
        return FastDirStats::default();
    }

    let (_, root_mtime) = metadata_timestamps(&meta);
    let queue_lock = Mutex::new(DirQueueState {
        stack: vec![root.to_path_buf()],
        active_workers: 0,
        done: false,
    });
    let queue_cv = Condvar::new();

    #[cfg(unix)]
    let seen_inodes: Mutex<HashSet<(u64, u64)>> = {
        use std::os::unix::fs::MetadataExt;
        let mut s = HashSet::new();
        s.insert((meta.dev(), meta.ino()));
        Mutex::new(s)
    };

    let total_bytes = AtomicU64::new(0);
    let total_files = AtomicU64::new(0);
    let total_dirs = AtomicU64::new(0);
    let latest_mtime = AtomicU64::new(root_mtime.unwrap_or(0).max(0) as u64);
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 6);

    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            scope.spawn(|| {
                let mut local_bytes: u64 = 0;
                let mut local_files: u64 = 0;
                let mut local_dirs: u64 = 0;
                let mut local_mtime: u64 = 0;

                loop {
                    if cancel_flag.map(|f| f.load(Ordering::Relaxed)).unwrap_or(false) {
                        break;
                    }

                    let dir = {
                        let mut guard = queue_lock.lock().unwrap_or_else(|e| e.into_inner());
                        loop {
                            if cancel_flag.map(|f| f.load(Ordering::Relaxed)).unwrap_or(false) {
                                guard.done = true;
                                queue_cv.notify_all();
                                break None;
                            }
                            if let Some(p) = guard.stack.pop() {
                                guard.active_workers += 1;
                                break Some(p);
                            }
                            if guard.active_workers == 0 {
                                guard.done = true;
                                queue_cv.notify_all();
                                break None;
                            }
                            guard = queue_cv.wait(guard).unwrap_or_else(|e| e.into_inner());
                            if guard.done {
                                break None;
                            }
                        }
                    };

                    let Some(current_dir) = dir else {
                        break;
                    };

                    local_dirs += 1;
                    let mut child_dirs: Vec<PathBuf> = Vec::new();

                    if let Ok(entries) = fs::read_dir(&current_dir) {
                        for entry in entries.flatten() {
                            let entry_path = entry.path();
                            let Ok(entry_meta) = fs::symlink_metadata(&entry_path) else {
                                continue;
                            };
                            if entry_meta.file_type().is_symlink() {
                                continue;
                            }
                            if entry_meta.is_dir() {
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::MetadataExt;
                                    let key = (entry_meta.dev(), entry_meta.ino());
                                    let mut seen = seen_inodes.lock().unwrap_or_else(|e| e.into_inner());
                                    if !seen.insert(key) {
                                        continue;
                                    }
                                }
                                child_dirs.push(entry_path);
                            } else if entry_meta.is_file() {
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::MetadataExt;
                                    if entry_meta.nlink() > 1 {
                                        let key = (entry_meta.dev(), entry_meta.ino());
                                        let mut seen = seen_inodes.lock().unwrap_or_else(|e| e.into_inner());
                                        if !seen.insert(key) {
                                            continue;
                                        }
                                    }
                                }
                                let (sz, _) = effective_file_size(&entry_meta);
                                local_bytes = local_bytes.saturating_add(sz);
                                local_files += 1;
                                if let Some(m) = entry_meta
                                    .modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs()))
                                {
                                    if m > local_mtime {
                                        local_mtime = m;
                                    }
                                }
                            }
                        }
                    }

                    {
                        let mut guard = queue_lock.lock().unwrap_or_else(|e| e.into_inner());
                        let pushed = !child_dirs.is_empty();
                        if pushed {
                            guard.stack.extend(child_dirs);
                        }
                        guard.active_workers = guard.active_workers.saturating_sub(1);
                        if pushed {
                            queue_cv.notify_all();
                        } else if guard.stack.is_empty() && guard.active_workers == 0 {
                            guard.done = true;
                            queue_cv.notify_all();
                        }
                    }
                }

                total_bytes.fetch_add(local_bytes, Ordering::Relaxed);
                total_files.fetch_add(local_files, Ordering::Relaxed);
                total_dirs.fetch_add(local_dirs, Ordering::Relaxed);
                if local_mtime > 0 {
                    latest_mtime.fetch_max(local_mtime, Ordering::Relaxed);
                }
            });
        }
    });

    let max_m = latest_mtime.load(Ordering::Relaxed);
    FastDirStats {
        size_bytes: total_bytes.load(Ordering::Relaxed),
        file_count: total_files.load(Ordering::Relaxed),
        dir_count: total_dirs.load(Ordering::Relaxed),
        latest_modified: if max_m > 0 { Some(max_m as i64) } else { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_measure_dir_parallel_counts_files_and_dedups_hardlinks() {
        let dir = std::env::temp_dir().join(format!("diskatlas_fastwalk_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("nested")).unwrap();

        let p1 = dir.join("a.bin");
        fs::File::create(&p1).unwrap().write_all(&vec![1u8; 4096]).unwrap();
        let p2 = dir.join("nested/b.bin");
        fs::File::create(&p2).unwrap().write_all(&vec![2u8; 2048]).unwrap();

        #[cfg(unix)]
        {
            let link = dir.join("nested/a_hardlink.bin");
            fs::hard_link(&p1, &link).unwrap();
        }

        let stats = measure_dir_parallel(&dir);
        assert_eq!(stats.size_bytes, 4096 + 2048);
        assert_eq!(stats.file_count, 2);
        assert!(stats.dir_count >= 2);
        assert!(stats.latest_modified.is_some());

        let _ = fs::remove_dir_all(&dir);
    }
}
