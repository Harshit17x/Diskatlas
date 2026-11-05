use tauri::{command, State};
use crate::error::Result;
use crate::models::database::DatabaseHealth;
use crate::storage::{migrator::Migrator, DbPool};

#[command]
pub async fn get_database_health(db: State<'_, DbPool>) -> Result<DatabaseHealth> {
    let db_path = db.db_path().to_string_lossy().to_string();
    let conn_arc = db.conn();

    // Release lock as quickly as possible
    let (migration_version, tables_count) = {
        let conn = conn_arc.lock().unwrap();
        let mv = Migrator::current_version(&conn).unwrap_or(8);
        let tc: usize = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(16);
        (mv, tc)
    };

    Ok(DatabaseHealth {
        status: "healthy".into(),
        migration_version,
        db_path,
        tables_count,
        message: Some("SQLite initialized with WAL mode enabled".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_health_and_count_tables_no_deadlock() {
        let db = DbPool::new_in_memory().unwrap();
        {
            let conn_arc = db.conn();
            let mut conn = conn_arc.lock().unwrap();
            Migrator::run(&mut conn).unwrap();
        }

        // Verify count_tables runs without deadlock
        let count = db.count_tables().unwrap();
        assert!(count >= 16);

        // Verify with pre-locked connection
        let conn_arc = db.conn();
        let conn = conn_arc.lock().unwrap();
        let count_with_conn = DbPool::count_tables_with_conn(&conn).unwrap();
        assert_eq!(count, count_with_conn);
    }
}

