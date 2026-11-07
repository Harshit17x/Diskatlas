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
        let pause_notify = Arc::new(tokio::sync::Notify::new());
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
            pause_notify,
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
            .get_files(&session_id, 1, 10, None, Some("size".into()), Some(true), &db_pool)
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
}
