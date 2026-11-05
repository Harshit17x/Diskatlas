use std::path::{Path, PathBuf};

/// Inspects whether a path targets an essential operating system, boot, driver,
/// or recovery component that must NEVER be modified or deleted.
pub fn is_system_critical_path(path: &Path) -> (bool, Option<&'static str>) {
    let path_str = path.to_string_lossy();
    let lower = path_str.to_lowercase().replace('/', "\\");

    // 1. Filesystem & Drive roots
    if lower == "c:\\" || lower == "c:" || lower == "\\" || lower == "/" {
        return (true, Some("Root drive / filesystem root cannot be deleted"));
    }
    if lower.len() <= 3 && lower.ends_with(":\\") {
        return (true, Some("Volume drive root cannot be deleted"));
    }

    // 2. Windows Boot Files & Loaders
    let boot_files = [
        "bootmgr",
        "boot.ini",
        "bcd",
        "bootmgfw.efi",
        "ntoskrnl.exe",
        "hal.dll",
        "winload.exe",
        "winload.efi",
        "bootstat.dat",
        "memtest.exe",
    ];
    for bf in boot_files {
        if lower.ends_with(&format!("\\{}", bf)) || lower.ends_with(&format!("/{}", bf)) {
            return (true, Some("Critical Windows boot loader or kernel component"));
        }
    }

    // 3. Windows Kernel Memory, Hibernation & Swap Files
    let swap_files = [
        "pagefile.sys",
        "hiberfil.sys",
        "swapfile.sys",
        "dumpstack.log",
        "memory.dmp",
    ];
    for sf in swap_files {
        if lower.ends_with(&format!("\\{}", sf)) || lower.ends_with(&format!("/{}", sf)) {
            return (true, Some("Active operating system memory, page, or hibernation image"));
        }
    }

    // 4. Windows Core OS & Driver Directories
    if lower.contains("\\windows\\system32\\drivers") {
        return (true, Some("Critical hardware driver subsystem directory"));
    }
    if lower.ends_with(".sys") && lower.contains("\\windows\\") {
        return (true, Some("Operating system kernel driver file (.sys)"));
    }
    if lower.contains("\\windows\\winsxs") {
        return (true, Some("Windows Component Store (WinSxS) side-by-side assembly"));
    }
    if lower.contains("\\windows\\servicing") {
        return (true, Some("Windows Update & Servicing infrastructure"));
    }
    if lower == "c:\\windows" || lower == "c:\\windows\\" || lower == "\\windows" {
        return (true, Some("Windows operating system root directory"));
    }
    if lower == "c:\\windows\\system32" || lower == "c:\\windows\\system32\\" {
        return (true, Some("Windows System32 core execution environment"));
    }
    if lower == "c:\\windows\\syswow64" || lower == "c:\\windows\\syswow64\\" {
        return (true, Some("Windows SysWOW64 32-bit execution subsystem"));
    }

    // 5. Recovery & Volume Infrastructure
    if lower.contains("\\system volume information") || lower.contains("/system volume information") {
        return (true, Some("System Volume Information containing VSS restore points"));
    }
    if lower.contains("\\recovery\\") || lower.ends_with("winre.wim") {
        return (true, Some("Windows Recovery Environment (WinRE)"));
    }
    if lower == "c:\\$recycle.bin" || lower == "c:\\$recycle.bin\\" {
        return (true, Some("Volume Recycle Bin root"));
    }

    // 6. Unix / macOS Critical System Directories
    let unix_critical_roots = [
        "/system",
        "/system/library",
        "/usr",
        "/usr/bin",
        "/usr/sbin",
        "/bin",
        "/sbin",
        "/etc",
        "/dev",
        "/proc",
        "/sys",
        "/private/var",
        "/library/extensions",
        "/library/apple",
    ];
    let normalized_unix = path_str.replace('\\', "/").to_lowercase();
    for ucr in unix_critical_roots {
        if normalized_unix == ucr || normalized_unix == format!("{}/", ucr) || normalized_unix.starts_with(&format!("{}/", ucr)) {
            return (true, Some("Protected Unix / macOS core operating system path"));
        }
    }

    (false, None)
}

