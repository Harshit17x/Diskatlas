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
    database::get_database_health,
    drives::{get_drive_details, get_drives, list_drives},
    system::get_system_info,
};
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

            // 5. Store DB pool into Tauri managed state for command handlers
            app.manage(db_pool);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            get_drives,
            get_drive_details,
            list_drives,
            get_database_health,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DiskAtlas desktop application");
}
