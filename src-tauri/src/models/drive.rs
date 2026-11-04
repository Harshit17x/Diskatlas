use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DriveType {
    InternalHdd,
    InternalSsd,
    ExternalHdd,
    ExternalSsd,
    Usb,
    SdCard,
    Network,
    Optical,
    Ram,
    Unknown,
}

impl DriveType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DriveType::InternalHdd => "Internal HDD",
            DriveType::InternalSsd => "Internal SSD",
            DriveType::ExternalHdd => "External HDD",
            DriveType::ExternalSsd => "External SSD",
            DriveType::Usb => "USB Flash Drive",
            DriveType::SdCard => "Memory Card (SD)",
            DriveType::Network => "Network Drive",
            DriveType::Optical => "Optical Drive",
            DriveType::Ram => "RAM Disk",
            DriveType::Unknown => "Unknown Drive",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub id: String,
    pub name: String,
    pub drive_letter: Option<String>,
    pub mount_point: String,
    pub file_system: Option<String>,
    /// Total capacity in bytes. None if the volume is unformatted, empty media, or inaccessible.
    pub total_bytes: Option<u64>,
    /// Available capacity in bytes. None if inaccessible.
    pub available_bytes: Option<u64>,
    /// Used capacity in bytes. None if inaccessible.
    pub used_bytes: Option<u64>,
    /// Used capacity percentage (0.0 to 100.0). None if capacity is unavailable.
    pub used_percent: Option<f64>,
    pub drive_type: DriveType,
    /// Volume identifier, serial number or GUID (e.g. "B4A1-2C9D", "Volume{...}", "/dev/disk1s1")
    pub volume_id: Option<String>,
    pub is_removable: bool,
    pub is_system: bool,
    pub is_read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveDetails {
    pub drive: DriveInfo,
    /// Hardware interface bus type (e.g. "NVMe", "USB", "SATA", "SCSI", "APFS Container")
    pub bus_type: Option<String>,
    pub serial_number: Option<String>,
    pub sectors_per_cluster: Option<u32>,
    pub bytes_per_sector: Option<u32>,
    pub is_encrypted: Option<bool>,
    pub flags: Vec<String>,
}
