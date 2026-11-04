use sysinfo::System;
use crate::models::system::SystemInfo;

pub struct SystemInspector;

impl SystemInspector {
    pub fn inspect() -> SystemInfo {
        let mut sys = System::new_all();
        sys.refresh_all();

        let os_name = System::name().unwrap_or_else(|| "Unknown OS".into());
        let os_version = System::os_version().unwrap_or_else(|| "".into());
        let kernel_version = System::kernel_version().unwrap_or_else(|| "".into());
        let host_name = System::host_name().unwrap_or_else(|| "localhost".into());
        let arch = std::env::consts::ARCH.to_string();
        let cpu_count = sys.cpus().len();
        let total_memory_bytes = sys.total_memory();
        let app_version = env!("CARGO_PKG_VERSION").to_string();

        SystemInfo {
            os_name,
            os_version,
            kernel_version,
            host_name,
            arch,
            cpu_count,
            total_memory_bytes,
            app_version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_inspector_extracts_valid_data() {
        let info = SystemInspector::inspect();
        assert!(!info.os_name.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.cpu_count > 0);
        assert!(info.total_memory_bytes > 0);
    }
}

