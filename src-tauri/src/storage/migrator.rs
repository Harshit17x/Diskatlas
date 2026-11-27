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
    Migration {
        version: 4,
        name: "0004_storage_classification",
        sql: include_str!("../../migrations/0004_storage_classification.sql"),
    },
    Migration {
        version: 5,
        name: "0005_developer_storage",
        sql: include_str!("../../migrations/0005_developer_storage.sql"),
    },
    Migration {
        version: 6,
        name: "0006_advanced_analysis",
        sql: include_str!("../../migrations/0006_advanced_analysis.sql"),
    },
    Migration {
        version: 7,
        name: "0007_storage_intelligence",
        sql: include_str!("../../migrations/0007_storage_intelligence.sql"),
    },
    Migration {
        version: 8,
        name: "0008_cleanup_audit_history",
        sql: include_str!("../../migrations/0008_cleanup_audit_history.sql"),
    },
    Migration {
        version: 9,
        name: "0009_rebuild_category_summaries",
        sql: include_str!("../../migrations/0009_rebuild_category_summaries.sql"),
    },
    Migration {
        version: 10,
        name: "0010_integrity_and_repair",
        sql: include_str!("../../migrations/0010_integrity_and_repair.sql"),
    },
    Migration {
        version: 11,
        name: "0011_normalize_summaries_and_indexes",
        sql: include_str!("../../migrations/0011_normalize_summaries_and_indexes.sql"),
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

        // Large pre-existing databases (>50k rows) can hold tens of millions of file/dir rows.
        // Heavy historical backfill/reindexing runs asynchronously in `DbPool::run_startup_maintenance`
        // after app startup so the desktop window always opens immediately.
        let is_large_existing_db = Self::table_max_id(conn, "files") >= 50_000
            || Self::table_max_id(conn, "directories") >= 50_000;

        for migration in MIGRATIONS {
            if migration.version > current_version {
                info!(
                    "Applying migration {}: '{}'...",
                    migration.version, migration.name
                );

                let tx = conn.transaction()?;
                if is_large_existing_db && migration.version >= 9 {
                    Self::apply_lightweight_large_db_migration(&tx, migration.version)?;
                } else {
                    tx.execute_batch(migration.sql).map_err(|e| {
                        AppError::Migration(format!(
                            "Failed executing migration {} ('{}'): {}",
                            migration.version, migration.name, e
                        ))
                    })?;
                }

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

    fn table_max_id(conn: &Connection, table: &str) -> i64 {
        let sql = format!("SELECT COALESCE(MAX(id), 0) FROM {}", table);
        conn.query_row(&sql, [], |row| row.get(0)).unwrap_or(0)
    }

    /// Defers heavy multi-million-row index builds / orphan purges / summary rebuilds to
    /// background startup maintenance while keeping lightweight DDL & schema versioning instant.
    fn apply_lightweight_large_db_migration(tx: &Connection, version: i64) -> Result<()> {
        match version {
            9 => {
                tx.execute(
                    "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('maintenance:summaries_pending', '1')",
                    [],
                )?;
            }
            10 => {
                tx.execute_batch(
                    "CREATE INDEX IF NOT EXISTS idx_category_summaries_unique
                         ON category_summaries(session_id, category);
                     CREATE INDEX IF NOT EXISTS idx_class_summaries_category
                         ON classification_summaries(session_id, category);
                     INSERT OR REPLACE INTO app_settings (key, value) VALUES ('maintenance:dirs_unique_pending', '1');
                     INSERT OR REPLACE INTO app_settings (key, value) VALUES ('maintenance:orphans_pending', '1');
                     INSERT OR REPLACE INTO app_settings (key, value) VALUES ('maintenance:summaries_pending', '1');",
                )?;
            }
            11 => {
                tx.execute(
                    "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('maintenance:summaries_pending', '1')",
                    [],
                )?;
            }
            _ => {}
        }
        Ok(())
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
        assert_eq!(version, 11);

        // Verify tables were created
        let tables_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN (
                    'drives', 'scan_sessions', 'app_settings', 'files', 'directories',
                    'scan_errors', '_migrations', 'storage_snapshots', 'category_summaries',
                    'category_overrides', 'classification_summaries', 'developer_projects',
                    'developer_ecosystem_summaries', 'file_hashes', 'storage_recommendations',
                    'cleanup_audit_events'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables_count, 16);
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        let v1 = Migrator::run(&mut conn).unwrap();
        let v2 = Migrator::run(&mut conn).unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_large_db_defers_heavy_migrations_9_to_11() {
        let mut conn = Connection::open_in_memory().unwrap();
        for migration in &MIGRATIONS[..8] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
                (migration.version, migration.name),
            )
            .unwrap();
        }

        conn.execute(
            "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
             VALUES ('drv-1', 'Main', '/tmp', 'APFS', 1000, 500)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO scan_sessions (id, drive_id, root_path, status, total_files)
             VALUES ('sess-large', 'drv-1', '/tmp', 'completed', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (id, session_id, path, name, parent_path, size_bytes, category)
             VALUES (50000, 'sess-large', '/tmp/a.rs', 'a.rs', '/tmp', 2048, 'developer')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO category_summaries (session_id, category, file_count, total_bytes)
             VALUES ('sess-large', 'code', 1, 2048)",
            [],
        )
        .unwrap();

        let version = Migrator::run(&mut conn).unwrap();
        assert_eq!(version, 11);

        let flag = |k: &str| -> String {
            conn.query_row("SELECT value FROM app_settings WHERE key = ?1", [k], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(flag("maintenance:dirs_unique_pending"), "1");
        assert_eq!(flag("maintenance:orphans_pending"), "1");
        assert_eq!(flag("maintenance:summaries_pending"), "1");

        let unique_dir_idx_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'idx_dirs_session_path_unique')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            !unique_dir_idx_exists,
            "idx_dirs_session_path_unique should be deferred on large DBs"
        );

        let cat_sum_idx_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'idx_category_summaries_unique')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(cat_sum_idx_exists);
    }
}

