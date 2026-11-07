# DiskAtlas: Project Specification

## 1. Project Overview

**DiskAtlas** is a high-performance, local-first storage intelligence and disk analyzer designed specifically for software engineers, systems developers, and power users. Modern development tooling (Docker caches, nested `node_modules`, Python virtual environments, Rust `target/` directories, Xcode DerivedData, Android SDKs) rapidly consumes dozens to hundreds of gigabytes. DiskAtlas provides an air-gapped, zero-telemetry, developer-aware utility that indexes storage at hardware speeds (>50,000 files/sec), visualizes space distribution with interactive D3 charts, classifies build artifacts across major ecosystems, and offers safe, reversible, audit-logged cleanups.

- **License**: MIT
- **Platforms**: macOS (APFS/HFS+), Windows 10/11 (NTFS/ReFS/FAT32), Linux (ext4/Btrfs/XFS)
- **Architecture**: Hybrid desktop app combining a native multi-threaded Rust core (Tauri 2.2) and a reactive web UI (React 19, TypeScript, Tailwind CSS, D3.js).

---

## 2. Technology Stack & Key Dependencies

### Frontend
- **Framework**: React 19.0.0 with TypeScript 5.6.3 and Vite 6.0.7
- **Styling**: Tailwind CSS 3.4.17 with CSS variables and custom scrollbar styling
- **Visualizations**: D3.js 7.9.0 (Squarified Treemaps, Radial Sunbursts) & Recharts 2.15.0
- **Icons & UI Primitives**: Lucide React 0.469.0, Canvas-confetti
- **State Management**: Reactive React hooks, custom stores, and Tauri IPC event listeners

### Backend (Rust Core)
- **Framework & IPC**: Tauri 2.2.0 (`tauri`, `tauri-build`, `tauri-plugin-dialog`, `tauri-plugin-opener`)
- **Concurrency & Async**: Tokio 1.42 (`rt-multi-thread`, `sync`, `macros`), Rayon 1.10 (data parallelism), Crossbeam Channels
- **Database**: Embedded SQLite 3 via Rusqlite 0.32.1 (bundled, WAL mode, transaction batching)
- **Filesystem & System Metrics**: `sysinfo` 0.33.0, `walkdir` 2.5.0, `trash` 5.2.0 (native OS Recycle Bin / Trash)
- **Serialization & Hashing**: Serde & Serde JSON 1.0, Sha2 0.10.8 (cryptographic duplicate verification), UUID 1.11, Chrono 0.4.39

---

## 3. Core Architectural Modules

```
+-------------------------------------------------------------------------------+
|                            DiskAtlas UI (React 19)                            |
|    D3 Treemap / Sunburst  *  Developer Workspace  *  Safe Cleanup Center      |
+---------------------------------------▲---------------------------------------+
                                        | Tauri 2 IPC (Type-Safe RPC & Events)
+---------------------------------------▼---------------------------------------+
|                           DiskAtlas Native Core (Rust)                        |
|  +--------------------+  +----------------------+  +-----------------------+  |
|  |   Scanner Engine   |  |  Artifact Classifier |  |  Analysis & Duplicate |  |
|  | (Dedicated Thread) |  |   (7+ Ecosystems)    |  |   (4-Stage SHA-256)   |  |
|  +---------+----------+  +----------+-----------+  +-----------+-----------+  |
|            |                        |                          |              |
|  +---------▼------------------------▼--------------------------▼-----------+  |
|  |                 Embedded SQLite Database (WAL Mode)                     |  |
|  |     Files  *  Directories  *  Snapshots  *  Cleanup Audit Log           |  |
|  +----------------------------------+--------------------------------------+  |
|                                     |                                         |
|  +----------------------------------▼--------------------------------------+  |
|  |           Safe Cleanup Engine & System Protection Safeguards            |  |
|  |        TOCTOU Checks  *  OS Whitelist  *  Reversible OS Trash           |  |
|  +-------------------------------------------------------------------------+  |
+-------------------------------------------------------------------------------+
```

1. **Scanner Engine (`src-tauri/src/services/scanner/`)**:
   - Traverses filesystems on a dedicated OS thread.
   - Respects filesystem boundaries, bypasses symlink recursion and Windows NTFS junctions.
   - Emits throttled progress events (every 250ms) to maintain smooth 60 FPS UI rendering.
   - Flushes file records into SQLite in batched transactions (5,000 files/batch).
2. **Developer Intelligence Engine (`src-tauri/src/services/developer/`, `models/intelligence.rs`)**:
   - Classifies build caches and project dependencies across Node.js, Python, Rust, Docker, Java/Android, C/C++, and IDEs.
   - Extracts workspace metadata (e.g. package identities from `package.json`).
3. **Storage Analysis & Duplicate Finder (`src-tauri/src/services/analysis/`)**:
   - 4-stage duplicate detection pipeline: Size match -> Metadata check -> Block header check -> Full cryptographic SHA-256 hash.
   - Aggregates file categories, largest consumers, and extension distributions.
4. **Safe Cleanup Center & Audit Trail (`src-tauri/src/services/cleanup/`)**:
   - Zero automated deletions: requires explicit review and confirmation phrase `CONFIRM_CLEANUP`.
   - Four layers of defense: OS & boot whitelist, TOCTOU re-validation, reversible trash bin default, and immutable SQLite audit logging.
5. **Storage History & Snapshots (`src-tauri/src/services/history/`)**:
   - Periodic drive snapshots tracking storage growth, diffing filesystem shifts over time.

---

## 4. Fundamental Invariants & Guarantees

- **100% Local-First / Zero Telemetry**: Air-gapped; no external HTTP requests, telemetry SDKs, or background analytics.
- **Strict Read-Only Scanning**: Scanner processes never mutate or touch file permissions/contents during indexing.
- **Destructive Action Safety**: Filesystem mutations are strictly guarded, preventing accidental destruction of OS boot, kernel, driver, or critical system assets.
- **Data Durability**: SQLite operates with Write-Ahead Logging (`PRAGMA journal_mode = WAL`), memory-mapped I/O, and foreign key verification.

