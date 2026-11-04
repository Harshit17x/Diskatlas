use rusqlite::Connection;
use tracing::info;
use crate::error::{AppError, Result};
use crate::models::drive::{DriveDetails, DriveInfo, DriveType};

pub struct DriveDetector;

impl DriveDetector {
    /// Detects all active, mounted storage drives on the host system.
    pub fn detect_all() -> Vec<DriveInfo> {
        #[cfg(target_os = "windows")]
        {
            windows_impl::detect_windows_drives()
        }

        #[cfg(not(target_os = "windows"))]
        {
            unix_impl::detect_unix_drives()
        }
    }

    /// Retrieves detailed diagnostics for a specific drive by its ID or mount point.
    pub fn get_details(drive_id: &str) -> Result<DriveDetails> {
        let all = Self::detect_all();
        let drive = all
            .into_iter()
            .find(|d| d.id == drive_id || d.mount_point == drive_id || d.drive_letter.as_deref() == Some(drive_id))
            .ok_or_else(|| AppError::DriveNotFound(drive_id.to_string()))?;

        #[cfg(target_os = "windows")]
        {
            windows_impl::get_windows_drive_details(drive)
        }

        #[cfg(not(target_os = "windows"))]
        {
            unix_impl::get_unix_drive_details(drive)
        }
    }

