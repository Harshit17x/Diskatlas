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

export type ScanStatus =
  | 'queued'
  | 'running'
  | 'paused'
  | 'completed'
  | 'completedWithErrors'
  | 'cancelled'
  | 'failed'
  | 'interrupted';

export interface ScanProgress {
  sessionId: string;
  driveId: string;
  rootPath: string;
  status: ScanStatus;
  filesScanned: number;
  dirsScanned: number;
  bytesScanned: number;
  errorCount: number;
  currentPath: string;
  elapsedMillis: number;
}

export interface ScanFileItem {
  id: number;
  sessionId: string;
  path: string;
  name: string;
  extension: string | null;
  parentPath: string;
  sizeBytes: number;
  allocatedBytes: number | null;
  createdAt: number | null;
  modifiedAt: number | null;
  isHidden: boolean;
  isSystem: boolean;
  isSymlink: boolean;
}

export interface ScanFilePage {
  items: ScanFileItem[];
  totalCount: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface ScanSummary {
  sessionId: string;
  driveId: string;
  rootPath: string;
  status: ScanStatus;
  totalFiles: number;
  totalDirs: number;
  totalBytes: number;
  errorCount: number;
  durationMillis: number;
}

export interface ScanSessionRecord {
  id: string;
  driveId: string;
  driveLabel: string | null;
  rootPath: string;
  status: ScanStatus;
  totalFiles: number;
  totalDirectories: number;
  totalBytes: number;
  errorCount: number;
  startedAt: string;
  completedAt: string | null;
  durationMillis: number | null;
}

export interface ScanSessionPage {
  items: ScanSessionRecord[];
  totalCount: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface CategorySummaryItem {
  category: string;
  fileCount: number;
  totalBytes: number;
  percentageBytes: number;
}

export interface DirectoryRecord {
  path: string;
  name: string;
  totalSizeBytes: number;
  fileCount: number;
  dirCount: number;
}

export interface ScanErrorItem {
  path: string;
  errorType: string;
  message: string;
  occurredAt: string;
}

export interface StorageSnapshotRecord {
  id: string;
  sessionId: string | null;
  driveId: string;
  label: string;
  rootPath: string;
  totalFiles: number;
  totalDirectories: number;
  totalBytes: number;
  createdAt: string;
}

export interface SessionDetails {
  session: ScanSessionRecord;
  categories: CategorySummaryItem[];
  topDirectories: DirectoryRecord[];
  errorSamples: ScanErrorItem[];
}

export interface CategoryDeltaItem {
  category: string;
  baselineBytes: number;
  targetBytes: number;
  deltaBytes: number;
  baselineFiles: number;
  targetFiles: number;
  deltaFiles: number;
}

export interface DirectoryDeltaItem {
  path: string;
  baselineSizeBytes: number;
  targetSizeBytes: number;
  deltaBytes: number;
}

export interface FileModifiedItem {
  path: string;
  name: string;
  extension: string | null;
  baselineSizeBytes: number;
  targetSizeBytes: number;
  sizeDeltaBytes: number;
  modifiedAt: number | null;
}

export interface ScanComparisonResult {
  baselineSession: ScanSessionRecord;
  targetSession: ScanSessionRecord;
  sizeDeltaBytes: number;
  filesDelta: number;
  directoriesDelta: number;
  durationDeltaMillis: number | null;
  categoryDeltas: CategoryDeltaItem[];
  directoryDeltas: DirectoryDeltaItem[];
  addedFilesCount: number;
  removedFilesCount: number;
  modifiedFilesCount: number;
  addedFilesSample: ScanFileItem[];
  removedFilesSample: ScanFileItem[];
  modifiedFilesSample: FileModifiedItem[];
}

export type StorageCategoryKey =
  | 'os'
  | 'applications'
  | 'developer'
  | 'temporary'
  | 'documents'
  | 'images'
  | 'videos'
  | 'audio'
  | 'archives'
  | 'browsers'
  | 'cloud'
  | 'gaming'
  | 'backups'
  | 'vm_emulators'
  | 'hidden'
  | 'unknown';

export interface SubcategoryStatItem {
  subcategory: string;
  fileCount: number;
  totalBytes: number;
}

export interface AppStatItem {
  appName: string;
  fileCount: number;
  totalBytes: number;
}

export interface CategoryStatItem {
  category: string;
  label: string;
  fileCount: number;
  dirCount: number;
  totalBytes: number;
  percentageBytes: number;
  avgConfidence: number;
  subcategories: SubcategoryStatItem[];
  topApps: AppStatItem[];
}

export interface StorageAnalysisProfile {
  sessionId: string;
  driveId: string;
  totalFiles: number;
  totalBytes: number;
  categories: CategoryStatItem[];
  classifiedPercent: number;
  overallConfidence: number;
}

export interface CategoryOverrideRule {
  id: number;
  pattern: string;
  category: string;
  subcategory: string | null;
  createdAt: string;
}
