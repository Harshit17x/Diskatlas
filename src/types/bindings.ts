/**
 * Rust to TypeScript type contracts.
 * Matches structs defined in src-tauri/src/models/
 */

export type DriveType = 'Fixed' | 'Removable' | 'Network' | 'CDRom' | 'Ram' | 'Unknown';

export interface DriveInfo {
  id: string;               // e.g. "C:", "/dev/disk1s1"
  label: string;            // e.g. "Windows", "Macintosh HD"
  mountPoint: string;       // e.g. "C:\\", "/"
  fileSystem: string;       // e.g. "NTFS", "APFS", "ext4"
  totalBytes: number;       // total space
  availableBytes: number;   // free space
  usedBytes: number;        // total - available
  usedPercent: number;      // 0 - 100
  isRemovable: boolean;
  isSystem: boolean;
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

