use crate::models::classification::{ClassificationResult, StorageCategory};

pub struct RuleMatcher;

#[inline]
fn eq_path_byte_ci(a: u8, b: u8) -> bool {
    a.eq_ignore_ascii_case(&b) || ((a == b'/' || a == b'\\') && (b == b'/' || b == b'\\'))
}

#[inline]
fn contains_ci(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() {
        return true;
    }
    if h.len() < n.len() {
        return false;
    }
    'outer: for i in 0..=h.len() - n.len() {
        for j in 0..n.len() {
            if !eq_path_byte_ci(h[i + j], n[j]) {
                continue 'outer;
            }
        }
        return true;
    }
    false
}

#[inline]
fn starts_with_ci(haystack: &str, prefix: &str) -> bool {
    let h = haystack.as_bytes();
    let p = prefix.as_bytes();
    if h.len() < p.len() {
        return false;
    }
    for i in 0..p.len() {
        if !eq_path_byte_ci(h[i], p[i]) {
            return false;
        }
    }
    true
}

#[inline]
fn ends_with_ci(haystack: &str, suffix: &str) -> bool {
    let h = haystack.as_bytes();
    let s = suffix.as_bytes();
    if h.len() < s.len() {
        return false;
    }
    let off = h.len() - s.len();
    for i in 0..s.len() {
        if !eq_path_byte_ci(h[off + i], s[i]) {
            return false;
        }
    }
    true
}

#[inline]
fn eq_ext(ext: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|c| ext.eq_ignore_ascii_case(c))
}

