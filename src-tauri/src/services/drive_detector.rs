use sysinfo::Disks;
use rusqlite::Connection;
use tracing::info;
use crate::error::Result;
use crate::models::drive::DriveInfo;

pub struct DriveDetector;

impl DriveDetector {
    /// Detects all active, mounted disks on the host system.
    pub fn detect_all() -> Vec<DriveInfo> {
        let disks = Disks::new_with_refreshed_list();
        let mut results = Vec::new();

        for disk in disks.list() {
            let mount_path = disk.mount_point().to_string_lossy().to_string();
            let name = disk.name().to_string_lossy().to_string();
            let label = if name.trim().is_empty() {
                mount_path.clone()
            } else {
                name.clone()
            };

            let fs_type = disk.file_system().to_string_lossy().to_string();
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);
            let used_percent = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };

            let is_removable = disk.is_removable();

            // Detect system/OS boot partition
            let is_system = Self::is_system_drive(&mount_path);

            let id = if !name.is_empty() {
                name
            } else {
                mount_path.clone()
            };

            results.push(DriveInfo {
                id,
                label,
                mount_point: mount_path,
                file_system: if fs_type.is_empty() { "Unknown".into() } else { fs_type },
                total_bytes: total,
                available_bytes: available,
                used_bytes: used,
                used_percent,
                is_removable,
                is_system,
            });
        }

        // Sort: System drive first, then by total capacity descending
        results.sort_by(|a, b| {
            b.is_system
                .cmp(&a.is_system)
                .then_with(|| b.total_bytes.cmp(&a.total_bytes))
        });

        results
    }

    /// Syncs detected drives into SQLite drives table
    pub fn sync_to_db(conn: &mut Connection, drives: &[DriveInfo]) -> Result<()> {
        let tx = conn.transaction()?;
        for drive in drives {
            tx.execute(
                "INSERT INTO drives (
                    id, label, mount_point, filesystem, total_bytes, available_bytes, is_removable, is_system
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label,
                    mount_point = excluded.mount_point,
                    filesystem = excluded.filesystem,
                    total_bytes = excluded.total_bytes,
                    available_bytes = excluded.available_bytes,
                    is_removable = excluded.is_removable,
                    is_system = excluded.is_system;",
                (
                    &drive.id,
                    &drive.label,
                    &drive.mount_point,
                    &drive.file_system,
                    drive.total_bytes,
                    drive.available_bytes,
                    drive.is_removable as i32,
                    drive.is_system as i32,
                ),
            )?;
        }
        tx.commit()?;
        info!("Synchronized {} drives to SQLite database", drives.len());
        Ok(())
    }

    fn is_system_drive(mount_point: &str) -> bool {
        #[cfg(target_os = "windows")]
        {
            let sys_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
            mount_point.starts_with(&sys_drive)
        }

        #[cfg(not(target_os = "windows"))]
        {
            mount_point == "/"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_detection_does_not_panic() {
        let drives = DriveDetector::detect_all();
        // Almost all running systems (macOS, Windows, Linux) have at least 1 mounted filesystem
        assert!(!drives.is_empty(), "Should discover at least one mounted volume");
    }
}

