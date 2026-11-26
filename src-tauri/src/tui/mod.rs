use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::services::cleanup::safeguards;

#[derive(Debug, Clone)]
struct PendingDelete {
    path: PathBuf,
    name: String,
    size_bytes: u64,
    cloud_warning: Option<String>,
}

fn lock_state(state: &Arc<Mutex<TuiScanState>>) -> MutexGuard<'_, TuiScanState> {
    state.lock().unwrap_or_else(|e| e.into_inner())
}

fn truncate_left_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars || max_chars <= 3 {
        return s.to_string();
    }
    let skip = char_count - (max_chars - 3);
    let tail: String = s.chars().skip(skip).collect();
    format!("...{}", tail)
}

fn truncate_right_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars || max_chars <= 3 {
        return s.to_string();
    }
    let head: String = s.chars().take(max_chars - 3).collect();
    format!("{}...", head)
}

fn is_unsafe_tui_delete(path: &Path) -> Option<&'static str> {
    let (is_critical, reason) = safeguards::is_system_critical_path(path);
    if is_critical {
        Some(reason.unwrap_or("Protected operating system or boot component"))
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct TuiItem {
    pub path: PathBuf,
    pub name: String,
    pub size_bytes: u64,
    pub is_dir: bool,
    pub file_count: u64,
}

#[derive(Default)]
pub struct TuiScanState {
    pub is_scanning: bool,
    pub files_scanned: u64,
    pub dirs_scanned: u64,
    pub bytes_scanned: u64,
    pub current_path: String,
    pub active_item: String,
    pub items: Vec<TuiItem>,
}

pub struct RawTerminalGuard;

impl RawTerminalGuard {
    pub fn enter() -> Self {
        // Switch terminal to raw mode and hide cursor
        #[cfg(unix)]
        {
            let _ = std::process::Command::new("stty")
                .args(["raw", "-echo"])
                .status();
        }
        print!("\x1b[?25l\x1b[s"); // hide cursor and save pos
        let _ = io::stdout().flush();
        Self
    }
}

impl Drop for RawTerminalGuard {
    fn drop(&mut self) {
        // Restore terminal to cooked mode and show cursor
        print!("\x1b[?25h\x1b[u\x1b[0m\r\n");
        let _ = io::stdout().flush();
        #[cfg(unix)]
        {
            let _ = std::process::Command::new("stty")
                .args(["-raw", "echo"])
                .status();
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;
    if b >= TB {
        format!("{:.2} TB", b / TB)
    } else if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_number(val: u64) -> String {
    let s = val.to_string();
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

pub fn make_bar(percent: f64, width: usize) -> String {
    let p = percent.clamp(0.0, 1.0);
    let filled = (p * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "\x1b[36m{}\x1b[90m{}\x1b[0m",
        "█".repeat(filled),
        "░".repeat(empty)
    )
}

/// Computes recursive directory size and file count for direct child entries
fn scan_child_path(
    path: &Path,
    cancel_flag: &Arc<AtomicBool>,
    state: &Arc<Mutex<TuiScanState>>,
) -> (u64, u64) {
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;

    let mut total_bytes = 0u64;
    let mut total_files = 0u64;
    let mut visited_dirs: HashSet<(u64, u64)> = HashSet::new();
    let mut seen_inodes: HashSet<(u64, u64)> = HashSet::new();

    #[cfg(unix)]
    if let Ok(root_meta) = std::fs::symlink_metadata(path) {
        visited_dirs.insert((root_meta.dev(), root_meta.ino()));
    }

    let mut stack = vec![path.to_path_buf()];

    while let Some(current) = stack.pop() {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        let cur_str = current.to_string_lossy().to_string();
        {
            let mut s = lock_state(state);
            s.dirs_scanned += 1;
            s.active_item = cur_str;
        }

        let entries = match std::fs::read_dir(&current) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry_res in entries {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }

            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };
            let entry_path = entry.path();
            let meta = match std::fs::symlink_metadata(&entry_path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            if meta.file_type().is_symlink() {
                continue; // Avoid symlink cycles
            }

            if meta.is_dir() {
                #[cfg(unix)]
                if !visited_dirs.insert((meta.dev(), meta.ino())) {
                    continue;
                }
                // Skip virtual firmlinks and heavy container mirrors
                let p_str = entry_path.to_string_lossy();
                if p_str.starts_with("/System/Volumes")
                    || p_str.contains("/Library/Containers/") && p_str.contains("/Data/")
                {
                    continue;
                }
                stack.push(entry_path);
            } else if meta.is_file() {
                let mut counted_size = meta.len();
                #[cfg(unix)]
                if meta.nlink() > 1 && !seen_inodes.insert((meta.dev(), meta.ino())) {
                    counted_size = 0;
                }
                total_bytes += counted_size;
                total_files += 1;

                let mut s = lock_state(state);
                s.files_scanned += 1;
                s.bytes_scanned += counted_size;
            }
        }
    }

    let _ = (&visited_dirs, &seen_inodes);
    (total_bytes, total_files)
}

/// Scans the target folder and populates direct child sizes
pub fn run_directory_analysis(
    target_path: &Path,
    cancel_flag: &Arc<AtomicBool>,
    state: &Arc<Mutex<TuiScanState>>,
) {
    {
        let mut s = lock_state(state);
        s.is_scanning = true;
        s.current_path = target_path.to_string_lossy().to_string();
        s.items.clear();
    }

    let entries = match std::fs::read_dir(target_path) {
        Ok(e) => e,
        Err(_) => {
            lock_state(state).is_scanning = false;
            return;
        }
    };

    let mut direct_items = Vec::new();
    for entry_res in entries.flatten() {
        let p = entry_res.path();
        let name = entry_res.file_name().to_string_lossy().to_string();
        let meta = match std::fs::symlink_metadata(&p) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_symlink = meta.file_type().is_symlink();
        let is_dir = !is_symlink && meta.is_dir();
        let size = if is_symlink { 0 } else { meta.len() };
        direct_items.push((p, name, is_dir, size));
    }

    let mut scanned_items = Vec::new();

    for (p, name, is_dir, file_len) in direct_items {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        if is_dir {
            let (dir_size, dir_files) = scan_child_path(&p, cancel_flag, state);
            scanned_items.push(TuiItem {
                path: p,
                name,
                size_bytes: dir_size,
                is_dir: true,
                file_count: dir_files,
            });
        } else {
            {
                let mut s = lock_state(state);
                s.files_scanned += 1;
                s.bytes_scanned += file_len;
            }
            scanned_items.push(TuiItem {
                path: p,
                name,
                size_bytes: file_len,
                is_dir: false,
                file_count: 1,
            });
        }

        // Keep items sorted descending by size
        let mut sorted = scanned_items.clone();
        sorted.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes).then_with(|| a.name.cmp(&b.name)));
        lock_state(state).items = sorted;
    }

    scanned_items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes).then_with(|| a.name.cmp(&b.name)));
    {
        let mut s = lock_state(state);
        s.items = scanned_items;
        s.is_scanning = false;
        s.active_item = "Scan complete".to_string();
    }
}

