use rusqlite::Connection;
use tracing::info;
use crate::error::{AppError, Result};

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "0001_initial_schema",
        sql: include_str!("../../migrations/0001_initial_schema.sql"),
    },
    Migration {
        version: 2,
        name: "0002_files_and_directories",
        sql: include_str!("../../migrations/0002_files_and_directories.sql"),
    },
    Migration {
        version: 3,
        name: "0003_storage_history_and_snapshots",
        sql: include_str!("../../migrations/0003_storage_history_and_snapshots.sql"),
    },
];

pub struct Migrator;

impl Migrator {
    /// Runs all pending embedded migrations sequentially inside transactions.
    pub fn run(conn: &mut Connection) -> Result<i64> {
        // Ensure migration tracking table exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS _migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
            [],
        )?;

        // Determine current applied version
        let current_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        info!("Current database migration version: {}", current_version);

        for migration in MIGRATIONS {
            if migration.version > current_version {
                info!(
                    "Applying migration {}: '{}'...",
                    migration.version, migration.name
                );

                let tx = conn.transaction()?;
                tx.execute_batch(migration.sql).map_err(|e| {
                    AppError::Migration(format!(
                        "Failed executing migration {} ('{}'): {}",
                        migration.version, migration.name, e
                    ))
                })?;

                tx.execute(
                    "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
                    (migration.version, migration.name),
                )?;

                tx.commit()?;
                info!("Migration {} successfully applied", migration.version);
            }
        }

        // Return latest version
        let latest_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(latest_version)
    }

    /// Retrieve the current migration version without running new migrations
    pub fn current_version(conn: &Connection) -> Result<i64> {
        let exists: bool = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = '_migrations'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !exists {
            return Ok(0);
        }

        let version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_migrations_apply_cleanly_on_empty_db() {
        let mut conn = Connection::open_in_memory().unwrap();
        let version = Migrator::run(&mut conn).unwrap();
        assert_eq!(version, 3);

        // Verify tables were created
        let tables_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN (
                    'drives', 'scan_sessions', 'app_settings', 'files', 'directories',
                    'scan_errors', '_migrations', 'storage_snapshots', 'category_summaries'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables_count, 9);
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        let v1 = Migrator::run(&mut conn).unwrap();
        let v2 = Migrator::run(&mut conn).unwrap();
        assert_eq!(v1, v2);
    }
}

