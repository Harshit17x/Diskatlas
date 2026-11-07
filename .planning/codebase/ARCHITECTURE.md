<!-- refreshed: 2026-11-06 -->
<Architecture>
**Analysis Date:** 2026-11-06

## System Overview

DiskAtlas is an ultra-fast, local-first disk intelligence and storage management desktop application engineered with Tauri v2, Rust, React 19, and SQLite. The architecture maintains a strict boundary between an unprivileged webview presentation layer and a high-performance native core executing concurrent system-level operations.

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Frontend Presentation Layer                           │
│     React 19 · TypeScript 5.7 · Tailwind CSS · TanStack Table · D3.js      │
├──────────────────────┬──────────────────────┬───────────────────────────────┤
│   UI Shell & Views   │  Zustand Store Mesh  │  Interactive Visualizations   │
│   `src/features/`    │   `src/stores/`      │ `src/components/visualizations│
└──────────┬───────────┴──────────┬───────────┴───────────────┬───────────────┘
           │                      │                           │
           │ `safeInvoke(cmd)`    │ `listen(event)`           │ IPC Queries
           ▼                      ▼                           ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Tauri v2 IPC & Bridge Layer                          │
│     `src-tauri/src/commands/` · `src-tauri/capabilities/default.json`       │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      │
         ┌────────────────────────────┼────────────────────────────┐
         │                            │                            │
         ▼                            ▼                            ▼
┌──────────────────┐        ┌──────────────────┐        ┌──────────────────┐
│  Scanner Engine  │        │ Analytics & ML   │        │ Cleanup Engine & │
│ (OS Worker Thread│        │  Intelligence    │        │ Safeguard Rails  │
│`services/scanner`│        │`services/analysi`│        │`services/cleanup`│
└────────┬─────────┘        └────────┬─────────┘        └────────┬─────────┘
         │                           │                           │
         └───────────────────────────┼───────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Persistence & Hardware Interface                         │
│  SQLite 3 (WAL mode) · `rusqlite` · `trash` crate · `sysinfo` Hardware API  │
│  `src-tauri/src/storage/` · `src-tauri/migrations/`                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| **Application Core** | Bootstraps Tauri runtime, initializes structured logging, sets up DB pool and managed state | `src-tauri/src/lib.rs` |
| **Scanner Manager** | Coordinates scan lifecycles, active session tracking, atomic pause/resume/cancel flags, recovery | `src-tauri/src/services/scanner/manager.rs` |
| **Scanner Engine** | Executes recursive filesystem traversal on a dedicated OS thread, bounds sparse files, aggregates directory rollups | `src-tauri/src/services/scanner/engine.rs` |
| **Classification Engine** | Deterministically categorizes files into 16 storage domains with priority matching and custom overrides | `src-tauri/src/services/classifier/rules.rs` |
| **Developer Intelligence** | Detects projects and caches across 10+ developer ecosystems (Node, Rust, Python, Docker, Git) | `src-tauri/src/services/developer/mod.rs` |
| **Analysis Engine** | Queries large, hidden, forgotten files; executes staged 3-tier duplicate file detection with hash caching | `src-tauri/src/services/analysis/mod.rs` |
| **Intelligence Engine** | Evaluates storage heuristics, scores recommendations by risk and recoverable bytes | `src-tauri/src/services/intelligence/mod.rs` |
| **Cleanup Engine** | Orchestrates safe file deletion via system Trash or permanent removal with strict OS safeguard validation | `src-tauri/src/services/cleanup/mod.rs` |
| **Safeguards Guardrail** | Enforces OS boundary protection (kernel, boot, drivers, recovery, cloud sync roots) | `src-tauri/src/services/cleanup/safeguards.rs` |
| **Storage & Migrator** | Manages SQLite connection pool with WAL mode and runs embedded transactional schema migrations | `src-tauri/src/storage/connection.rs` |
| **Hardware & Drive Detector**| Queries mount points, drive types (SSD/HDD/NVMe/USB), partitions, and filesystem metrics via `sysinfo` | `src-tauri/src/services/drive_detector.rs` |
| **Visualization Service** | Generates bounded hierarchical node trees for radial Sunburst and squarified Treemap charts | `src-tauri/src/services/visualization/mod.rs` |
| **Frontend Shell** | Coordinates top-level layout (Sidebar, Header, StatusBar, CommandPalette) and tab-based routing | `src/app/App.tsx` |
| **IPC Bridge Utility** | Provides type-safe wrappers around Tauri `invoke` and maps backend Rust errors into `AppError` | `src/utils/tauri.ts` |
| **Scan Store** | Manages scan state, UI modal controls, throttled IPC event listeners, and fallback heartbeat polling | `src/stores/scanStore.ts` |
| **Visualization Store** | Manages D3 chart data, breadcrumb hierarchies, and directory drill-down/up navigation | `src/stores/visualizationStore.ts` |

## Pattern Overview

**Overall:** Hybrid Hexagonal / Layered Architecture with Event-Driven IPC and Dedicated Thread Offloading.

**Key Characteristics:**
- **Decoupled Asynchronous IPC:** All frontend-to-backend communication occurs over Tauri v2 JSON-RPC invoke channels (`safeInvoke`) or typed IPC event streams (`scan://progress`, `scan://completed`, `scan://error`).
- **Dedicated Thread File System I/O:** Heavy recursive filesystem scanning is isolated on an OS thread spawned by `std::thread::Builder`, deliberately preventing blocking POSIX/Win32 I/O from starving the Tokio async runtime.
- **Microsecond In-Memory Classification:** Files are classified into categories during the initial directory traversal pass (`RuleMatcher::classify`), avoiding heavy post-scan SQL table scans across millions of rows.
- **Local-First SQLite Repository:** All session data, directories, files, snapshots, and audit events are persisted locally in SQLite configured with Write-Ahead Logging (`WAL`), 64 MB RAM cache, and memory-mapped I/O. Zero data leaves the user machine.
- **Multi-Store State Separation:** Frontend state is divided across 10 specialized Zustand stores, isolating high-frequency progress updates (`scanStore`) from static drive listings (`driveStore`) or chart calculations (`visualizationStore`).

