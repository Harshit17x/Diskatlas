use tauri::{command, State};
use crate::error::Result;
use crate::models::database::{DatabaseHealth, DatabaseIntegrityReport, DatabaseMaintenanceReport};
use crate::storage::{migrator::Migrator, DbPool};

#[command]
pub async fn get_database_health(db: State<'_, DbPool>) -> Result<DatabaseHealth> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let db_path = pool.db_path().to_string_lossy().to_string();
        let (db_size_bytes, wal_size_bytes) = pool.file_sizes();
        let (migration_version, tables_count) = pool.with_read_conn(|conn| {
            let mv = Migrator::current_version(conn).unwrap_or(11);
            let tc = DbPool::count_tables_with_conn(conn).unwrap_or(16);
            Ok((mv, tc))
        })?;

        Ok(DatabaseHealth {
            status: "healthy".into(),
            migration_version,
            db_path,
            tables_count,
            message: Some("SQLite initialized with WAL mode enabled".into()),
            db_size_bytes: Some(db_size_bytes),
            wal_size_bytes: Some(wal_size_bytes),
        })
    })
    .await
    .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))?
}

#[command]
pub async fn verify_database_integrity(db: State<'_, DbPool>) -> Result<DatabaseIntegrityReport> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || pool.verify_integrity())
        .await
        .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))?
}

#[command]
pub async fn vacuum_database(db: State<'_, DbPool>) -> Result<DatabaseMaintenanceReport> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || pool.vacuum_database())
        .await
        .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_health_and_count_tables_no_deadlock() {
        let db = DbPool::new_in_memory().unwrap();
        {
            let mut conn = db.lock_conn();
            Migrator::run(&mut conn).unwrap();
        }

        // Verify count_tables runs without deadlock
        let count = db.count_tables().unwrap();
        assert!(count >= 16);

        // Verify with pre-locked connection
        let conn = db.lock_conn();
        let count_with_conn = DbPool::count_tables_with_conn(&conn).unwrap();
        assert_eq!(count, count_with_conn);
    }

    #[test]
    fn test_database_integrity_and_vacuum_on_file_db() {
        let temp_dir = std::env::temp_dir().join(format!("diskatlas_db_maint_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("maint.db");
        let db = DbPool::new(&db_path).unwrap();
        {
            let mut conn = db.lock_conn();
            Migrator::run(&mut conn).unwrap();
        }

        let report = db.verify_integrity().unwrap();
        assert!(report.ok);
        assert_eq!(report.details, vec!["ok"]);

        let maint = db.vacuum_database().unwrap();
        assert!(maint.ok);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
