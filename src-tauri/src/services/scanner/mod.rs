pub mod engine;
pub mod manager;
pub mod models;

pub use engine::ScanEngine;
pub use manager::ScanManager;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::sync::atomic::AtomicU8;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    use crate::models::scan::{ScanProgress, ScanStatus};
    use crate::services::scanner::engine::STATUS_RUNNING;
    use crate::services::scanner::models::ScanOptions;
    use crate::storage::migrator::Migrator;
    use crate::storage::DbPool;

    #[tokio::test]
    async fn test_synthetic_filesystem_scanner() {
        // 1. Setup temporary database with migrations
        let test_run_id = Uuid::new_v4().to_string();
        let temp_base = std::env::temp_dir().join(format!("diskatlas_test_{}", test_run_id));
        fs::create_dir_all(&temp_base).unwrap();

        let db_path = temp_base.join("test_diskatlas.db");
        let db_pool = DbPool::new(&db_path).unwrap();

        {
            let conn_arc = db_pool.conn();
            let mut conn = conn_arc.lock().unwrap();
            Migrator::run(&mut conn).unwrap();
            conn.execute(
                "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
                 VALUES ('TEST_DRIVE', 'Test Virtual Drive', '/test', 'apfs', 10000000, 5000000);",
                (),
            )
            .unwrap();
        }

        // 2. Create synthetic test directory hierarchy
        let root = temp_base.join("synthetic_root");
        fs::create_dir_all(&root).unwrap();

        // 2.1 Nested folders
        let nested_dir = root.join("level1").join("level2");
        fs::create_dir_all(&nested_dir).unwrap();
        let mut f1 = File::create(nested_dir.join("sample.txt")).unwrap();
        f1.write_all(b"Hello from nested file!").unwrap();

        // 2.2 Hidden files
        let hidden_file = root.join(".hidden_config");
        let mut f2 = File::create(&hidden_file).unwrap();
        f2.write_all(b"secret=true").unwrap();

        // 2.3 Unknown extension
        let custom_ext_file = root.join("artifact.datx9");
        let mut f3 = File::create(&custom_ext_file).unwrap();
        f3.write_all(b"binary payload").unwrap();

        // 2.4 Duplicate content files
        let dup1 = root.join("dup1.bin");
        let dup2 = root.join("dup2.bin");
        let sample_payload = vec![0xAB; 1024];
        File::create(&dup1).unwrap().write_all(&sample_payload).unwrap();
        File::create(&dup2).unwrap().write_all(&sample_payload).unwrap();

        // 2.5 Symlink (if supported on platform)
        #[cfg(unix)]
        {
            let symlink_path = root.join("symlink_to_dup1");
            let _ = std::os::unix::fs::symlink(&dup1, &symlink_path);
        }

        // 3. Run ScanEngine
        let session_id = format!("test-session-{}", test_run_id);
        let options = ScanOptions {
            session_id: session_id.clone(),
            drive_id: "TEST_DRIVE".to_string(),
            root_path: root.clone(),
            exclusions: vec![],
            batch_size: 10,
        };

        let status = Arc::new(AtomicU8::new(STATUS_RUNNING));
        let pause_signal = Arc::new((Mutex::new(()), std::sync::Condvar::new()));
        let progress = Arc::new(Mutex::new(ScanProgress {
            session_id: session_id.clone(),
            drive_id: "TEST_DRIVE".to_string(),
            root_path: root.to_string_lossy().to_string(),
            status: ScanStatus::Running,
            files_scanned: 0,
            dirs_scanned: 0,
            bytes_scanned: 0,
            error_count: 0,
            current_path: String::new(),
            current_file: None,
            recent_files: Vec::new(),
            elapsed_millis: 0,
        }));

        let summary = ScanEngine::run_scan(
            options,
            status,
            pause_signal,
            progress,
            db_pool.clone(),
            None,
        )
        .unwrap();

        assert_eq!(summary.status, ScanStatus::Completed);
        assert!(summary.total_files >= 4, "Should index at least 4 files");
        assert!(summary.total_dirs >= 3, "Should index at least root + 2 nested dirs");
        assert!(summary.total_bytes >= 2048, "Total bytes should exceed sum of files");

        // 4. Verify SQLite database persistence and pagination
        let manager = ScanManager::new();
        let page = manager
            .get_files(Some(&session_id), 1, 10, None, Some("size".into()), Some(true), &db_pool)
            .unwrap();

        assert_eq!(page.total_count, summary.total_files);
        assert!(!page.items.is_empty());

        // Verify hidden file attribute was captured
        let hidden_found = page.items.iter().any(|item| item.is_hidden && item.name == ".hidden_config");
        assert!(hidden_found, "Should have marked .hidden_config as hidden");

        // Verify unknown extension was captured
        let ext_found = page.items.iter().any(|item| item.extension.as_deref() == Some("datx9"));
        assert!(ext_found, "Should have extracted 'datx9' extension");

        // Cleanup temporary test tree
        let _ = fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_scan_manager_live_progress() {
        use std::path::PathBuf;
        use std::thread::sleep;
        use std::time::Duration;

        let test_run_id = Uuid::new_v4().to_string();
        let temp_base = std::env::temp_dir().join(format!("diskatlas_live_{}", test_run_id));
        fs::create_dir_all(&temp_base).unwrap();
        let db_path = temp_base.join("test_live.db");
        let db_pool = DbPool::new(&db_path).unwrap();
        {
            let conn_arc = db_pool.conn();
            let mut conn = conn_arc.lock().unwrap();
            Migrator::run(&mut conn).unwrap();
        }

        let manager = ScanManager::new();
        let session_id = manager.start_scan(
            PathBuf::from("/"),
            "Macintosh HD".to_string(),
            None,
            db_pool.clone(),
            None,
        ).unwrap();

        println!("Started session: {}", session_id);

        for i in 1..=10 {
            sleep(Duration::from_millis(100));
            let p = manager.get_progress(&session_id, &db_pool).unwrap();
            println!("Check #{}: status={:?}, files={}, dirs={}, bytes={}, errors={}, current_path={}",
                i, p.status, p.files_scanned, p.dirs_scanned, p.bytes_scanned, p.error_count, p.current_path);
            if p.files_scanned > 0 || p.dirs_scanned > 0 {
                println!("SUCCESS: Scanned {} files, {} dirs!", p.files_scanned, p.dirs_scanned);
                break;
            }
        }

        manager.cancel_scan(&session_id).unwrap();
        let _ = fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_category_summaries_alias_collision_regression() {
        let test_run_id = Uuid::new_v4().to_string();
        let temp_base = std::env::temp_dir().join(format!("diskatlas_cat_{}", test_run_id));
        fs::create_dir_all(&temp_base).unwrap();
        let db_path = temp_base.join("test_cat.db");
        let db_pool = DbPool::new(&db_path).unwrap();
        let session_id = format!("test-session-{}", test_run_id);
        {
            let conn_arc = db_pool.conn();
            let mut conn = conn_arc.lock().unwrap();
            Migrator::run(&mut conn).unwrap();
            conn.execute(
                "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
                 VALUES ('TEST_DRIVE', 'Test Drive', '/test', 'apfs', 10000000, 5000000);",
                (),
            )
            .unwrap();
            conn.execute(
                "INSERT INTO scan_sessions (id, drive_id, root_path, status)
                 VALUES (?1, 'TEST_DRIVE', '/test', 'running');",
                [&session_id],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO files (session_id, path, name, extension, parent_path, size_bytes, category)
                 VALUES
                 (?1, '/test/a.rs', 'a.rs', 'rs', '/test', 100, 'classifier_code'),
                 (?1, '/test/b.rs', 'b.rs', 'rs', '/test', 200, 'classifier_other'),
                 (?1, '/test/c.png', 'c.png', 'png', '/test', 300, 'classifier_other');",
                [&session_id],
            )
            .unwrap();
        }

        ScanEngine::update_session_status(
            &db_pool,
            &session_id,
            ScanStatus::Completed,
            3,
            1,
            600,
            0,
        )
        .unwrap();

        {
            let conn = db_pool.lock_conn();
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM category_summaries WHERE session_id = ?1;",
                    [&session_id],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 2);
        }

        let _ = fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_synthesize_missing_ancestors_and_root_tree_connectivity() {
        let test_run_id = Uuid::new_v4().to_string();
        let temp_base = std::env::temp_dir().join(format!("diskatlas_tree_synth_{}", test_run_id));
        fs::create_dir_all(&temp_base).unwrap();
        let db_path = temp_base.join("test_tree.db");
        let db_pool = DbPool::new(&db_path).unwrap();
        let session_id = format!("test-tree-{}", test_run_id);

        {
            let mut conn = db_pool.lock_conn();
            crate::storage::migrator::Migrator::run(&mut conn).unwrap();
            conn.execute(
                "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
                 VALUES ('MAC', 'Macintosh HD', '/', 'apfs', 10000000, 5000000);",
                (),
            )
            .unwrap();
            conn.execute(
                "INSERT INTO scan_sessions (id, drive_id, root_path, status)
                 VALUES (?1, 'MAC', '/', 'running');",
                [&session_id],
            )
            .unwrap();
        }

        // Simulate macOS `/` firmlink seeding where `/Users` and `/usr` were never visited directly
        let mut dir_map = std::collections::HashMap::new();
        dir_map.insert(
            "/Users/alice".to_string(),
            crate::services::scanner::models::RawDirItem {
                path: "/Users/alice".to_string(),
                name: "alice".to_string(),
                parent_path: Some("/Users".to_string()),
                file_count: 10,
                dir_count: 2,
                total_size_bytes: 5000,
                created_at: None,
                modified_at: None,
                is_hidden: false,
                is_system: false,
            },
        );
        dir_map.insert(
            "/usr/local".to_string(),
            crate::services::scanner::models::RawDirItem {
                path: "/usr/local".to_string(),
                name: "local".to_string(),
                parent_path: Some("/usr".to_string()),
                file_count: 4,
                dir_count: 1,
                total_size_bytes: 2000,
                created_at: None,
                modified_at: None,
                is_hidden: false,
                is_system: false,
            },
        );
        dir_map.insert(
            "/Applications".to_string(),
            crate::services::scanner::models::RawDirItem {
                path: "/Applications".to_string(),
                name: "Applications".to_string(),
                parent_path: Some("/".to_string()),
                file_count: 3,
                dir_count: 0,
                total_size_bytes: 3000,
                created_at: None,
                modified_at: None,
                is_hidden: false,
                is_system: false,
            },
        );

        ScanEngine::rollup_and_flush_directories(
            &db_pool,
            &session_id,
            &mut dir_map,
            100,
            std::path::Path::new("/"),
        )
        .unwrap();
        ScanEngine::ensure_root_directory_row(
            &db_pool,
            &session_id,
            std::path::Path::new("/"),
            17,
            3,
            10000,
        )
        .unwrap();

        let conn = db_pool.lock_conn();
        let children: Vec<(String, i64)> = {
            let mut stmt = conn
                .prepare("SELECT path, total_size_bytes FROM directories WHERE session_id = ?1 AND parent_path = '/' ORDER BY total_size_bytes DESC")
                .unwrap();
            stmt.query_map([&session_id], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };

        assert_eq!(
            children,
            vec![
                ("/Users".to_string(), 5000),
                ("/Applications".to_string(), 3000),
                ("/usr".to_string(), 2000),
            ]
        );

        let _ = fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_scan_starts_traversal_without_waiting_on_db_write_lock() {
        use std::sync::atomic::Ordering;
        use std::time::Duration;
        use crate::services::scanner::engine::STATUS_CANCELLED;

        let test_run_id = Uuid::new_v4().to_string();
        let temp_base = std::env::temp_dir().join(format!("diskatlas_nonblock_{}", test_run_id));
        let root = temp_base.join("tree");
        fs::create_dir_all(root.join("sub")).unwrap();
        File::create(root.join("alpha.txt")).unwrap().write_all(b"12345").unwrap();
        File::create(root.join("sub/beta.log")).unwrap().write_all(b"67890").unwrap();

        let db_path = temp_base.join("nonblock.db");
        let db_pool = DbPool::new(&db_path).unwrap();
        {
            let mut conn = db_pool.lock_conn();
            Migrator::run(&mut conn).unwrap();
        }

        // Simulate a long-running background DB write holding `lock_conn()` across scan launch.
        let held_write_lock = db_pool.lock_conn();

        let session_id = format!("nonblock-{}", test_run_id);
        let options = ScanOptions {
            session_id: session_id.clone(),
            drive_id: "TEST_DRIVE".to_string(),
            root_path: root.clone(),
            exclusions: vec![],
            batch_size: 2500,
        };
        let status = Arc::new(AtomicU8::new(STATUS_RUNNING));
        let pause_signal = Arc::new((Mutex::new(()), std::sync::Condvar::new()));
        let progress = Arc::new(Mutex::new(ScanProgress {
            session_id: session_id.clone(),
            drive_id: "TEST_DRIVE".to_string(),
            root_path: root.to_string_lossy().to_string(),
            status: ScanStatus::Running,
            files_scanned: 0,
            dirs_scanned: 0,
            bytes_scanned: 0,
            error_count: 0,
            current_path: root.to_string_lossy().to_string(),
            current_file: None,
            recent_files: Vec::new(),
            elapsed_millis: 0,
        }));

        let status_clone = Arc::clone(&status);
        let pause_clone = Arc::clone(&pause_signal);
        let progress_clone = Arc::clone(&progress);
        let pool_clone = db_pool.clone();
        let join = std::thread::spawn(move || {
            ScanEngine::run_scan(
                options,
                status_clone,
                pause_clone,
                progress_clone,
                pool_clone,
                None,
            )
        });

        // While the DB write lock is STILL held, filesystem traversal must discover files immediately.
        let mut observed_files = 0;
        let mut observed_recent = Vec::new();
        for _ in 0..50 {
            {
                let p = progress.lock().unwrap();
                observed_files = p.files_scanned;
                observed_recent = p.recent_files.clone();
                if observed_files >= 2 {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(
            observed_files >= 2,
            "Traversal stalled waiting on DB write lock (files_scanned = {})",
            observed_files
        );
        assert!(
            observed_recent.iter().any(|p| p.ends_with("alpha.txt") || p.ends_with("beta.log")),
            "recent_files should contain real files, got {:?}",
            observed_recent
        );

        // Request cancellation while lock is held, then release the lock so final flush completes cleanly.
        status.store(STATUS_CANCELLED, Ordering::SeqCst);
        drop(held_write_lock);

        let summary = join.join().unwrap().unwrap();
        assert_eq!(summary.status, ScanStatus::Cancelled);
        assert!(summary.total_files >= 2);

        let manager = ScanManager::new();
        let page = manager
            .get_files(Some(&session_id), 1, 10, None, Some("size".into()), Some(true), &db_pool)
            .unwrap();
        assert_eq!(page.total_count, summary.total_files);

        let _ = fs::remove_dir_all(&temp_base);
    }
}