## Layers

### 1. Presentation Layer (Webview UI)
- **Purpose:** Renders the user interface, responds to user gestures, updates visual charts, and displays filesystem analytics.
- **Location:** `src/`
- **Contains:** React 19 functional components, Zustand stores, Lucide icons, D3 visualization wrappers, Tailwind CSS styling.
- **Depends on:** Tauri v2 JavaScript API (`@tauri-apps/api/core`, `@tauri-apps/api/event`), TanStack Table, D3.js.
- **Used by:** End-user through the native application window.

### 2. IPC & Command Bridge Layer
- **Purpose:** Exposes typed RPC endpoints from Rust to the Webview, validates permissions, and handles serialization.
- **Location:** `src-tauri/src/commands/` and `src-tauri/capabilities/default.json`
- **Contains:** Tauri `#[command]` functions for 34 endpoints covering scanner, drives, classification, developer, intelligence, cleanup, visualization, and history.
- **Depends on:** Core services (`src-tauri/src/services/`), managed application state (`DbPool`, `ScanManager`).
- **Used by:** Frontend stores via `src/utils/tauri.ts`.

### 3. Core Domain & Service Layer
- **Purpose:** Implements all storage intelligence logic, directory traversal, deduplication, safeguard checking, and recommendation synthesis.
- **Location:** `src-tauri/src/services/`
- **Contains:** `ScanEngine`, `ScanManager`, `ClassifierService`, `DeveloperService`, `AnalysisService`, `IntelligenceService`, `CleanupService`, `VisualizationService`, `DriveDetector`, `SystemInspector`.
- **Depends on:** Data models (`src-tauri/src/models/`), storage abstraction (`src-tauri/src/storage/`), OS crates (`rusqlite`, `sysinfo`, `trash`, `sha2`).
- **Used by:** Tauri command handlers.

### 4. Persistence & Storage Layer
- **Purpose:** Manages relational database storage, migration sequencing, cached statement execution, and WAL configuration.
- **Location:** `src-tauri/src/storage/` and `src-tauri/migrations/`
- **Contains:** `DbPool`, `Migrator`, 8 embedded SQL migration scripts (`0001_initial_schema.sql` through `0008_cleanup_audit_history.sql`).
- **Depends on:** `rusqlite` with `bundled` feature.
- **Used by:** All services requiring persistence.

### 5. OS & Hardware Abstraction Layer
- **Purpose:** Direct interaction with operating system storage systems, filesystems, recycle bins, and hardware metrics.
- **Location:** Native Rust crates integrated in `src-tauri/Cargo.toml`.
- **Contains:** `std::fs`, `trash` (Recycle bin abstraction), `sysinfo` (mount points, memory, CPU, disk hardware metrics), `sha2` (cryptographic hashing).
- **Used by:** Service implementations.

## Data Flow

### Primary Request Path: Filesystem Scan Lifecycle