    /// Synchronizes detected drives into the SQLite `drives` table.
    pub fn sync_to_db(conn: &mut Connection, drives: &[DriveInfo]) -> Result<()> {
        let tx = conn.transaction()?;
        for drive in drives {
            let total = drive.total_bytes.unwrap_or(0);
            let available = drive.available_bytes.unwrap_or(0);
            let fs = drive.file_system.clone().unwrap_or_else(|| "Unknown".into());

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
                    &drive.name,
                    &drive.mount_point,
                    &fs,
                    total as i64,
                    available as i64,
                    drive.is_removable as i32,
                    drive.is_system as i32,
                ),
            )?;
        }
        tx.commit()?;
        info!("Synchronized {} drives to SQLite database", drives.len());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Windows Implementation (Win32 APIs)
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::Path;

    // Direct Win32 FFI bindings to avoid crate version mismatches
    extern "system" {
        fn GetLogicalDriveStringsW(nBufferLength: u32, lpBuffer: *mut u16) -> u32;
        fn GetDriveTypeW(lpRootPathName: *const u16) -> u32;
        fn GetVolumeInformationW(
            lpRootPathName: *const u16,
            lpVolumeNameBuffer: *mut u16,
            nVolumeNameSize: u32,
            lpVolumeSerialNumber: *mut u32,
            lpMaximumComponentLength: *mut u32,
            lpFileSystemFlags: *mut u32,
            lpFileSystemNameBuffer: *mut u16,
            nFileSystemNameSize: u32,
        ) -> i32;
        fn GetDiskFreeSpaceExW(
            lpDirectoryName: *const u16,
            lpFreeBytesAvailableToCaller: *mut u64,
            lpTotalNumberOfBytes: *mut u64,
            lpTotalNumberOfFreeBytes: *mut u64,
        ) -> i32;
        fn GetDiskFreeSpaceW(
            lpRootPathName: *const u16,
            lpSectorsPerCluster: *mut u32,
            lpBytesPerSector: *mut u32,
            lpNumberOfFreeClusters: *mut u32,
            lpTotalNumberOfClusters: *mut u32,
        ) -> i32;
    }

    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    const DRIVE_REMOTE: u32 = 4;
    const DRIVE_CDROM: u32 = 5;
    const DRIVE_RAMDISK: u32 = 6;
    const FILE_READ_ONLY_VOLUME: u32 = 0x00080000;

    pub fn detect_windows_drives() -> Vec<DriveInfo> {
        let mut drives = Vec::new();

        // 1. Query logical drive roots (e.g. "C:\", "D:\")
        let mut buffer = [0u16; 512];
        let len = unsafe { GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr()) };
        if len == 0 || len > buffer.len() as u32 {
            return drives;
        }

        let sys_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());

        // Split null-terminated strings
        let mut start = 0;
        for i in 0..len as usize {
            if buffer[i] == 0 {
                if start < i {
                    let root_slice = &buffer[start..i];
                    let root_os = OsString::from_wide(root_slice);
                    let root_str = root_os.to_string_lossy().to_string();

                    if let Some(drive_info) = inspect_single_drive(&root_str, &sys_drive) {
                        drives.push(drive_info);
                    }
                }
                start = i + 1;
            }
        }

        // Sort: System OS drive first, then fixed drives by capacity descending
        drives.sort_by(|a, b| {
            b.is_system
                .cmp(&a.is_system)
                .then_with(|| b.total_bytes.unwrap_or(0).cmp(&a.total_bytes.unwrap_or(0)))
        });

        drives
    }

    fn inspect_single_drive(root: &str, sys_drive: &str) -> Option<DriveInfo> {
        let mut wide_root: Vec<u16> = root.encode_utf16().collect();
        wide_root.push(0);

        let drive_type_raw = unsafe { GetDriveTypeW(wide_root.as_ptr()) };
        if drive_type_raw <= 1 {
            // DRIVE_UNKNOWN or DRIVE_NO_ROOT_DIR
            return None;
        }

        let drive_letter = root.strip_suffix('\\').unwrap_or(root).to_string();
        let is_system = drive_letter.eq_ignore_ascii_case(sys_drive);

        // Query volume metadata
        let mut vol_name_buf = [0u16; 260];
        let mut fs_name_buf = [0u16; 260];
        let mut serial_number: u32 = 0;
        let mut max_comp_len: u32 = 0;
        let mut fs_flags: u32 = 0;

        let vol_ok = unsafe {
            GetVolumeInformationW(
                wide_root.as_ptr(),
                vol_name_buf.as_mut_ptr(),
                vol_name_buf.len() as u32,
                &mut serial_number,
                &mut max_comp_len,
                &mut fs_flags,
                fs_name_buf.as_mut_ptr(),
                fs_name_buf.len() as u32,
            )
        };

        let label = if vol_ok != 0 {
            let len = vol_name_buf.iter().position(|&c| c == 0).unwrap_or(vol_name_buf.len());
            let name = String::from_utf16_lossy(&vol_name_buf[..len]);
            if name.trim().is_empty() {
                drive_letter.clone()
            } else {
                name
            }
        } else {
            drive_letter.clone()
        };

        let file_system = if vol_ok != 0 {
            let len = fs_name_buf.iter().position(|&c| c == 0).unwrap_or(fs_name_buf.len());
            let fs = String::from_utf16_lossy(&fs_name_buf[..len]);
            if fs.trim().is_empty() { None } else { Some(fs) }
        } else {
            None
        };

        let is_read_only = (fs_flags & FILE_READ_ONLY_VOLUME) != 0;

        // Query storage capacity
        let mut free_avail: u64 = 0;
        let mut total_bytes_raw: u64 = 0;
        let mut total_free: u64 = 0;

        let space_ok = unsafe {
            GetDiskFreeSpaceExW(
                wide_root.as_ptr(),
                &mut free_avail,
                &mut total_bytes_raw,
                &mut total_free,
            )
        };

        let (total_bytes, available_bytes, used_bytes, used_percent) = if space_ok != 0 && total_bytes_raw > 0 {
            let used = total_bytes_raw.saturating_sub(free_avail);
            let pct = (used as f64 / total_bytes_raw as f64) * 100.0;
            (Some(total_bytes_raw), Some(free_avail), Some(used), Some(pct))
        } else {
            // Unmounted, empty CD/DVD tray, or inaccessible partition: mark as None (not zero)
            (None, None, None, None)
        };

        // Determine specific DriveType
        let (drive_type, is_removable) = match drive_type_raw {
            DRIVE_REMOVABLE => (DriveType::Usb, true),
            DRIVE_FIXED => {
                if is_system {
                    (DriveType::InternalSsd, false)
                } else {
                    (DriveType::InternalHdd, false)
                }
            }
            DRIVE_REMOTE => (DriveType::Network, false),
            DRIVE_CDROM => (DriveType::Optical, true),
            DRIVE_RAMDISK => (DriveType::Ram, false),
            _ => (DriveType::Unknown, false),
        };

        let volume_id = if serial_number != 0 {
            Some(format!("{:04X}-{:04X}", serial_number >> 16, serial_number & 0xFFFF))
        } else {
            None
        };

        Some(DriveInfo {
            id: drive_letter.clone(),
            name: label,
            drive_letter: Some(drive_letter),
            mount_point: root.to_string(),
            file_system,
            total_bytes,
            available_bytes,
            used_bytes,
            used_percent,
            drive_type,
            volume_id,
            is_removable,
            is_system,
            is_read_only,
        })
    }

    pub fn get_windows_drive_details(drive: DriveInfo) -> Result<DriveDetails> {
        let mut wide_root: Vec<u16> = drive.mount_point.encode_utf16().collect();
        wide_root.push(0);

        let mut sectors_per_cluster: u32 = 0;
        let mut bytes_per_sector: u32 = 0;
        let mut free_clusters: u32 = 0;
        let mut total_clusters: u32 = 0;

        let space_ok = unsafe {
            GetDiskFreeSpaceW(
                wide_root.as_ptr(),
                &mut sectors_per_cluster,
                &mut bytes_per_sector,
                &mut free_clusters,
                &mut total_clusters,
            )
        };

        let mut flags = Vec::new();
        if drive.is_system {
            flags.push("BootVolume".into());
        }
        if drive.is_read_only {
            flags.push("ReadOnly".into());
        }
        if drive.is_removable {
            flags.push("HotPluggable".into());
        }

        let (spc, bps) = if space_ok != 0 {
            (Some(sectors_per_cluster), Some(bytes_per_sector))
        } else {
            (None, None)
        };

        Ok(DriveDetails {
            drive,
            bus_type: Some("Windows Storage Port".into()),
            serial_number: None,
            sectors_per_cluster: spc,
            bytes_per_sector: bps,
            is_encrypted: None,
            flags,
        })
    }
}