pub fn launch_tui(initial_path: Option<PathBuf>) {
    let resolved_path = initial_path.unwrap_or_else(|| {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    });

    let _guard = RawTerminalGuard::enter();

    let state = Arc::new(Mutex::new(TuiScanState::default()));
    let cancel_flag = Arc::new(AtomicBool::new(false));

    let mut current_dir = resolved_path;
    let mut selected_index = 0usize;
    let mut scroll_offset = 0usize;
    let mut status_message = String::new();
    let mut pending_delete: Option<PendingDelete> = None;

    let spawn_scan = |dir: &PathBuf, flag: &Arc<AtomicBool>, st: &Arc<Mutex<TuiScanState>>| {
        flag.store(false, Ordering::SeqCst);
        let c = Arc::clone(flag);
        let s = Arc::clone(st);
        let d = dir.clone();
        std::thread::spawn(move || {
            run_directory_analysis(&d, &c, &s);
        })
    };

    // Spawn initial scanner thread
    let mut scan_handle = spawn_scan(&current_dir, &cancel_flag, &state);

    let start_time = Instant::now();
    let mut last_draw = Instant::now();

    loop {
        // Redraw terminal UI every ~40ms or on keypress
        if last_draw.elapsed() >= Duration::from_millis(40) {
            let snap = {
                let s = lock_state(&state);
                (
                    s.is_scanning,
                    s.files_scanned,
                    s.dirs_scanned,
                    s.bytes_scanned,
                    s.active_item.clone(),
                    s.items.clone(),
                )
            };
            let (is_scanning, files, dirs, _bytes, active_item, items) = snap;

            let total_bytes: u64 = items.iter().map(|it| it.size_bytes).sum();
            let total_size_display = format_bytes(total_bytes);

            // Clear screen & home cursor
            print!("\x1b[2J\x1b[H");

            // 1. Header Banner
            println!("\x1b[1;36m┌────────────────────────────────────────────────────────────────────────┐\x1b[0m");
            println!(
                "\x1b[1;36m│\x1b[0m \x1b[1;37mDiskAtlas TUI - Universal Storage Intelligence\x1b[0m  \x1b[90m(tw93/mole method)\x1b[0m  \x1b[1;36m│\x1b[0m"
            );
            println!("\x1b[1;36m└────────────────────────────────────────────────────────────────────────┘\x1b[0m");

            // 2. Location & Stats Bar
            println!(
                "\x1b[1;33m📁 Target:\x1b[0m \x1b[1;37m{}\x1b[0m",
                current_dir.display()
            );

            let status_badge = if is_scanning {
                "\x1b[1;32m● SCANNING\x1b[0m"
            } else {
                "\x1b[1;34m✔ READY\x1b[0m"
            };

            let elapsed_sec = start_time.elapsed().as_secs();
            println!(
                "Status: {} │ Files: \x1b[37m{}\x1b[0m │ Dirs: \x1b[37m{}\x1b[0m │ Sized: \x1b[1;32m{}\x1b[0m │ Time: \x1b[90m{}s\x1b[0m",
                status_badge,
                format_number(files),
                format_number(dirs),
                total_size_display,
                elapsed_sec
            );

            if is_scanning {
                let truncated_item = truncate_left_chars(&active_item, 60);
                println!("\x1b[90mTraversing: {}\x1b[0m", truncated_item);
            } else {
                println!("\x1b[90mFound {} items in current folder\x1b[0m", items.len());
            }

            println!("\x1b[90m────────────────────────────────────────────────────────────────────────\x1b[0m");
            println!(
                "\x1b[1;37m {:<3}  {:<4}  {:<32} {:>10}  {:<12} {:>8}\x1b[0m",
                "#", "TYPE", "NAME", "SIZE", "PERCENT", "ITEMS"
            );
            println!("\x1b[90m────────────────────────────────────────────────────────────────────────\x1b[0m");

            // Render table items with paging
            let max_rows = 15usize;
            if selected_index >= items.len() && !items.is_empty() {
                selected_index = items.len() - 1;
            }
            if selected_index < scroll_offset {
                scroll_offset = selected_index;
            } else if selected_index >= scroll_offset + max_rows {
                scroll_offset = selected_index - max_rows + 1;
            }

            let visible_items = items
                .iter()
                .enumerate()
                .skip(scroll_offset)
                .take(max_rows);

            for (idx, item) in visible_items {
                let is_selected = idx == selected_index;
                let ratio = if total_bytes > 0 {
                    item.size_bytes as f64 / total_bytes as f64
                } else {
                    0.0
                };
                let bar = make_bar(ratio, 10);
                let pct_str = format!("{:>3.0}%", ratio * 100.0);

                let type_badge = if item.is_dir {
                    "\x1b[1;34m[DIR]\x1b[0m"
                } else {
                    "\x1b[90m[FIL]\x1b[0m"
                };

                let name_display = truncate_right_chars(&item.name, 30);

                if is_selected {
                    println!(
                        "\x1b[7m {:<3}  {:<4}  {:<32} {:>10}  {:>3} {:>8} \x1b[0m",
                        idx + 1,
                        if item.is_dir { "[DIR]" } else { "[FIL]" },
                        name_display,
                        format_bytes(item.size_bytes),
                        pct_str,
                        item.file_count
                    );
                } else {
                    println!(
                        " {:<3}  {}  \x1b[37m{:<32}\x1b[0m \x1b[1;33m{:>10}\x1b[0m  {} {} \x1b[90m{:>8}\x1b[0m",
                        idx + 1,
                        type_badge,
                        name_display,
                        format_bytes(item.size_bytes),
                        bar,
                        pct_str,
                        item.file_count
                    );
                }
            }

            if items.is_empty() {
                if is_scanning {
                    println!("\x1b[33m  Scanning directory contents... Please wait.\x1b[0m");
                } else {
                    println!("\x1b[90m  Empty directory.\x1b[0m");
                }
            }

            // Fill empty rows to keep terminal height consistent
            let rendered_count = items.len().saturating_sub(scroll_offset).min(max_rows);
            for _ in rendered_count..max_rows {
                println!();
            }

            println!("\x1b[90m────────────────────────────────────────────────────────────────────────\x1b[0m");
            if let Some(ref pending) = pending_delete {
                let cloud_note = pending
                    .cloud_warning
                    .as_deref()
                    .map(|w| format!(" [CLOUD: {}]", w))
                    .unwrap_or_default();
                println!(
                    "\x1b[1;33m⚠ Trash '{}' ({})?{} Press [y] to confirm or any key to cancel.\x1b[0m",
                    pending.name,
                    format_bytes(pending.size_bytes),
                    cloud_note
                );
            } else if !status_message.is_empty() {
                println!("\x1b[1;32m✔ {}\x1b[0m", status_message);
            } else {
                println!(
                    "\x1b[1;37m[j/↓]\x1b[0m Down  \x1b[1;37m[k/↑]\x1b[0m Up  \x1b[1;37m[Enter/l]\x1b[0m Open  \x1b[1;37m[Backspace/h]\x1b[0m Up  \x1b[1;37m[d]\x1b[0m Trash  \x1b[1;37m[r]\x1b[0m Rescan  \x1b[1;37m[q]\x1b[0m Quit"
                );
            }

            let _ = io::stdout().flush();
            last_draw = Instant::now();
        }

        // Non-blocking poll for stdin input
        let mut buffer = [0u8; 8];
        let bytes_read = poll_nonblocking_stdin(&mut buffer).unwrap_or(0);

        if bytes_read > 0 {
            let key = &buffer[..bytes_read];

            if let Some(pending) = pending_delete.take() {
                if key == b"y" || key == b"Y" {
                    if let Some(reason) = is_unsafe_tui_delete(&pending.path) {
                        status_message = format!("Blocked by safety safeguards: {}", reason);
                    } else {
                        match trash::delete(&pending.path) {
                            Ok(_) => {
                                status_message = format!("Moved to Trash: {}", pending.name);
                                cancel_flag.store(true, Ordering::SeqCst);
                                let _ = scan_handle.join();
                                scan_handle = spawn_scan(&current_dir, &cancel_flag, &state);
                            }
                            Err(e) => {
                                status_message = format!("Failed to trash: {}", e);
                            }
                        }
                    }
                } else {
                    status_message = "Trash cancelled.".to_string();
                }
                continue;
            }

            status_message.clear();

            match key {
                // 'q' or Ctrl+C -> Exit
                b"q" | b"Q" | &[3] => {
                    cancel_flag.store(true, Ordering::SeqCst);
                    let _ = scan_handle.join();
                    break;
                }
                // 'j' or Down arrow (\x1b[B)
                b"j" | b"\x1b[B" => {
                    let items_len = lock_state(&state).items.len();
                    if items_len > 0 && selected_index + 1 < items_len {
                        selected_index += 1;
                    }
                }
                // 'k' or Up arrow (\x1b[A)
                b"k" | b"\x1b[A" => {
                    if selected_index > 0 {
                        selected_index -= 1;
                    }
                }
                // Enter (\r or \n) or 'l' or Right arrow (\x1b[C) -> Drill down
                b"\r" | b"\n" | b"l" | b"\x1b[C" => {
                    let items = lock_state(&state).items.clone();
                    if let Some(item) = items.get(selected_index) {
                        if item.is_dir && item.path.exists() {
                            cancel_flag.store(true, Ordering::SeqCst);
                            let _ = scan_handle.join();

                            current_dir = item.path.clone();
                            selected_index = 0;
                            scroll_offset = 0;
                            scan_handle = spawn_scan(&current_dir, &cancel_flag, &state);
                        }
                    }
                }
                // Backspace (127 or 8) or 'h' or Left arrow (\x1b[D) -> Go to parent
                &[127] | &[8] | b"h" | b"\x1b[D" => {
                    if let Some(parent) = current_dir.parent() {
                        cancel_flag.store(true, Ordering::SeqCst);
                        let _ = scan_handle.join();

                        current_dir = parent.to_path_buf();
                        selected_index = 0;
                        scroll_offset = 0;
                        scan_handle = spawn_scan(&current_dir, &cancel_flag, &state);
                    }
                }
                // 'r' -> Rescan
                b"r" | b"R" => {
                    cancel_flag.store(true, Ordering::SeqCst);
                    let _ = scan_handle.join();
                    scan_handle = spawn_scan(&current_dir, &cancel_flag, &state);
                    status_message = "Rescanning current directory...".to_string();
                }
                // 'd' -> Stage item for explicit confirmation (safeguarded)
                b"d" | b"D" => {
                    let items = lock_state(&state).items.clone();
                    if let Some(item) = items.get(selected_index) {
                        if let Some(reason) = is_unsafe_tui_delete(&item.path) {
                            status_message = format!("Blocked by safety safeguards: {}", reason);
                        } else {
                            let (_, cloud_warning) = safeguards::detect_cloud_sync(&item.path);
                            pending_delete = Some(PendingDelete {
                                path: item.path.clone(),
                                name: item.name.clone(),
                                size_bytes: item.size_bytes,
                                cloud_warning,
                            });
                        }
                    }
                }
                _ => {}
            }
        } else {
            std::thread::sleep(Duration::from_millis(15));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_safe_truncation() {
        let ascii = "abcdefghijklmnopqrstuvwxyz".repeat(3);
        let left = truncate_left_chars(&ascii, 20);
        assert!(left.starts_with("..."));
        assert_eq!(left.chars().count(), 20);

        let multi = "文件夹_📁_αβγδεζηθικλμνξοπρστυφχψω_end";
        let left_mb = truncate_left_chars(multi, 16);
        let right_mb = truncate_right_chars(multi, 16);
        assert_eq!(left_mb.chars().count(), 16);
        assert_eq!(right_mb.chars().count(), 16);
    }

    #[test]
    fn test_tui_delete_blocks_system_critical_paths() {
        assert!(is_unsafe_tui_delete(Path::new("/")).is_some());
        assert!(is_unsafe_tui_delete(Path::new("/System/Library")).is_some());
        assert!(is_unsafe_tui_delete(Path::new("C:\\Windows\\System32")).is_some());
        assert!(is_unsafe_tui_delete(Path::new("/tmp/safe_temp_artifact.zip")).is_none());
    }
}

#[cfg(unix)]
fn poll_nonblocking_stdin(buf: &mut [u8]) -> io::Result<usize> {
    use std::os::unix::io::AsRawFd;
    let stdin_fd = io::stdin().as_raw_fd();

    unsafe {
        let mut pollfd = libc::pollfd {
            fd: stdin_fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let res = libc::poll(&mut pollfd, 1, 0); // non-blocking poll with 0 timeout
        if res > 0 && (pollfd.revents & libc::POLLIN) != 0 {
            io::stdin().read(buf)
        } else {
            Ok(0)
        }
    }
}

#[cfg(not(unix))]
fn poll_nonblocking_stdin(_buf: &mut [u8]) -> io::Result<usize> {
    Ok(0)
}