```text
User Click "Start Scan"
   │
   ▼
[ScanModal.tsx] ──> useScanStore.startScan(path, driveId)
                         │
                         ▼
                    safeInvoke('start_scan', { path, driveId })
                         │
                         ▼ [Tauri IPC]
           [src-tauri/src/commands/scanner.rs::start_scan]
                         │
                         ▼
     [ScanManager::start_scan] ──> Generates session_id (UUID v4)
                               ──> Registers ActiveScanHandle (Atomic flags)
                               ──> Spawns dedicated OS worker thread
                               │
                               ▼
     [ScanEngine::run_scan (OS Thread)]
          ├── Initializes session record in SQLite (drives & scan_sessions tables)
          ├── Traverses dir_stack (LIFO, skips APFS sealed system & loop firmlinks)
          ├── Extracts metadata (timestamps, symlink status, sparse allocated size)
          ├── Runs inline classification via RuleMatcher::classify (zero DB cost)
          ├── Aggregates recursive directory rollups in dir_map
          │
          ├── [Throttled Progress Emit (every 100ms / dir transition)]
          │        │
          │        ▼
          │   app_handle.emit("scan://progress", &ScanProgress)
          │        │
          │        ▼ [Tauri Event IPC]
          │   useScanStore.listen("scan://progress") ──> Updates UI progress & counter
          │
          ├── Flushes batches to SQLite (2500 files per transaction)
          │
          ├── Post-Scan Intelligence Pipeline:
          │     1. ClassifierService::classify_and_persist_session
          │     2. DeveloperService::analyze_developer_storage
          │     3. IntelligenceService::generate_intelligence_profile
          │     4. Storage snapshots & category summaries generation
          │
          ▼
     app_handle.emit("scan://completed", &ScanSummary)
          │
          ▼ [Tauri Event IPC]
     useScanStore.listen("scan://completed")
          ├── useScanStore.fetchScanFiles(sessionId)
          ├── useDriveStore.fetchDrives() & fetchDbHealth()
          └── useVisualizationStore.fetchAnalytics(sessionId)
```

### Secondary Request Path: Safe Cleanup Execution Lifecycle

```text
User Selects Items to Clean in [CleanupCenterView.tsx]
   │
   ▼
useCleanupStore.executeCleanup({ batchId, items, permanent, dryRun })
   │
   ▼
safeInvoke('execute_cleanup', request)
   │
   ▼ [Tauri IPC]
[src-tauri/src/commands/cleanup.rs::execute_cleanup]
   │
   ▼
[CleanupService::execute_cleanup]
   │
   ├── Phase 1: Safeguard Verification (safeguards::is_system_critical_path)
   │   - Checks root drive bounds, Windows System32/WinSxS, macOS /System, /Library
   │   - If critical: blocks immediately, records 'blocked_security' in audit log
   │
   ├── Phase 2: Execution Mode
   │   - If dry_run = true: simulates outcome, checks file existence and permissions
   │   - If permanent = true: invokes std::fs::remove_file or remove_dir_all
   │   - Default (reversible): invokes trash::delete(path) to send to OS Recycle Bin
   │
   ├── Phase 3: Transactional Audit Recording
   │   - Inserts row into cleanup_audit_events table (batch_id, size, method, status)
   │
   ▼
Returns ExecuteCleanupResponse { totalCleanedBytes, successCount, failedCount, ... }
   │
   ▼
useCleanupStore receives response ──> Updates audit banner & triggers fetchDrives()
```

### Staged Duplicate Detection Flow

1. **Size Partitioning:** `AnalysisService::find_duplicates` groups all scanned files in SQLite having identical `size_bytes >= min_size` and identical parent/extension constraints.
2. **Partial Prefix Hash (Tier 1):** For size candidates, reads the first 4,096 bytes and computes a partial SHA-256 hash. Files with divergent headers are immediately excluded without reading gigabytes of data.
3. **Full Cryptographic Hash (Tier 2):** Remaining colliding candidates undergo full SHA-256 hashing.
4. **Hash Table Caching:** Hashes are saved in SQLite `file_hashes(path, size_bytes, modified_at, sha256)` indexed by `size_bytes` and `sha256`. Unmodified files in future scans reuse existing hashes instantly.

**State Management Strategy:**
- Single source of truth in native SQLite database for all heavy datasets.
- Ephemeral live scan progress stored in atomic shared structs (`Arc<Mutex<ScanProgress>>`).
- Client-side Zustand stores consume paginated slices (e.g. 50 items/page) to maintain constant 60 FPS UI performance.
- Resilient polling fallback: in addition to event listeners, `useScanStore` maintains a 200ms polling heartbeat against `get_scan_progress` ensuring UI synchronization never freezes even if webview IPC events drop under heavy load.

