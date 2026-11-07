use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
    let mut total_bytes = 0u64;
    let mut total_files = 0u64;

    let mut stack = vec![path.to_path_buf()];

    while let Some(current) = stack.pop() {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        let cur_str = current.to_string_lossy().to_string();
        if let Ok(mut s) = state.lock() {
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
                // Skip virtual firmlinks and heavy caches
                let p_str = entry_path.to_string_lossy();
                if p_str.starts_with("/System/Volumes")
                    || p_str.contains("/Library/Containers/") && p_str.contains("/Data/")
                {
                    continue;
                }
                stack.push(entry_path);
            } else if meta.is_file() {
                let size = meta.len();
                total_bytes += size;
                total_files += 1;

                if let Ok(mut s) = state.lock() {
                    s.files_scanned += 1;
                    s.bytes_scanned += size;
                }
            }
        }
    }

    (total_bytes, total_files)
}

/// Scans the target folder and populates direct child sizes
pub fn run_directory_analysis(
    target_path: &Path,
    cancel_flag: &Arc<AtomicBool>,
    state: &Arc<Mutex<TuiScanState>>,
) {
    if let Ok(mut s) = state.lock() {
        s.is_scanning = true;
        s.current_path = target_path.to_string_lossy().to_string();
        s.items.clear();
    }

    let entries = match std::fs::read_dir(target_path) {
        Ok(e) => e,
        Err(_) => {
            if let Ok(mut s) = state.lock() {
                s.is_scanning = false;
            }
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
        let is_dir = meta.is_dir();
        direct_items.push((p, name, is_dir, meta.len()));
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
            if let Ok(mut s) = state.lock() {
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
        if let Ok(mut s) = state.lock() {
            s.items = sorted;
        }
    }

    scanned_items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes).then_with(|| a.name.cmp(&b.name)));
    if let Ok(mut s) = state.lock() {
        s.items = scanned_items;
        s.is_scanning = false;
        s.active_item = "Scan complete".to_string();
    }
}

pub fn launch_tui(initial_path: Option<PathBuf>) {
    let resolved_path = initial_path.unwrap_or_else(|| {
        std::env::var("HOME")
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

    // Spawn initial scanner thread
    let mut scan_cancel = Arc::clone(&cancel_flag);
    let mut scan_state = Arc::clone(&state);
    let cur_dir_clone = current_dir.clone();
    let mut scan_handle = std::thread::spawn(move || {
        run_directory_analysis(&cur_dir_clone, &scan_cancel, &scan_state);
    });

    let start_time = Instant::now();
    let mut last_draw = Instant::now();

    loop {
        // Redraw terminal UI every ~40ms or on keypress
        if last_draw.elapsed() >= Duration::from_millis(40) {
            let snap = {
                let s = state.lock().unwrap();
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
                let truncated_item = if active_item.len() > 60 {
                    format!("...{}", &active_item[active_item.len() - 57..])
                } else {
                    active_item
                };
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

                let mut name_display = item.name.clone();
                if name_display.len() > 30 {
                    name_display.truncate(27);
                    name_display.push_str("...");
                }

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
            if !status_message.is_empty() {
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
        let bytes_read = match poll_nonblocking_stdin(&mut buffer) {
            Ok(n) => n,
            Err(_) => 0,
        };

        if bytes_read > 0 {
            status_message.clear();
            let key = &buffer[..bytes_read];

            match key {
                // 'q' or Ctrl+C -> Exit
                b"q" | b"Q" | &[3] => {
                    cancel_flag.store(true, Ordering::SeqCst);
                    let _ = scan_handle.join();
                    break;
                }
                // 'j' or Down arrow (\x1b[B)
                b"j" | b"\x1b[B" => {
                    let items_len = state.lock().unwrap().items.len();
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
                    let items = state.lock().unwrap().items.clone();
                    if let Some(item) = items.get(selected_index) {
                        if item.is_dir && item.path.exists() {
                            cancel_flag.store(true, Ordering::SeqCst);
                            let _ = scan_handle.join();

                            current_dir = item.path.clone();
                            selected_index = 0;
                            scroll_offset = 0;

                            cancel_flag.store(false, Ordering::SeqCst);
                            scan_cancel = Arc::clone(&cancel_flag);
                            scan_state = Arc::clone(&state);
                            let next_dir = current_dir.clone();
                            scan_handle = std::thread::spawn(move || {
                                run_directory_analysis(&next_dir, &scan_cancel, &scan_state);
                            });
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

                        cancel_flag.store(false, Ordering::SeqCst);
                        scan_cancel = Arc::clone(&cancel_flag);
                        scan_state = Arc::clone(&state);
                        let next_dir = current_dir.clone();
                        scan_handle = std::thread::spawn(move || {
                            run_directory_analysis(&next_dir, &scan_cancel, &scan_state);
                        });
                    }
                }
                // 'r' -> Rescan
                b"r" | b"R" => {
                    cancel_flag.store(true, Ordering::SeqCst);
                    let _ = scan_handle.join();
                    cancel_flag.store(false, Ordering::SeqCst);
                    scan_cancel = Arc::clone(&cancel_flag);
                    scan_state = Arc::clone(&state);
                    let cur = current_dir.clone();
                    scan_handle = std::thread::spawn(move || {
                        run_directory_analysis(&cur, &scan_cancel, &scan_state);
                    });
                    status_message = "Rescanning current directory...".to_string();
                }
                // 'd' -> Move to Trash
                b"d" | b"D" => {
                    let items = state.lock().unwrap().items.clone();
                    if let Some(item) = items.get(selected_index) {
                        let path_to_trash = item.path.clone();
                        match trash::delete(&path_to_trash) {
                            Ok(_) => {
                                status_message = format!("Moved to Trash: {}", item.name);
                                // Trigger refresh
                                cancel_flag.store(true, Ordering::SeqCst);
                                let _ = scan_handle.join();
                                cancel_flag.store(false, Ordering::SeqCst);
                                scan_cancel = Arc::clone(&cancel_flag);
                                scan_state = Arc::clone(&state);
                                let cur = current_dir.clone();
                                scan_handle = std::thread::spawn(move || {
                                    run_directory_analysis(&cur, &scan_cancel, &scan_state);
                                });
                            }
                            Err(e) => {
                                status_message = format!("Failed to trash: {}", e);
                            }
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
