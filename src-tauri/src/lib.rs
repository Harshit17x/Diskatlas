pub mod commands;
pub mod error;
pub mod models;
pub mod services;
pub mod storage;

use std::path::PathBuf;
use tauri::Manager;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::commands::{
    classification::{
        add_category_override, classify_session, delete_category_override, get_category_files,
        get_category_overrides, get_storage_analysis,
    },
    database::get_database_health,
    drives::{get_drive_details, get_drives, list_drives},
    history::{
        compare_scans, create_manual_snapshot, delete_scan_session, get_scan_sessions,
        get_session_details, get_storage_snapshots,
    },
    scanner::{cancel_scan, get_scan_files, get_scan_progress, pause_scan, resume_scan, start_scan},
    system::get_system_info,
};
use crate::services::scanner::ScanManager;
use crate::storage::{migrator::Migrator, DbPool};

pub fn run() {
    // 1. Initialize structured logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,diskatlas=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting DiskAtlas Desktop Core...");

    tauri::Builder::default()
        .setup(|app| {
            // 2. Resolve database storage directory with portable fallback
            let primary_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("./.diskatlas_data"));

            let app_data_dir = match std::fs::create_dir_all(&primary_dir) {
                Ok(_) => primary_dir,
                Err(err) => {
                    tracing::warn!(
                        "Could not create primary app data dir '{}': {}. Falling back to local data directory.",
                        primary_dir.display(),
                        err
                    );
                    let fallback = PathBuf::from("./.diskatlas_data");
                    let _ = std::fs::create_dir_all(&fallback);
                    fallback
                }
            };

            let db_path = app_data_dir.join("diskatlas.db");
            info!("Database target path: {}", db_path.display());

            // 3. Initialize SQLite Connection Pool with WAL mode
            let db_pool = DbPool::new(&db_path)?;

            // 4. Run baseline embedded migrations
            {
                let conn_arc = db_pool.conn();
                let mut conn = conn_arc.lock().unwrap();
                let latest_version = Migrator::run(&mut conn)?;
                info!("Database initialized at schema version {}", latest_version);
            }

            // 5. Store DB pool and Scanner Manager into Tauri managed state for command handlers
            let scan_manager = ScanManager::new();
            // Automatically recover any orphaned/interrupted scans from previous unclean shutdowns
            let _ = scan_manager.recover_interrupted_scans(&db_pool);

            app.manage(db_pool);
            app.manage(scan_manager);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            get_drives,
            get_drive_details,
            list_drives,
            get_database_health,
            start_scan,
            pause_scan,
            resume_scan,
            cancel_scan,
            get_scan_progress,
            get_scan_files,
            get_scan_sessions,
            get_session_details,
            delete_scan_session,
            get_storage_snapshots,
            create_manual_snapshot,
            compare_scans,
            get_storage_analysis,
            classify_session,
            get_category_files,
            get_category_overrides,
            add_category_override,
            delete_category_override,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DiskAtlas desktop application");
}
