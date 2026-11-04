/**
 * Rust to TypeScript type contracts.
 * Matches structs defined in src-tauri/src/models/
 */

export type DriveType =
  | 'internalHdd'
  | 'internalSsd'
  | 'externalHdd'
  | 'externalSsd'
  | 'usb'
  | 'sdCard'
  | 'network'
  | 'optical'
  | 'ram'
  | 'unknown';

export interface DriveInfo {
  id: string;               // e.g. "C:", "/dev/disk1s1"
  name: string;             // Volume name or label e.g. "Windows", "Macintosh HD"
  driveLetter: string | null; // e.g. "C:", "D:"
  mountPoint: string;       // e.g. "C:\\", "/"
  fileSystem: string | null;// e.g. "NTFS", "APFS", "FAT32"
  totalBytes: number | null; // null if unmounted, empty optical tray, or inaccessible
  availableBytes: number | null;
  usedBytes: number | null;
  usedPercent: number | null;
  driveType: DriveType;
  volumeId: string | null;  // Serial number or GUID
  isRemovable: boolean;
  isSystem: boolean;
  isReadOnly: boolean;
}

export interface DriveDetails {
  drive: DriveInfo;
  busType: string | null;   // e.g. "NVMe", "USB", "SATA", "Apple Fabric"
  serialNumber: string | null;
  sectorsPerCluster: number | null;
  bytesPerSector: number | null;
  isEncrypted: boolean | null;
  flags: string[];
}

export interface SystemInfo {
  osName: string;           // "Windows 11 Pro", "macOS 15.0"
  osVersion: string;
  kernelVersion: string;
  hostName: string;
  arch: string;             // "x86_64", "aarch64"
  cpuCount: number;
  totalMemoryBytes: number;
  appVersion: string;
}

export interface DatabaseHealth {
  status: 'healthy' | 'degraded' | 'error';
  migrationVersion: number;
  dbPath: string;
  tablesCount: number;
  message?: string;
}

export interface AppError {
  code: string;
  message: string;
  details?: string;
}