## Key Abstractions

### `ActiveScanHandle`
- **Purpose:** Represents an active background scan in memory, providing handle references for external control.
- **Location:** `src-tauri/src/services/scanner/manager.rs`
- **Pattern:** Handle pattern combining `AtomicU8` status flag, `tokio::sync::Notify` waiter, and `Arc<Mutex<ScanProgress>>`.

### `RuleMatcher`
- **Purpose:** Pure function deterministic classifier mapping file attributes (path, name, extension, hidden, system) into exactly one of 16 categories with subcategory, app name, and confidence score.
- **Location:** `src-tauri/src/services/classifier/rules.rs`
- **Pattern:** Specification / Chain-of-Rules pattern with strict precedence to prevent double-counting.

### `DbPool`
- **Purpose:** Encapsulates the SQLite connection, configuring mandatory PRAGMA optimizations and providing thread-safe connection sharing.
- **Location:** `src-tauri/src/storage/connection.rs`
- **Pattern:** Thread-safe Repository / Connection Pool wrapper with `Arc<Mutex<Connection>>`.

### `safeInvoke`
- **Purpose:** Type-safe bridge on the frontend for calling Tauri IPC commands, normalizing both native errors and transport failures into structured `AppError`.
- **Location:** `src/utils/tauri.ts`
- **Pattern:** Result/Either Wrapper pattern (`{ ok: true, data } | { ok: false, error }`).

## Entry Points

### 1. Native Desktop Entry Point
- **Location:** `src-tauri/src/main.rs`
- **Triggers:** OS execution of the compiled DiskAtlas binary.
- **Responsibilities:** Configures Windows subsystem (`windows_subsystem = "windows"` to suppress console prompt in release mode) and hands execution to `diskatlas_lib::run()`.

### 2. Desktop Core Orchestration Entry Point
- **Location:** `src-tauri/src/lib.rs`
- **Triggers:** Invoked by `main.rs`.
- **Responsibilities:**
  - Configures `tracing-subscriber` structured logging with environment filter.
  - Resolves primary `app_data_dir` with local directory fallback (`./.diskatlas_data`).
  - Initializes `DbPool` and runs database migrations through `Migrator::run(&mut conn)`.
  - Instantiates `ScanManager` and auto-recovers interrupted scans from previous unclean shutdowns.
  - Registers managed state (`DbPool`, `ScanManager`) and 34 IPC commands with `tauri::Builder`.

### 3. Frontend Webview Entry Point
- **Location:** `src/app/main.tsx`
- **Triggers:** Loaded by `index.html` inside the Tauri Webview.
- **Responsibilities:** Mounts React 19 root into `#root` DOM element with `React.StrictMode` and imports global stylesheet (`src/app/index.css`).

### 4. Application UI Controller
- **Location:** `src/app/App.tsx`
- **Triggers:** Rendered by `main.tsx`.
- **Responsibilities:**
  - Initializes global theme class (`dark` mode toggle on `document.documentElement`).
  - Dispatches initial background fetches: `fetchDrives()`, `fetchSystemInfo()`, `fetchDbHealth()`.
  - Attaches Tauri IPC event listeners (`setupEventListeners`).
  - Registers global navigation shortcuts (Cmd+1 to Cmd+0, Cmd+,).
  - Renders UI shell (`Header`, `Sidebar`, `StatusBar`, `CommandPalette`, modals) and active tab view.

## Architectural Constraints

- **Threading Isolation:** Filesystem scanning and disk metadata calls are strictly blocking synchronous operations. They must never run on the Tokio asynchronous executor thread pool; doing so causes async runtime starvation and freezes webview IPC. The scan engine is always spawned on a dedicated OS thread via `std::thread::Builder`.
- **Global State Synchronization:** SQLite connections are encapsulated inside `Arc<Mutex<rusqlite::Connection>>`. Multiple threads coordinate through short-lived lock acquisitions. SQLite WAL mode ensures readers do not block writers and writers do not block readers.
- **Circular Imports:** Rust modules are organized into strictly acyclic directory layers: `models` -> `storage` -> `services` -> `commands` -> `lib`. No service imports command modules. On the frontend, Zustand stores avoid direct static cross-imports; cross-store actions use `import('./store').then(...)` or `useStore.getState().action()`.
- **Memory Bounding during Scans:** Traversal avoids accumulating full file trees in RAM. Files are streamed directly to SQLite in 2,500-item batch transactions. Directory sizes are aggregated using a flat in-memory hashmap keyed by path string and rolled up bottom-up in a single pass before flushing to the database.
- **Security Boundaries:** Cleanup operations never execute untrusted paths. All target paths are checked against `safeguards::is_system_critical_path` before deletion. Destructive permanent deletions require explicit user confirmation parameters.