/// Detects whether a path resides inside a known cloud storage synchronized folder
/// (e.g. OneDrive, Google Drive, Dropbox, iCloud) where deletions propagate across devices.
pub fn detect_cloud_sync(path: &Path) -> (bool, Option<String>) {
    let path_str = path.to_string_lossy();
    let lower = path_str.to_lowercase();

    if lower.contains("onedrive") || lower.contains("skydrive") {
        return (
            true,
            Some("Item is in a OneDrive folder. Deleting this will propagate deletion to your cloud storage and synchronized devices.".to_string())
        );
    }
    if lower.contains("google drive") || lower.contains("googledrive") {
        return (
            true,
            Some("Item is in a Google Drive folder. Deleting this will sync deletion to Google Drive and other synced clients.".to_string())
        );
    }
    if lower.contains("dropbox") {
        return (
            true,
            Some("Item is in a Dropbox folder. Deleting will immediately update cloud storage and remove it from linked devices.".to_string())
        );
    }
    if lower.contains("icloud") || lower.contains("mobile documents") || lower.contains("com~apple~clouddocs") {
        return (
            true,
            Some("Item is stored in iCloud Drive. Deletion will be synchronized to all Apple devices using this account.".to_string())
        );
    }
    if lower.contains("nextcloud") {
        return (
            true,
            Some("Item is synchronized via Nextcloud. Deleting will propagate to your private cloud storage.".to_string())
        );
    }
    if lower.contains("box sync") || lower.contains("box\\") || lower.contains("box/") {
        return (
            true,
            Some("Item is inside a Box folder. Deleting will propagate to your Box enterprise storage.".to_string())
        );
    }

    (false, None)
}

/// Validates that a path is well-formed, safe, and not targeting protected system components.
pub fn validate_path_safety(path_str: &str) -> std::result::Result<PathBuf, String> {
    if path_str.trim().is_empty() {
        return Err("Path string cannot be empty".to_string());
    }

    let path = PathBuf::from(path_str);
    let (is_critical, reason) = is_system_critical_path(&path);
    if is_critical {
        return Err(format!(
            "Blocked by security safeguards: {}",
            reason.unwrap_or("Protected operating system or boot component")
        ));
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_critical_path_detection() {
        // Windows system directories and drivers
        assert!(is_system_critical_path(Path::new("C:\\Windows")).0);
        assert!(is_system_critical_path(Path::new("C:\\Windows\\System32")).0);
        assert!(is_system_critical_path(Path::new("C:\\Windows\\System32\\drivers\\ntfs.sys")).0);
        assert!(is_system_critical_path(Path::new("C:\\bootmgr")).0);
        assert!(is_system_critical_path(Path::new("C:\\pagefile.sys")).0);
        assert!(is_system_critical_path(Path::new("C:\\System Volume Information")).0);
        assert!(is_system_critical_path(Path::new("C:\\Windows\\WinSxS\\manifest.xml")).0);
        assert!(is_system_critical_path(Path::new("C:\\")).0);

        // Unix / macOS system paths
        assert!(is_system_critical_path(Path::new("/System")).0);
        assert!(is_system_critical_path(Path::new("/System/Library/Extensions/AppleKext.kext")).0);
        assert!(is_system_critical_path(Path::new("/usr/bin")).0);
        assert!(is_system_critical_path(Path::new("/usr/bin/bash")).0);
        assert!(is_system_critical_path(Path::new("/bin/sh")).0);
        assert!(is_system_critical_path(Path::new("/etc/hosts")).0);
        assert!(is_system_critical_path(Path::new("/")).0);

        // Safe user paths (should NOT be marked critical)
        assert!(!is_system_critical_path(Path::new("C:\\Users\\User\\Downloads\\old_installer.exe")).0);
        assert!(!is_system_critical_path(Path::new("C:\\Projects\\node_modules")).0);
        assert!(!is_system_critical_path(Path::new("/Users/user/Desktop/test_project/target")).0);
        assert!(!is_system_critical_path(Path::new("/tmp/synthetic_test_file.log")).0);
    }

    #[test]
    fn test_cloud_sync_detection() {
        let (is_cloud, warning) = detect_cloud_sync(Path::new("C:\\Users\\User\\OneDrive\\Documents\\large_file.zip"));
        assert!(is_cloud);
        assert!(warning.unwrap().contains("OneDrive"));

        let (is_cloud_db, _) = detect_cloud_sync(Path::new("/Users/user/Dropbox/Shared/archive.tar.gz"));
        assert!(is_cloud_db);

        let (is_cloud_gdrive, _) = detect_cloud_sync(Path::new("D:\\Google Drive\\Backups\\archive.bak"));
        assert!(is_cloud_gdrive);

        let (is_not_cloud, _) = detect_cloud_sync(Path::new("C:\\Users\\User\\Downloads\\local_file.iso"));
        assert!(!is_not_cloud);
    }
}
