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

export interface DeveloperProjectItem {
  id: string;
  sessionId: string;
  name: string;
  rootPath: string;
  ecosystem: string;
  dependencyBytes: number;
  cacheBytes: number;
  buildOutputBytes: number;
  totalBytes: number;
  fileCount: number;
  lastModifiedAt: number | null;
  confidence: number;
}

export interface DeveloperEcosystemSummary {
  ecosystem: string;
  projectCount: number;
  totalBytes: number;
  dependencyBytes: number;
  cacheBytes: number;
  buildBytes: number;
}

export interface DockerStorageInfo {
  isAvailable: boolean;
  imagesBytes: number;
  imagesCount: number;
  containersBytes: number;
  containersCount: number;
  volumesBytes: number;
  volumesCount: number;
  buildCacheBytes: number;
  totalBytes: number;
  statusMessage: string;
}

export interface GlobalToolCacheItem {
  tool: string;
  path: string;
  sizeBytes: number;
  fileCount: number;
  exists: boolean;
}

export interface DeveloperAnalysisProfile {
  sessionId: string;
  totalDeveloperBytes: number;
  totalProjects: number;
  ecosystems: DeveloperEcosystemSummary[];
  projects: DeveloperProjectItem[];
  docker: DockerStorageInfo;
  globalCaches: GlobalToolCacheItem[];
}

// ==========================================
// PHASE 8: ADVANCED STORAGE ANALYSIS TYPES
// ==========================================

export interface LargeFileItem {
  id: number;
  sessionId: string;
  path: string;
  name: string;
  extension: string | null;
  sizeBytes: number;
  category: string | null;
  modifiedAt: number | null;
  isHidden: boolean;
  isSystem: boolean;
}

export interface LargeFilesQuery {
  sessionId?: string;
  minSizeBytes?: number;
  extension?: string;
  pathFilter?: string;
  sortBy?: 'size_desc' | 'size_asc' | 'modified_desc' | 'name_asc';
  page: number;
  pageSize: number;
}

