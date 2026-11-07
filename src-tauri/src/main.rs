// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--tui" || arg == "-t") {
        let path = args
            .iter()
            .skip(1)
            .find(|arg| !arg.starts_with('-'))
            .map(std::path::PathBuf::from);
        diskatlas_lib::run_tui(path);
    } else {
        diskatlas_lib::run();
    }
}