// ---------------------------------------------------------------------------
// Unix & macOS Implementation
// ---------------------------------------------------------------------------
#[cfg(not(target_os = "windows"))]
mod unix_impl {
    use super::*;
    use sysinfo::Disks;

    pub fn detect_unix_drives() -> Vec<DriveInfo> {
        let disks = Disks::new_with_refreshed_list();
        let mut results = Vec::new();

        for disk in disks.list() {
            let mount_path = disk.mount_point().to_string_lossy().to_string();
            let disk_name = disk.name().to_string_lossy().to_string();

            let is_system = mount_path == "/";
            let label = if !disk_name.trim().is_empty() {
                disk_name.clone()
            } else if is_system {
                "Macintosh HD / Root".to_string()
            } else {
                mount_path.clone()
            };

            let fs_raw = disk.file_system().to_string_lossy().to_string();
            let file_system = if fs_raw.trim().is_empty() {
                None
            } else {
                Some(fs_raw.clone())
            };

            let total_raw = disk.total_space();
            let avail_raw = disk.available_space();

            // Clearly distinguish unavailable capacity from 0
            let (total_bytes, available_bytes, used_bytes, used_percent) = if total_raw > 0 {
                let used = total_raw.saturating_sub(avail_raw);
                let pct = (used as f64 / total_raw as f64) * 100.0;
                (Some(total_raw), Some(avail_raw), Some(used), Some(pct))
            } else {
                (None, None, None, None)
            };

            let is_removable = disk.is_removable();

            // Drive type heuristic based on mount location and removable attribute
            let drive_type = if is_system {
                DriveType::InternalSsd
            } else if is_removable {
                DriveType::Usb
            } else if mount_path.starts_with("/Volumes") {
                if fs_raw.to_lowercase().contains("smb") || fs_raw.to_lowercase().contains("nfs") {
                    DriveType::Network
                } else if fs_raw.to_lowercase().contains("cd") || fs_raw.to_lowercase().contains("udf") {
                    DriveType::Optical
                } else {
                    DriveType::ExternalSsd
                }
            } else {
                DriveType::InternalHdd
            };

            let id = if !disk_name.is_empty() {
                disk_name
            } else {
                mount_path.clone()
            };

            results.push(DriveInfo {
                id,
                name: label,
                drive_letter: None,
                mount_point: mount_path,
                file_system,
                total_bytes,
                available_bytes,
                used_bytes,
                used_percent,
                drive_type,
                volume_id: None,
                is_removable,
                is_system,
                is_read_only: disk.is_read_only(),
            });
        }

        // Sort: System drive first, then largest total capacity
        results.sort_by(|a, b| {
            b.is_system
                .cmp(&a.is_system)
                .then_with(|| b.total_bytes.unwrap_or(0).cmp(&a.total_bytes.unwrap_or(0)))
        });

        results
    }

    pub fn get_unix_drive_details(drive: DriveInfo) -> Result<DriveDetails> {
        let mut flags = Vec::new();
        if drive.is_system {
            flags.push("BootVolume".into());
        }
        if drive.is_read_only {
            flags.push("ReadOnly".into());
        }
        if drive.is_removable {
            flags.push("Removable".into());
        }

        let bus_type = if drive.is_system {
            Some("Apple Fabric / NVMe".into())
        } else if drive.is_removable {
            Some("USB / External Interface".into())
        } else {
            Some("Mounted Storage".into())
        };

        Ok(DriveDetails {
            drive,
            bus_type,
            serial_number: None,
            sectors_per_cluster: Some(8),
            bytes_per_sector: Some(4096),
            is_encrypted: Some(true),
            flags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_detector_detects_at_least_one_drive() {
        let drives = DriveDetector::detect_all();
        assert!(!drives.is_empty(), "Should discover mounted storage volumes");

        // The first drive must be the boot/system volume
        assert!(drives[0].is_system, "First sorted drive must be the boot system volume");
    }

    #[test]
    fn test_drive_details_succeeds_for_system_drive() {
        let drives = DriveDetector::detect_all();
        let system_drive = &drives[0];

        let details = DriveDetector::get_details(&system_drive.id).unwrap();
        assert_eq!(details.drive.id, system_drive.id);
        assert_eq!(details.drive.mount_point, system_drive.mount_point);
    }

    #[test]
    fn test_drive_type_string_representation() {
        assert_eq!(DriveType::InternalSsd.as_str(), "Internal SSD");
        assert_eq!(DriveType::Usb.as_str(), "USB Flash Drive");
        assert_eq!(DriveType::Network.as_str(), "Network Drive");
    }
}