impl RuleMatcher {
    /// Deterministically classifies a file into exactly one of the 16 storage categories.
    /// Evaluates path substrings zero-allocation (ASCII case-insensitive + `/` == `\`) to
    /// maximize throughput on multi-million-file parallel scans.
    pub fn classify(
        path: &str,
        name: &str,
        extension: Option<&str>,
        is_hidden: bool,
        is_system: bool,
    ) -> ClassificationResult {
        let ext = extension.unwrap_or("");

        // 1. Operating System (Priority 1)
        if Self::is_os_path(path, is_system) {
            let app = if contains_ci(path, "windows") {
                "Windows Operating System"
            } else {
                "System Operating Core"
            };
            return ClassificationResult {
                category: StorageCategory::OperatingSystem.id().to_string(),
                subcategory: Some("Core System Files".to_string()),
                app_name: Some(app.to_string()),
                confidence: 0.95,
            };
        }

        // 2. Virtual Machines & Emulators (Priority 2)
        if let Some((subcat, app)) = Self::match_virtual_machines(path, ext) {
            return ClassificationResult {
                category: StorageCategory::VirtualMachines.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.95,
            };
        }

        // 3. Gaming (Priority 3)
        if let Some((subcat, app)) = Self::match_gaming(path, ext) {
            return ClassificationResult {
                category: StorageCategory::Gaming.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.95,
            };
        }

        // 4. Developer Storage (Priority 4)
        if let Some((subcat, app)) = Self::match_developer(path, ext) {
            return ClassificationResult {
                category: StorageCategory::Developer.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.92,
            };
        }

        // 5. Browser Storage (Priority 5)
        if let Some((subcat, app)) = Self::match_browsers(path) {
            return ClassificationResult {
                category: StorageCategory::Browsers.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.95,
            };
        }

        // 6. Cloud Synchronization (Priority 6)
        if let Some((subcat, app)) = Self::match_cloud(path) {
            return ClassificationResult {
                category: StorageCategory::Cloud.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.90,
            };
        }

        // 7. Backups & Recovery (Priority 7)
        if let Some((subcat, app)) = Self::match_backups(path, ext) {
            return ClassificationResult {
                category: StorageCategory::Backups.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.90,
            };
        }

        // 8. Temporary Files & Caches (Priority 8)
        if let Some(subcat) = Self::match_temporary(path, ext) {
            return ClassificationResult {
                category: StorageCategory::Temporary.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some("Temporary Storage".to_string()),
                confidence: 0.85,
            };
        }

        // 9. Installed Applications (Priority 9)
        if let Some((subcat, app)) = Self::match_applications(path, ext) {
            return ClassificationResult {
                category: StorageCategory::Applications.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.90,
            };
        }

        // 10. Downloads & Archives (Priority 10)
        if contains_ci(path, "/downloads/")
            || eq_ext(ext, &["zip", "tar", "gz", "7z", "rar", "bz2", "xz", "iso", "dmg", "pkg", "tgz", "zst"])
        {
            return ClassificationResult {
                category: StorageCategory::Archives.id().to_string(),
                subcategory: Some(if ext.is_empty() { "Downloads".to_string() } else { "Archive Files".to_string() }),
                app_name: None,
                confidence: 0.85,
            };
        }

        // 11. Videos (Priority 11)
        if eq_ext(ext, &["mp4", "mkv", "mov", "avi", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "ts", "3gp"]) {
            return ClassificationResult {
                category: StorageCategory::Videos.id().to_string(),
                subcategory: Some("Video Media".to_string()),
                app_name: None,
                confidence: 0.85,
            };
        }

        // 12. Images (Priority 12)
        if eq_ext(ext, &["png", "jpg", "jpeg", "gif", "svg", "webp", "ico", "bmp", "tiff", "tif", "psd", "ai", "raw", "cr2", "nef", "heic", "avif"]) {
            return ClassificationResult {
                category: StorageCategory::Images.id().to_string(),
                subcategory: Some("Raster & Vector Images".to_string()),
                app_name: None,
                confidence: 0.85,
            };
        }

        // 13. Audio (Priority 13)
        if eq_ext(ext, &["mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "aiff", "alac", "mid", "midi"]) {
            return ClassificationResult {
                category: StorageCategory::Audio.id().to_string(),
                subcategory: Some("Audio Recordings".to_string()),
                app_name: None,
                confidence: 0.85,
            };
        }

        // 14. Personal Documents (Priority 14)
        if eq_ext(ext, &["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "txt", "rtf", "csv", "tsv", "md", "epub", "pages", "numbers", "key"]) {
            return ClassificationResult {
                category: StorageCategory::Documents.id().to_string(),
                subcategory: Some("Productivity & Office Documents".to_string()),
                app_name: None,
                confidence: 0.80,
            };
        }

        // 15. Hidden Storage (Priority 15)
        if is_hidden || name.starts_with('.') {
            return ClassificationResult {
                category: StorageCategory::Hidden.id().to_string(),
                subcategory: Some("Hidden Configuration & Metadata".to_string()),
                app_name: None,
                confidence: 0.70,
            };
        }

        // 16. Unknown Storage (Fallback / Catch-All - 100% accounted for, never dropped)
        ClassificationResult {
            category: StorageCategory::Unknown.id().to_string(),
            subcategory: Some(if ext.is_empty() { "Unextended Files".to_string() } else { format!(".{} files", ext.to_uppercase()) }),
            app_name: None,
            confidence: 0.20,
        }
    }

    fn is_os_path(p: &str, is_system: bool) -> bool {
        contains_ci(p, "/windows/")
            || contains_ci(p, ":/windows")
            || contains_ci(p, "/$recycle.bin/")
            || contains_ci(p, "/system volume information/")
            || starts_with_ci(p, "/system/")
            || starts_with_ci(p, "/private/var/")
            || starts_with_ci(p, "/usr/lib/")
            || starts_with_ci(p, "/sbin/")
            || starts_with_ci(p, "/dev/")
            || starts_with_ci(p, "/proc/")
            || starts_with_ci(p, "/sys/")
            || (is_system && !contains_ci(p, "/program files/") && !contains_ci(p, "/applications/"))
    }

    fn match_virtual_machines(p: &str, ext: &str) -> Option<(String, String)> {
        if contains_ci(p, "virtualbox") || contains_ci(p, ".virtualbox") {
            return Some(("VirtualBox VM Images".into(), "VirtualBox".into()));
        }
        if contains_ci(p, "docker") || contains_ci(p, ".docker") || contains_ci(p, "wsl/docker") {
            return Some(("Docker Containers & Storage".into(), "Docker".into()));
        }
        if contains_ci(p, "vmware") {
            return Some(("VMware Disk Images".into(), "VMware".into()));
        }
        if eq_ext(ext, &["vmdk", "vdi", "vhd", "vhdx", "qcow2", "ova", "ovf"]) {
            return Some(("Virtual Disk Image".into(), "Virtual Machine".into()));
        }
        None
    }

    fn match_gaming(p: &str, _ext: &str) -> Option<(String, String)> {
        if contains_ci(p, "steamapps") || contains_ci(p, "/steam/") {
            return Some(("Steam Game Content".into(), "Steam".into()));
        }
        if contains_ci(p, "epic games") {
            return Some(("Epic Games Content".into(), "Epic Games".into()));
        }
        if contains_ci(p, "riot games") {
            return Some(("Riot Games Content".into(), "Riot Games".into()));
        }
        if contains_ci(p, "battle.net") || contains_ci(p, "blizzard") {
            return Some(("Blizzard Content".into(), "Battle.net".into()));
        }
        if contains_ci(p, "gog galaxy") || contains_ci(p, "gog games") {
            return Some(("GOG Content".into(), "GOG Galaxy".into()));
        }
        None
    }

    fn match_developer(p: &str, ext: &str) -> Option<(String, String)> {
        if contains_ci(p, "/node_modules/") || contains_ci(p, "/.pnpm-store/") || contains_ci(p, "/library/pnpm/") {
            return Some(("npm / pnpm Dependencies".into(), "Node.js".into()));
        }
        if contains_ci(p, "/.cargo/") || contains_ci(p, "/.rustup/") {
            return Some(("Cargo Crates & Toolchains".into(), "Rust / Cargo".into()));
        }
        if contains_ci(p, "/target/debug/") || contains_ci(p, "/target/release/") {
            return Some(("Rust Build Artifacts".into(), "Rust / Cargo".into()));
        }
        if contains_ci(p, "/.git/") {
            return Some(("Git Object Database".into(), "Git VCS".into()));
        }
        if contains_ci(p, "/.gradle/") || contains_ci(p, "/.m2/repository") {
            return Some(("Java / Gradle Artifacts".into(), "Gradle / Maven".into()));
        }
        if contains_ci(p, "/.venv/") || contains_ci(p, "/venv/") || contains_ci(p, "/__pycache__/") || contains_ci(p, "/.cache/uv/") || contains_ci(p, "/.cache/pip/") {
            return Some(("Python Environment".into(), "Python".into()));
        }
        if contains_ci(p, "/deriveddata/") || contains_ci(p, "/pods/") || contains_ci(p, "/coresimulator/") {
            return Some(("Xcode Derived Data".into(), "Apple Xcode".into()));
        }
        if contains_ci(p, "/caches/homebrew") {
            return Some(("Homebrew Package Cache".into(), "Homebrew".into()));
        }

        // Developer source files
        if eq_ext(ext, &["rs", "ts", "tsx", "js", "jsx", "py", "go", "c", "cpp", "h", "hpp", "java", "cs", "swift", "kt", "html", "css", "scss", "json", "yaml", "yml", "toml", "xml", "sql", "sh", "bash", "zsh", "bat", "ps1"]) {
            return Some(("Source Code & Configuration".into(), "Developer Workspaces".into()));
        }

        None
    }

    fn match_browsers(p: &str) -> Option<(String, String)> {
        if contains_ci(p, "google/chrome") {
            return Some(("Chrome User Data".into(), "Google Chrome".into()));
        }
        if contains_ci(p, "microsoft/edge") || contains_ci(p, "microsoft edge") {
            return Some(("Edge User Data".into(), "Microsoft Edge".into()));
        }
        if contains_ci(p, "mozilla/firefox") || contains_ci(p, "caches/firefox") {
            return Some(("Firefox Profile Data".into(), "Mozilla Firefox".into()));
        }
        if contains_ci(p, "bravesoftware/brave") {
            return Some(("Brave Browser Data".into(), "Brave Browser".into()));
        }
        if contains_ci(p, "company.thebrowser.browser") {
            return Some(("Arc Browser Data".into(), "Arc Browser".into()));
        }
        if contains_ci(p, "library/safari") || contains_ci(p, "com.apple.safari") {
            return Some(("Safari Cache & State".into(), "Apple Safari".into()));
        }
        None
    }

    fn match_cloud(p: &str) -> Option<(String, String)> {
        if contains_ci(p, "onedrive") {
            return Some(("OneDrive Synced Files".into(), "Microsoft OneDrive".into()));
        }
        if contains_ci(p, "dropbox") {
            return Some(("Dropbox Synced Files".into(), "Dropbox".into()));
        }
        if contains_ci(p, "google drive") || contains_ci(p, "googledrive") {
            return Some(("Google Drive Sync".into(), "Google Drive".into()));
        }
        if contains_ci(p, "icloud") || contains_ci(p, "icloud~") || contains_ci(p, "mobile documents") {
            return Some(("iCloud Drive Sync".into(), "Apple iCloud".into()));
        }
        if contains_ci(p, "nextcloud") {
            return Some(("Nextcloud Sync".into(), "Nextcloud".into()));
        }
        None
    }

    fn match_backups(p: &str, ext: &str) -> Option<(String, String)> {
        if contains_ci(p, "timemachine") || contains_ci(p, "backups.backupdb") {
            return Some(("Time Machine Backup".into(), "Time Machine".into()));
        }
        if contains_ci(p, "filehistory") || contains_ci(p, "windowsimagebackup") {
            return Some(("Windows File History".into(), "Windows Backup".into()));
        }
        if contains_ci(p, "mobilesync/backup") {
            return Some(("iOS Device Backup".into(), "Apple MobileSync".into()));
        }
        if eq_ext(ext, &["bak", "bkp", "tib", "vbk", "gho", "wbcat"]) {
            return Some(("Backup Image File".into(), "Backup Utility".into()));
        }
        None
    }

    fn match_temporary(p: &str, ext: &str) -> Option<String> {
        if contains_ci(p, "/temp/") || ends_with_ci(p, "/temp") {
            return Some("System Temp Directory".into());
        }
        if contains_ci(p, "/caches/") || contains_ci(p, "/cache/") || contains_ci(p, "library/caches") {
            return Some("Application Cache".into());
        }
        if contains_ci(p, "prefetch") {
            return Some("Windows Prefetch".into());
        }
        if eq_ext(ext, &["tmp", "temp", "log", "dmp", "chk", "swp"]) {
            return Some("Temporary Log or Dump".into());
        }
        None
    }

    fn match_applications(orig_path: &str, ext: &str) -> Option<(String, String)> {
        if contains_ci(orig_path, "/applications/")
            || contains_ci(orig_path, "/program files/")
            || contains_ci(orig_path, "/program files (x86)/")
            || contains_ci(orig_path, "/appdata/local/programs/")
        {
            // Extract application name from original-case path segment
            let app_name = Self::extract_app_name_from_path(orig_path);
            return Some(("Application Executables & Bundles".into(), app_name));
        }
        if eq_ext(ext, &["exe", "dll", "app", "dylib", "so", "msi", "pkg"]) {
            return Some(("Binary Executable or Library".into(), "Independent Binary".into()));
        }
        None
    }

    fn extract_app_name_from_path(orig_path: &str) -> String {
        let parts: Vec<&str> = orig_path.split(&['/', '\\'][..]).collect();
        for (i, part) in parts.iter().enumerate() {
            let lower = part.to_lowercase();
            if (lower == "applications"
                || lower == "program files"
                || lower == "program files (x86)"
                || lower == "programs")
                && i + 1 < parts.len()
            {
                let next = parts[i + 1];
                if !next.is_empty() {
                    return next.strip_suffix(".app").unwrap_or(next).to_string();
                }
            }
        }
        "Installed Application".to_string()
    }
}

