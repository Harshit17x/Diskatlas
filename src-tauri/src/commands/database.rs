use tauri::{command, State};
use crate::error::Result;
use crate::models::database::DatabaseHealth;
use crate::storage::{migrator::Migrator, DbPool};

#[command]
pub async fn get_database_health(db: State<'_, DbPool>) -> Result<DatabaseHealth> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();

    let migration_version = Migrator::current_version(&conn)?;
    let tables_count = db.count_tables()?;
    let db_path = db.db_path().to_string_lossy().to_string();

    Ok(DatabaseHealth {
        status: "healthy".into(),
        migration_version,
        db_path,
        tables_count,
        message: Some("SQLite initialized with WAL mode enabled".into()),
    })
}

