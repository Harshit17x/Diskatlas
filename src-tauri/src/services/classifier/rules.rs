use crate::models::classification::{ClassificationResult, StorageCategory};

pub struct RuleMatcher;

impl RuleMatcher {
    /// Deterministically classifies a file into exactly one of the 16 storage categories.
    /// Rules are evaluated in strict priority order to prevent double-counting.
    pub fn classify(
        path: &str,
        name: &str,
        extension: Option<&str>,
        is_hidden: bool,
        is_system: bool,
    ) -> ClassificationResult {
        let p_lower = path.to_lowercase().replace('\\', "/");
        let ext_lower = extension.map(|e| e.to_lowercase());
        let ext = ext_lower.as_deref().unwrap_or("");

        // 1. Operating System (Priority 1)
        if Self::is_os_path(&p_lower, is_system) {
            let app = if p_lower.contains("windows") {
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
        if let Some((subcat, app)) = Self::match_virtual_machines(&p_lower, ext) {
            return ClassificationResult {
                category: StorageCategory::VirtualMachines.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.95,
            };
        }

        // 3. Gaming (Priority 3)
        if let Some((subcat, app)) = Self::match_gaming(&p_lower, ext) {
            return ClassificationResult {
                category: StorageCategory::Gaming.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.95,
            };
        }

        // 4. Developer Storage (Priority 4)
        if let Some((subcat, app)) = Self::match_developer(&p_lower, ext) {
            return ClassificationResult {
                category: StorageCategory::Developer.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.92,
            };
        }

        // 5. Browser Storage (Priority 5)
        if let Some((subcat, app)) = Self::match_browsers(&p_lower) {
            return ClassificationResult {
                category: StorageCategory::Browsers.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.95,
            };
        }

        // 6. Cloud Synchronization (Priority 6)
        if let Some((subcat, app)) = Self::match_cloud(&p_lower) {
            return ClassificationResult {
                category: StorageCategory::Cloud.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.90,
            };
        }

        // 7. Backups & Recovery (Priority 7)
        if let Some((subcat, app)) = Self::match_backups(&p_lower, ext) {
            return ClassificationResult {
                category: StorageCategory::Backups.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.90,
            };
        }

        // 8. Temporary Files & Caches (Priority 8)
        if let Some(subcat) = Self::match_temporary(&p_lower, ext) {
            return ClassificationResult {
                category: StorageCategory::Temporary.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some("Temporary Storage".to_string()),
                confidence: 0.85,
            };
        }

        // 9. Installed Applications (Priority 9)
        if let Some((subcat, app)) = Self::match_applications(&p_lower, ext) {
            return ClassificationResult {
                category: StorageCategory::Applications.id().to_string(),
                subcategory: Some(subcat),
                app_name: Some(app),
                confidence: 0.90,
            };
        }

        // 10. Downloads & Archives (Priority 10)
        if p_lower.contains("/downloads/") || matches!(ext, "zip" | "tar" | "gz" | "7z" | "rar" | "bz2" | "xz" | "iso" | "dmg" | "pkg" | "tgz" | "zst") {
            return ClassificationResult {
                category: StorageCategory::Archives.id().to_string(),
                subcategory: Some(if ext.is_empty() { "Downloads".to_string() } else { "Archive Files".to_string() }),
                app_name: None,
                confidence: 0.85,
            };
        }

        // 11. Videos (Priority 11)
        if matches!(ext, "mp4" | "mkv" | "mov" | "avi" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" | "ts" | "3gp") {
            return ClassificationResult {
                category: StorageCategory::Videos.id().to_string(),
                subcategory: Some("Video Media".to_string()),
                app_name: None,
                confidence: 0.85,
            };
        }

        // 12. Images (Priority 12)
        if matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" | "bmp" | "tiff" | "tif" | "psd" | "ai" | "raw" | "cr2" | "nef" | "heic" | "avif") {
            return ClassificationResult {
                category: StorageCategory::Images.id().to_string(),
                subcategory: Some("Raster & Vector Images".to_string()),
                app_name: None,
                confidence: 0.85,
            };
        }

        // 13. Audio (Priority 13)
        if matches!(ext, "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "aiff" | "alac" | "mid" | "midi") {
            return ClassificationResult {
                category: StorageCategory::Audio.id().to_string(),
                subcategory: Some("Audio Recordings".to_string()),
                app_name: None,
                confidence: 0.85,
            };
        }

        // 14. Personal Documents (Priority 14)
        if matches!(ext, "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "txt" | "rtf" | "csv" | "tsv" | "md" | "epub" | "pages" | "numbers" | "key") {
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
        p.contains("/windows/")
            || p.contains("/c:/windows")
            || p.contains("/$recycle.bin/")
            || p.contains("/system volume information/")
            || p.starts_with("/system/")
            || p.starts_with("/private/var/")
            || p.starts_with("/usr/lib/")
            || p.starts_with("/sbin/")
            || p.starts_with("/dev/")
            || p.starts_with("/proc/")
            || p.starts_with("/sys/")
            || (is_system && !p.contains("/program files/") && !p.contains("/applications/"))
    }

    fn match_virtual_machines(p: &str, ext: &str) -> Option<(String, String)> {
        if p.contains("virtualbox") || p.contains(".virtualbox") {
            return Some(("VirtualBox VM Images".into(), "VirtualBox".into()));
        }
        if p.contains("docker") || p.contains(".docker") || p.contains("wsl/docker") {
            return Some(("Docker Containers & Storage".into(), "Docker".into()));
        }
        if p.contains("vmware") {
            return Some(("VMware Disk Images".into(), "VMware".into()));
        }
        if matches!(ext, "vmdk" | "vdi" | "vhd" | "vhdx" | "qcow2" | "ova" | "ovf") {
            return Some(("Virtual Disk Image".into(), "Virtual Machine".into()));
        }
        None
    }

    fn match_gaming(p: &str, _ext: &str) -> Option<(String, String)> {
        if p.contains("steamapps") || p.contains("/steam/") {
            return Some(("Steam Game Content".into(), "Steam".into()));
        }
        if p.contains("epic games") {
            return Some(("Epic Games Content".into(), "Epic Games".into()));
        }
        if p.contains("riot games") {
            return Some(("Riot Games Content".into(), "Riot Games".into()));
        }
        if p.contains("battle.net") || p.contains("blizzard") {
            return Some(("Blizzard Content".into(), "Battle.net".into()));
        }
        if p.contains("gog galaxy") || p.contains("gog games") {
            return Some(("GOG Content".into(), "GOG Galaxy".into()));
        }
        None
    }

    fn match_developer(p: &str, ext: &str) -> Option<(String, String)> {
        if p.contains("/node_modules/") {
            return Some(("npm Dependencies".into(), "Node.js".into()));
        }
        if p.contains("/.cargo/") || p.contains("/.rustup/") {
            return Some(("Cargo Crates & Toolchains".into(), "Rust / Cargo".into()));
        }
        if p.contains("/target/debug/") || p.contains("/target/release/") {
            return Some(("Rust Build Artifacts".into(), "Rust / Cargo".into()));
        }
        if p.contains("/.git/") {
            return Some(("Git Object Database".into(), "Git VCS".into()));
        }
        if p.contains("/.gradle/") || p.contains("/.m2/repository") {
            return Some(("Java / Gradle Artifacts".into(), "Gradle / Maven".into()));
        }
        if p.contains("/.venv/") || p.contains("/venv/") || p.contains("/__pycache__/") {
            return Some(("Python Environment".into(), "Python".into()));
        }
        if p.contains("/deriveddata/") || p.contains("/pods/") {
            return Some(("Xcode Derived Data".into(), "Apple Xcode".into()));
        }

        // Developer source files
        if matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "c" | "cpp" | "h" | "hpp" | "java" | "cs" | "swift" | "kt" | "html" | "css" | "scss" | "json" | "yaml" | "yml" | "toml" | "xml" | "sql" | "sh" | "bash" | "zsh" | "bat" | "ps1") {
            return Some(("Source Code & Configuration".into(), "Developer Workspaces".into()));
        }

        None
    }

    fn match_browsers(p: &str) -> Option<(String, String)> {
        if p.contains("google/chrome") || p.contains("google\\chrome") {
            return Some(("Chrome User Data".into(), "Google Chrome".into()));
        }
        if p.contains("microsoft/edge") || p.contains("microsoft\\edge") {
            return Some(("Edge User Data".into(), "Microsoft Edge".into()));
        }
        if p.contains("mozilla/firefox") || p.contains("mozilla\\firefox") {
            return Some(("Firefox Profile Data".into(), "Mozilla Firefox".into()));
        }
        if p.contains("bravesoftware/brave") {
            return Some(("Brave Browser Data".into(), "Brave Browser".into()));
        }
        if p.contains("library/safari") || p.contains("com.apple.safari") {
            return Some(("Safari Cache & State".into(), "Apple Safari".into()));
        }
        None
    }

    fn match_cloud(p: &str) -> Option<(String, String)> {
        if p.contains("onedrive") {
            return Some(("OneDrive Synced Files".into(), "Microsoft OneDrive".into()));
        }
        if p.contains("dropbox") {
            return Some(("Dropbox Synced Files".into(), "Dropbox".into()));
        }
        if p.contains("google drive") || p.contains("googledrive") {
            return Some(("Google Drive Sync".into(), "Google Drive".into()));
        }
        if p.contains("icloud") || p.contains("icloud~") || p.contains("mobile documents") {
            return Some(("iCloud Drive Sync".into(), "Apple iCloud".into()));
        }
        if p.contains("nextcloud") {
            return Some(("Nextcloud Sync".into(), "Nextcloud".into()));
        }
        None
    }

    fn match_backups(p: &str, ext: &str) -> Option<(String, String)> {
        if p.contains("timemachine") || p.contains("backups.backupdb") {
            return Some(("Time Machine Backup".into(), "Time Machine".into()));
        }
        if p.contains("filehistory") || p.contains("windowsimagebackup") {
            return Some(("Windows File History".into(), "Windows Backup".into()));
        }
        if p.contains("mobilesync/backup") {
            return Some(("iOS Device Backup".into(), "Apple MobileSync".into()));
        }
        if matches!(ext, "bak" | "bkp" | "tib" | "vbk" | "gho" | "wbcat") {
            return Some(("Backup Image File".into(), "Backup Utility".into()));
        }
        None
    }

    fn match_temporary(p: &str, ext: &str) -> Option<String> {
        if p.contains("/temp/") || p.contains("\\temp\\") || p.ends_with("/temp") {
            return Some("System Temp Directory".into());
        }
        if p.contains("/caches/") || p.contains("/cache/") || p.contains("library/caches") {
            return Some("Application Cache".into());
        }
        if p.contains("prefetch") {
            return Some("Windows Prefetch".into());
        }
        if matches!(ext, "tmp" | "temp" | "log" | "dmp" | "chk" | "swp") {
            return Some("Temporary Log or Dump".into());
        }
        None
    }

    fn match_applications(p: &str, ext: &str) -> Option<(String, String)> {
        if p.contains("/applications/") || p.contains("/program files/") || p.contains("/program files (x86)/") || p.contains("/appdata/local/programs/") {
            // Extract application name from path segment
            let app_name = Self::extract_app_name_from_path(p);
            return Some(("Application Executables & Bundles".into(), app_name));
        }
        if matches!(ext, "exe" | "dll" | "app" | "dylib" | "so" | "msi" | "pkg") {
            return Some(("Binary Executable or Library".into(), "Independent Binary".into()));
        }
        None
    }

    fn extract_app_name_from_path(p: &str) -> String {
        let parts: Vec<&str> = p.split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            if (*part == "applications" || *part == "program files" || *part == "program files (x86)" || *part == "programs")
                && i + 1 < parts.len()
            {
                let next = parts[i + 1];
                if !next.is_empty() {
                    return next.replace(".app", "");
                }
            }
        }
        "Installed Application".to_string()
    }
}