export interface LargeFilesResponse {
  items: LargeFileItem[];
  totalItems: number;
  totalBytes: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface HiddenItem {
  id: number;
  path: string;
  name: string;
  isDirectory: boolean;
  sizeBytes: number;
  modifiedAt: number | null;
  hiddenType: 'system_protected' | 'user_hidden' | 'dotfile' | 'app_hidden' | string;
  protectionLevel: 'system_protected' | 'user_managed' | 'app_managed' | string;
  safetyNote: string;
}

export interface HiddenFilesQuery {
  sessionId?: string;
  filterType?: string;
  search?: string;
  page: number;
  pageSize: number;
}

export interface HiddenFilesResponse {
  items: HiddenItem[];
  totalItems: number;
  totalBytes: number;
  systemProtectedBytes: number;
  userHiddenBytes: number;
  dotfilesBytes: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface DuplicateFileItem {
  id: number;
  path: string;
  name: string;
  sizeBytes: number;
  modifiedAt: number | null;
  createdAt: number | null;
}

export interface DuplicateGroup {
  hash: string;
  unitSizeBytes: number;
  copiesCount: number;
  wastedBytes: number;
  sampleName: string;
  files: DuplicateFileItem[];
}

export interface DuplicateSearchQuery {
  sessionId?: string;
  minSizeBytes?: number;
  extension?: string;
  limitGroups?: number;
}

export interface DuplicateSearchResponse {
  groups: DuplicateGroup[];
  totalDuplicateFiles: number;
  totalWastedBytes: number;
  groupsCount: number;
  hashingAlgorithm: string;
}

export interface ForgottenFileItem {
  id: number;
  path: string;
  name: string;
  extension: string | null;
  sizeBytes: number;
  modifiedAt: number | null;
  ageDays: number;
  signalType: 'old_download' | 'old_installer' | 'old_backup' | 'old_archive' | 'stale_build_artifact' | string;
  safetyHeuristic: string;
}

export interface ForgottenFilesQuery {
  sessionId?: string;
  minAgeDays?: number;
  signalType?: string;
  page: number;
  pageSize: number;
}

export interface ForgottenFilesResponse {
  items: ForgottenFileItem[];
  totalItems: number;
  totalBytes: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface UnknownStorageItem {
  path: string;
  isDirectory: boolean;
  sizeBytes: number;
  fileCount: number;
  extension: string | null;
  modifiedAt: number | null;
  confidence: number;
}

export interface UnknownStorageResponse {
  items: UnknownStorageItem[];
  totalUnknownBytes: number;
  totalUnknownFiles: number;
  topUnknownDirectories: UnknownStorageItem[];
}

// ==========================================
// PHASE 9: STORAGE INTELLIGENCE TYPES
// ==========================================

export type RecommendationGroup =
  | 'safe_to_review'
  | 'review_required'
  | 'system_protected'
  | 'move_or_archive'
  | 'uninstall_official';

export type RiskLevel = 'none' | 'low' | 'moderate' | 'high' | 'critical';

export interface StorageRecommendation {
  id: string;
  sessionId: string;
  path: string;
  name: string;
  sizeBytes: number;
  category: string;
  group: RecommendationGroup;
  riskLevel: RiskLevel;
  reason: string;
  confidence: number;
  recoverableBytes: number;
  expectedConsequences: string;
  recommendedAction: string;
  createdAt: string;
}

export interface StorageFinding {
  id: string;
  title: string;
  description: string;
  severity: 'info' | 'advisory' | 'warning';
  impactBytes: number;
  affectedItemsCount: number;
  findingType: string;
}

export interface StorageConsumer {
  path: string;
  name: string;
  sizeBytes: number;
  category: string;
  percentageOfUsed: number;
  isDirectory: boolean;
  explanation: string;
}

export interface GroupSummaryItem {
  group: string;
  count: number;
  totalBytes: number;
  recoverableBytes: number;
}

export interface StorageIntelligenceProfile {
  sessionId: string;
  driveId: string;
  totalScannedBytes: number;
  potentialReclaimableBytes: number;
  safeReclaimableBytes: number;
  reviewRequiredBytes: number;
  systemProtectedBytes: number;
  findings: StorageFinding[];
  largestConsumers: StorageConsumer[];
  groupSummaries: Record<string, GroupSummaryItem>;
}

export interface RecommendationsQuery {
  sessionId?: string;
  group?: string;
  riskLevel?: string;
  search?: string;
  page: number;
  pageSize: number;
}

export interface RecommendationsResponse {
  items: StorageRecommendation[];
  totalItems: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

// ==========================================
// PHASE 10: STORAGE VISUALIZATION & ANALYTICS
// ==========================================

export interface HierarchicalNode {
  name: string;
  path: string;
  value: number;
  category: string | null;
  isDirectory: boolean;
  fileCount: number | null;
  children?: HierarchicalNode[];
}

export interface HierarchicalQuery {
  sessionId?: string;
  rootPath?: string;
  maxDepth?: number;
  minBytesThreshold?: number;
}

export interface ExtensionDistItem {
  extension: string;
  fileCount: number;
  totalBytes: number;
  percentage: number;
}

export interface AppStorageItem {
  appName: string;
  fileCount: number;
  totalBytes: number;
  percentage: number;
}

export interface LargestDirItem {
  path: string;
  name: string;
  totalBytes: number;
  fileCount: number;
  dirCount: number;
  percentage: number;
}

export interface CategoryChartItem {
  category: string;
  fileCount: number;
  totalBytes: number;
  percentage: number;
}

export interface StorageAnalyticsData {
  sessionId: string;
  totalBytes: number;
  totalFiles: number;
  categoryDistribution: CategoryChartItem[];
  extensionDistribution: ExtensionDistItem[];
  largestDirectories: LargestDirItem[];
  applicationStorage: AppStorageItem[];
  developerStorage: DeveloperEcosystemSummary[];
  historyTrends: StorageSnapshotRecord[];
}



