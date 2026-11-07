# External Integrations

**Analysis Date:** 2026-11-06

## APIs & External Services

DiskAtlas is engineered as an offline-first, local storage intelligence desktop application. It does **not** consume, communicate with, or transmit telemetry to any external cloud APIs, analytics endpoints, or remote services.

**Local Developer Tooling:**
- **Docker Engine CLI** - Queries local container, image, volume, and build cache storage metrics without modifying container runtime internals.
  - Client: `std::process::Command` executing `docker system df --format ...` in [`src-tauri/src/services/developer/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/developer/mod.rs#L301).
  - Auth: None (relies on local Docker socket permissions or user membership in `docker` group).
  - Fallback: Gracefully reports if Docker daemon is offline or if `docker` binary is absent from `PATH`.
- **macOS `diskutil` CLI** - Extracts exact APFS container metrics for the root volume.
  - Client: `std::process::Command` executing `diskutil info /` in [`src-tauri/src/services/drive_detector.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/drive_detector.rs#L361).
  - Auth: None (standard unprivileged diagnostic command).
  - Fallback: Falls back to standard `sysinfo::Disks` space calculations if `diskutil` execution fails.

## Data Storage

**Databases:**
- **Embedded SQLite (SQLite 3.47.2 bundled)**
  - Location: Resolved at startup via Tauri app data directory (`app.path().app_data_dir().join("diskatlas.db")`), falling back to local portable directory `./.diskatlas_data/diskatlas.db` if the system folder cannot be created ([`src-tauri/src/lib.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/lib.rs#L48-L68)).
  - Client: `rusqlite 0.33.0` with `bundled` feature in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L23).
  - Connection Pool: Thread-safe `DbPool` holding `Arc<Mutex<rusqlite::Connection>>` in [`src-tauri/src/storage/connection.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/storage/connection.rs).
  - Performance Pragmas:
    - `journal_mode = WAL` (Write-Ahead Logging for high-concurrency non-blocking reads).
    - `synchronous = NORMAL` (Optimized disk flushes while maintaining durability).
    - `busy_timeout = 5000` (5-second lock retry before failing).
    - `foreign_keys = ON` (Referential integrity enforcement across session cascading deletes).
    - `cache_size = -64000` (64 MB allocated in-memory page cache).
    - `temp_store = MEMORY` (Temporary indices and intermediate groupings computed in RAM).
    - `mmap_size = 268435456` (256 MB memory-mapped I/O for high-speed file table reads).
  - Embedded Migration Engine:
    - Handled by `Migrator::run` in [`src-tauri/src/storage/migrator.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/storage/migrator.rs) using compiled SQL strings (`include_str!`).
    - Tracks applied versions in internal `_migrations` table inside atomic transactions.
    - Migration scripts (8 total in [`src-tauri/migrations/`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/migrations/)):
      - `0001_initial_schema.sql`: Core tables `_migrations`, `drives`, `scan_sessions`.
      - `0002_files_and_directories.sql`: Indexed filesystem records `files`, `directories`, `scan_errors`.
      - `0003_storage_history_and_snapshots.sql`: `storage_snapshots`, `category_summaries`.
      - `0004_storage_classification.sql`: `category_overrides`, `classification_summaries`, classification fields on `files`.
      - `0005_developer_storage.sql`: `developer_projects`, `developer_ecosystem_summaries`.
      - `0006_advanced_analysis.sql`: `file_hashes` for duplicate detection, compound query indexes.
      - `0007_storage_intelligence.sql`: `storage_recommendations` and recommendation query indexes.
      - `0008_cleanup_audit_history.sql`: `cleanup_audit_events` for audit history tracking.

**File Storage:**
- **Local Filesystem Only**: All scan indexes, hashes, and configuration records are maintained inside the local SQLite database. No external cloud object storage (S3/GCS/Azure Blob) is utilized.

**Caching:**
- **Persistent Cryptographic File Hash Cache**: SQLite table `file_hashes` caches computed SHA-256 hashes against `(path, size_bytes, modified_at)` to eliminate redundant hashing across duplicate scans ([`src-tauri/src/services/analysis/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/analysis/mod.rs#L302)).
- **In-Memory Page Cache**: 64 MB RAM cache allocated directly inside SQLite via `cache_size = -64000`.
- **Global Package & Tool Cache Detection**: Queries local disk caches for Cargo, npm, pnpm, Yarn, pip, uv, Maven, Gradle, Android SDK, and Xcode DerivedData in [`src-tauri/src/services/developer/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/developer/mod.rs#L410-L448).

## Authentication & Identity

**Auth Provider:**
- **None / Custom Local-Only**: As an offline desktop utility, DiskAtlas has no authentication provider, user login, token exchange, or cloud account requirement. Access is bounded entirely by the operating system user permissions under which the application process is launched.

## Monitoring & Observability

**Error Tracking:**
- **None**: No external crash reporting services (such as Sentry, Bugsnag, or Datadog) are included.
- **Application Error Pipeline**: Backend failures are modeled with `thiserror` as `AppError` in [`src-tauri/src/error.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/error.rs), serialized with predictable error codes (`DATABASE_ERROR`, `IO_ERROR`, `PERMISSION_DENIED`, `VALIDATION_FAILED`, etc.), and converted by [`src/utils/tauri.ts`](file:///Users/harshitbisht/Desktop/DiskAtlas/src/utils/tauri.ts) into standardized TypeScript error objects.

**Logs:**
- **Structured Tracing**: Configured via `tracing` and `tracing-subscriber` in [`src-tauri/src/lib.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/lib.rs#L39-L42). Logs to stdout/stderr with configurable `RUST_LOG` environment filter (default: `"info,diskatlas=debug"`).

## CI/CD & Deployment

**Hosting:**
- **Local Desktop Binary**: Distributed directly as platform-specific packages:
  - macOS: Universal DMG / App bundle.
  - Windows: NSIS Installer (`.exe`) and WiX Installer (`.msi`).
  - Linux: Standalone AppImage or system packages.

**CI Pipeline:**
- **None detected**: Repository currently maintains GitHub issue/PR templates in [`.github/`](file:///Users/harshitbisht/Desktop/DiskAtlas/.github/), but no automated `.github/workflows/` CI/CD actions are configured.

## Environment Configuration

**Required env vars:**
- None. DiskAtlas runs out of the box with zero environment variables.

**Optional development vars:**
- `TAURI_DEV_HOST`: Specifies remote or network dev server IP address for Vite HMR in [`vite.config.ts`](file:///Users/harshitbisht/Desktop/DiskAtlas/vite.config.ts#L4).
- `RUST_LOG`: Configures log output verbosity for `tracing-subscriber`.

**Secrets location:**
- No secrets, tokens, or private credentials are stored or needed.

## Webhooks & Callbacks

**Incoming:**
- None.

**Outgoing:**
- None.

## Tauri IPC Bridge & Registered Commands

Tauri v2 acts as the communication bus between the React frontend and the Rust desktop engine. All commands are registered via `tauri::generate_handler!` in [`src-tauri/src/lib.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/lib.rs#L92-L131) and invoked from the frontend using `safeInvoke<T>` in [`src/utils/tauri.ts`](file:///Users/harshitbisht/Desktop/DiskAtlas/src/utils/tauri.ts).

### Registered IPC Commands (32 total)

| Command Category | Command Name | Source File | Purpose |
|------------------|--------------|-------------|---------|
| **System & Diagnostics** | `get_system_info` | [`src-tauri/src/commands/system.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/system.rs#L7) | Host OS, kernel, arch, CPU cores, memory stats |
| | `get_database_health` | [`src-tauri/src/commands/database.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/database.rs#L7) | SQLite connectivity, migration version, table count |
| **Drives & Partitions** | `get_drives` | [`src-tauri/src/commands/drives.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/drives.rs#L8) | Enumerate mounted storage partitions & sync to DB |
| | `get_drive_details` | [`src-tauri/src/commands/drives.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/drives.rs#L26) | Detailed bus type, sector sizes, encryption flags |
| | `list_drives` | [`src-tauri/src/commands/drives.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/drives.rs#L32) | Compatibility alias for drive enumeration |
| **Scanner Engine** | `start_scan` | [`src-tauri/src/commands/scanner.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/scanner.rs#L11) | Launch dedicated background thread scan |
| | `pause_scan` | [`src-tauri/src/commands/scanner.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/scanner.rs#L27) | Atomically pauses filesystem scan traversal |
| | `resume_scan` | [`src-tauri/src/commands/scanner.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/scanner.rs#L33) | Resumes paused scan via thread notification |
| | `cancel_scan` | [`src-tauri/src/commands/scanner.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/scanner.rs#L39) | Aborts scan and marks session cancelled |
| | `get_scan_progress` | [`src-tauri/src/commands/scanner.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/scanner.rs#L45) | Queries live in-memory scanning progress |
| | `get_scan_files` | [`src-tauri/src/commands/scanner.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/scanner.rs#L59) | Paginated query of indexed files for a session |
| **History & Snapshots** | `get_scan_sessions` | [`src-tauri/src/commands/history.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/history.rs) | List historical scan sessions with pagination |
| | `get_session_details` | [`src-tauri/src/commands/history.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/history.rs) | Aggregated category summaries and top directories |
| | `delete_scan_session` | [`src-tauri/src/commands/history.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/history.rs) | Cascading deletion of scan session and file index |
| | `get_storage_snapshots` | [`src-tauri/src/commands/history.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/history.rs) | Retrieve historical drive capacity snapshots |
| | `create_manual_snapshot`| [`src-tauri/src/commands/history.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/history.rs) | Capture a point-in-time capacity checkpoint |
| | `compare_scans` | [`src-tauri/src/commands/history.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/history.rs) | Differential comparison of two sessions |
| **Classification** | `get_storage_analysis` | [`src-tauri/src/commands/classification.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/classification.rs) | Category distribution (OS, Dev, Media, Apps) |
| | `classify_session` | [`src-tauri/src/commands/classification.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/classification.rs) | Trigger batch classification heuristic pipeline |
| | `get_category_files` | [`src-tauri/src/commands/classification.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/classification.rs) | Filter files by classified storage category |
| | `get_category_overrides`| [`src-tauri/src/commands/classification.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/classification.rs) | Retrieve user-defined category matching rules |
| | `add_category_override`| [`src-tauri/src/commands/classification.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/classification.rs) | Register custom pattern override |
| | `delete_category_override`| [`src-tauri/src/commands/classification.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/classification.rs) | Remove custom pattern override |
| **Developer Storage** | `get_developer_storage`| [`src-tauri/src/commands/developer.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/developer.rs) | Discover code repos, dependency/cache/build sizes |
| | `get_docker_storage` | [`src-tauri/src/commands/developer.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/developer.rs) | Docker image, container, volume, and build cache |
| | `get_global_tool_caches`| [`src-tauri/src/commands/developer.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/developer.rs) | npm, pnpm, Cargo, Gradle, pip, uv, Xcode sizes |
| **Advanced Analysis** | `get_large_files` | [`src-tauri/src/commands/analysis.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/analysis.rs) | Top files exceeding size thresholds (e.g. 50MB) |
| | `get_hidden_files` | [`src-tauri/src/commands/analysis.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/analysis.rs) | Distinguish dotfiles, user hidden, system files |
| | `find_duplicates` | [`src-tauri/src/commands/analysis.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/analysis.rs) | Staged size-first candidate streaming SHA-256 |
| | `get_forgotten_files` | [`src-tauri/src/commands/analysis.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/analysis.rs) | Old downloads, installers, archives, stale builds |
| | `get_unknown_storage` | [`src-tauri/src/commands/analysis.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/analysis.rs) | Unclassified files and parent directories rollup |
| **Storage Intelligence** | `get_storage_intelligence`| [`src-tauri/src/commands/intelligence.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/intelligence.rs) | High-impact findings, reclaimable bytes metrics |
| | `get_storage_recommendations`| [`src-tauri/src/commands/intelligence.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/intelligence.rs) | Actionable cleanup recommendations with risk levels |
| **Visualization** | `get_hierarchical_tree`| [`src-tauri/src/commands/visualization.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/visualization.rs) | Hierarchical tree nodes for Treemap & Sunburst |
| | `get_storage_analytics`| [`src-tauri/src/commands/visualization.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/visualization.rs) | Aggregate distributions (extensions, apps, dirs) |
| **Safe Cleanup Center** | `get_cleanup_plan` | [`src-tauri/src/commands/cleanup.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/cleanup.rs) | Pre-validated cleanup items with safeguard flags |
| | `execute_cleanup` | [`src-tauri/src/commands/cleanup.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/cleanup.rs) | Safeguard-verified execution via Trash / Recycle Bin |
| | `get_cleanup_audit_history`| [`src-tauri/src/commands/cleanup.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/commands/cleanup.rs) | Paginated audit trail of all cleanup actions |

### Tauri Event Streams (Rust to Frontend)

Tauri events stream real-time background scanning updates from the dedicated worker thread to frontend listeners registered in [`src/stores/scanStore.ts`](file:///Users/harshitbisht/Desktop/DiskAtlas/src/stores/scanStore.ts#L354-L434):

- **`scan://progress`** (Payload: `ScanProgress`):
  - Emitted from [`src-tauri/src/services/scanner/engine.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/engine.rs#L67).
  - Emits active session metrics (`filesScanned`, `dirsScanned`, `bytesScanned`, `errorCount`, `currentPath`, `elapsedMillis`).
  - Throttled to a maximum frequency of once every 100 milliseconds to avoid starving the UI thread.
- **`scan://completed`** (Payload: `ScanSummary`):
  - Emitted from [`src-tauri/src/services/scanner/engine.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/engine.rs#L640).
  - Signals scan conclusion and provides final file/directory counts, total bytes, and duration.
- **`scan://error`** (Payload: `string`):
  - Emitted from [`src-tauri/src/services/scanner/manager.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/manager.rs#L136).
  - Notifies frontend of fatal unrecoverable scanner errors.

## Operating System & Filesystem Integrations

### 1. Drive & Partition Detection
- **Windows Subsystem** ([`src-tauri/src/services/drive_detector.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/drive_detector.rs#L102-L349)):
  - Directly binds to Win32 Kernel APIs (`GetLogicalDriveStringsW`, `GetDriveTypeW`, `GetVolumeInformationW`, `GetDiskFreeSpaceExW`, `GetDiskFreeSpaceW`).
  - Correctly maps `DRIVE_REMOVABLE`, `DRIVE_FIXED`, `DRIVE_REMOTE`, `DRIVE_CDROM`, and `DRIVE_RAMDISK`.
- **macOS & Unix Subsystem** ([`src-tauri/src/services/drive_detector.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/drive_detector.rs#L353-L556)):
  - Uses `sysinfo::Disks`.
  - Filters out internal APFS firmlinks, pseudo-filesystems, and sub-volumes (`/System/Volumes`, `/private/var/vm`, `/dev`, `/Volumes/Recovery`, `auto_home`).
  - Resolves boot container capacity on macOS via `diskutil info /`.

### 2. High-Performance Filesystem Scanner
- **Dedicated OS Thread**: Background scans are dispatched to a dedicated OS thread via `std::thread::Builder::new().name("diskatlas-scan-...").spawn(...)` in [`src-tauri/src/services/scanner/manager.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/manager.rs#L115) to prevent starving Tauri's Tokio async executor with blocking filesystem I/O (`std::fs::read_dir`, `symlink_metadata`).
- **Symlink & Inode Loop Protection**:
  - Unix: Tracks visited inode sets `(u64, u64)` via `std::os::unix::fs::MetadataExt::ino` and `dev` in [`src-tauri/src/services/scanner/engine.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/engine.rs#L112-L114).
  - Non-Unix: Tracks canonical path sets.
- **Batch Persistence**: Indexes files and directories in transactional chunks of 2,500 records before committing to SQLite.
- **Atomic Pause/Cancel Controls**: Utilizes `std::sync::atomic::AtomicU8` and `tokio::sync::Notify` for instant pause, resume, and cancellation.

### 3. Safe Cleanup & Reversible Trash Integration
- **Cross-Platform Trash Integration**: Integrates the `trash` crate (`trash::delete(path)`) in [`src-tauri/src/services/cleanup/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs#L292) to send files to the OS Recycle Bin / Trash by default, ensuring all cleanups are reversible.
- **Multi-Layered Safeguards** ([`src-tauri/src/services/cleanup/safeguards.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/safeguards.rs)):
  - **Drive & Filesystem Roots**: Blocks root mounts (`/`, `C:\`).
  - **Windows Boot & Kernel**: Blocks `bootmgr`, `bcd`, `ntoskrnl.exe`, `hal.dll`, `winload.exe`, `pagefile.sys`, `hiberfil.sys`, `swapfile.sys`.
  - **Core Subsystems**: Blocks `Windows\System32\drivers`, `.sys` kernel drivers, `Windows\WinSxS`, `Windows\servicing`, `System Volume Information`, `Recovery`.
  - **Unix / macOS Protected Roots**: Blocks `/system`, `/system/library`, `/usr`, `/usr/bin`, `/usr/sbin`, `/bin`, `/sbin`, `/etc`, `/dev`, `/proc`, `/sys`, `/private/var`, `/library/extensions`.
  - **Cloud Synchronization Warnings**: Detects target paths inside OneDrive, Google Drive, Dropbox, iCloud Drive, Nextcloud, and Box to warn the user before deleting files that would propagate to synced cloud storage.
  - **Mandatory User Confirmation Phrase**: Cleanup operations strictly require the explicit confirmation string `"CONFIRM_CLEANUP"`. Automated or unconfirmed executions return a validation error.
  - **Audit Logging**: Every cleanup attempt records a permanent entry into the SQLite `cleanup_audit_events` table documenting batch ID, path, category, risk level, method (`recycle_bin` vs `permanent_delete`), status (`success`, `blocked_security`, `permission_denied`, `in_use`), and error messages.

---

*Integration audit: 2026-11-06*