## Anti-Patterns

### Anti-Pattern 1: Spawning Blocking Filesystem Traversal inside `tokio::spawn`

**What happens:** Using `tokio::spawn(async move { ... })` and calling `std::fs::read_dir` inside the async task.
**Why it's wrong:** Tokio worker threads are cooperative. Blocking system calls monopolize the worker thread, causing Tauri's IPC event dispatcher to starve and locking the UI webview.
**Do this instead:** Spawn a dedicated native OS thread using `std::thread::Builder::new().name(...).spawn(...)` as demonstrated in `src-tauri/src/services/scanner/manager.rs:115-140`.

### Anti-Pattern 2: Unthrottled IPC Progress Emissions

**What happens:** Emitting `app_handle.emit("scan://progress", ...)` on every single encountered file (e.g. 100,000 times per second).
**Why it's wrong:** Floods the IPC serialization bridge and saturates the webview JavaScript event queue, causing massive lag, memory spikes, and UI lockup.
**Do this instead:** Enforce strict throttling in `ScanEngine::emit_progress_if_needed` (`src-tauri/src/services/scanner/engine.rs:53`), emitting events only when 100ms has elapsed since the last emission or upon entering a new directory level.

### Anti-Pattern 3: Post-Scan Full Database Table Scans for Classification

**What happens:** Inserting unclassified files into SQLite and then executing massive SQL `UPDATE files SET category = ...` queries across millions of rows after traversal completes.
**Why it's wrong:** Causes lengthy 5 to 15 minute post-scan disk churn, locks the SQLite database, and delays user results.
**Do this instead:** Classify files in microsecond CPU time inline during file traversal using `RuleMatcher::classify` (`src-tauri/src/services/classifier/rules.rs`) and insert pre-classified rows directly in `src-tauri/src/services/scanner/engine.rs:471-495`.

## Error Handling

**Strategy:** Unified, strongly-typed error hierarchy mapped seamlessly across the Rust-to-TypeScript boundary without panics or swallowed exceptions.

**Patterns:**
- **Backend Error Mapping:** All native errors are captured in `AppError` enum (`src-tauri/src/error.rs`) utilizing `thiserror::Error`.
- **Custom JSON Serialization:** `AppError` implements custom Serde serialization converting Rust errors into standard JSON objects: `{ "code": "DATABASE_ERROR", "message": "...", "details": "..." }`.
- **Frontend Safe Invocations:** `safeInvoke<T>` in `src/utils/tauri.ts` wraps all `invoke<T>` calls in a `try/catch` block, guaranteeing that failures never crash React components and return `{ ok: false, error: AppError }`.
- **Cooperative Error Recovery:** Incomplete or interrupted scans from unclean process exits are detected and marked as `interrupted` upon subsequent startup by `ScanManager::recover_interrupted_scans` (`src-tauri/src/services/scanner/manager.rs:44`).
- **Filesystem Permission Resilience:** Inaccessible protected directories (e.g. macOS TCC-restricted paths, Windows System Volume Information) are captured gracefully as `RawErrorItem` records in the `scan_errors` table without aborting the parent scan session.

## Cross-Cutting Concerns

- **Logging & Diagnostics:** Structured asynchronous logging is powered by `tracing` and `tracing-subscriber` (`src-tauri/src/lib.rs:39-42`). Log events include session context, module paths, execution durations, and warning events.
- **Hardware & Platform Adaptability:** Conditional compilation attributes (`#[cfg(target_os = "macos")]`, `#[cfg(target_os = "windows")]`, `#[cfg(unix)]`) adapt paths, inode cycle checks, cluster allocations, and system exclusions automatically to the target platform.
- **Offline / Local-First Privacy:** Zero network telemetry, third-party trackers, or cloud dependencies exist in the codebase. All file intelligence, hash calculations, and cleanup actions occur strictly on the local machine.
- **Theme & Presentation Consistency:** UI theming is controlled by a centralized `theme` state in `uiStore.ts`, propagating the `.dark` class to `document.documentElement` and styling components with unified Tailwind CSS canvas/card/surface tokens.

---

*Architecture analysis: 2026-11-06*
