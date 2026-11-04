# DiskAtlas 🧭

> **Premium Local-First Storage Intelligence for Windows 10/11 & macOS.**  
> Built with **Tauri 2**, **Rust**, **React 19**, **TypeScript**, **SQLite (WAL)**, **D3.js**, and **Tailwind CSS**.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Tauri 2.0](https://img.shields.io/badge/Tauri-2.0-24C8DB.svg?logo=tauri&logoColor=white)](https://tauri.app)
[![React 19](https://img.shields.io/badge/React-19.0-61DAFB.svg?logo=react&logoColor=black)](https://react.dev)
[![Rust](https://img.shields.io/badge/Rust-1.80+-DEA584.svg?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Zero Telemetry](https://img.shields.io/badge/Telemetry-Zero-27AE82.svg)](docs/SECURITY_AUDIT.md)

---

## 🌟 Philosophy: Discover → Understand → Analyze → Recommend → Optimize

DiskAtlas is an ultra-fast, local-first storage analytics desktop application engineered for technical users, software engineers, and power users. It indexes drives at hardware speeds, classifies file ownership deterministically, uncovers dormant developer caches, visualizes hierarchical trees with D3, and recommends storage optimizations with strict operating system safeguards.

- 🔒 **Zero Remote Telemetry & Complete Local Privacy:** DiskAtlas is strictly offline. Zero remote telemetry, zero analytics SDKs, and zero file content transmissions.
- 🛡️ **Non-Destructive by Default:** Scanner routines operate exclusively with read-only metadata handles. Cleanup uses the native Windows Recycle Bin with strict confirmation barriers.
- ⚡ **High-Throughput Rust Core:** Multi-threaded filesystem traversal with bounded work stacks, avoiding stack overflows and loop traps on symlinks and Windows NTFS junctions.
- 💾 **Embedded SQLite WAL Engine:** Transactions buffered in memory-mapped WAL mode with 64 MB page caches, keeping memory footprint under 48 MB even during 500,000-file indexing runs.

---

## 🎨 Enterprise Design System & Palette

DiskAtlas rejects glassmorphism, decorative blur, and neon gimmicks in favor of a clean, high-density professional IDE aesthetic inspired by Linear and Raycast:

| Token | Hex | Role |
| :--- | :--- | :--- |
| **Background** | `#101114` | Global root desktop background |
| **Sidebar** | `#15171B` | Primary navigation pane |
| **Panel** | `#1B1E23` | Cards, elevated surfaces, and headers |
| **Elevated** | `#22262C` | Active rows, dropdowns, and hover states |
| **Border** | `#2D323A` | Subtle dividers and structural outlines |
| **Primary Text** | `#F1F3F5` | Headings, metrics, and primary values |
| **Secondary Text** | `#9BA3AE` | Labels, paths, and metadata |
| **Muted** | `#69717D` | Timestamps, icons, and deactivated elements |
| **Accent** | `#5687E8` | Interactive selections, charts, and primary buttons |
| **Success** | `#27AE82` | Low-risk indicators and completed scans |
| **Warning** | `#E5A54B` | Moderate-risk recommendations and warnings |
| **Danger** | `#DB6262` | Deletion triggers and critical risk alerts |

---

## 🚀 Key Modules & Capabilities

```
DiskAtlas Core Architecture
├── Hardware Discovery & Win32 Drive Engine
├── Universal Filesystem Scanner (Cycle-proof, Batched SQLite)
├── 16-Category Deterministic Classification Engine
├── Developer Storage Intelligence (Node.js, Rust, Python, Docker, C/C++, Java)
├── Advanced Analysis (Large Files, Staged Duplicates, Hidden/System, Forgotten Files)
├── Macro Intelligence & Evidence-Backed Recommendation Engine
├── D3 Visualizations (Squarified Treemaps, Radial Sunbursts, Analytics)
└── Safe Cleanup Center (Recycle Bin / Trash, Safeguards, Audit Trail)
```

### 1. Drive Discovery & Hardware Health
- Detects internal drives, NVMe/SATA SSDs, external HDDs, USB sticks, and virtual volumes.
- Retrieves volume labels, mount points, total capacity, free space, and filesystems (NTFS, FAT32, exFAT, APFS).

### 2. High-Throughput Universal Scanner
- Iterative work stack traversal that handles deep directories (>100 levels) without call-stack recursion.
- Bypasses directory junctions (`IO_REPARSE_TAG_MOUNT_POINT`) and cyclic symlinks.
- Captures logical sizes, Windows file attributes (`FILE_ATTRIBUTE_HIDDEN`, `FILE_ATTRIBUTE_SYSTEM`), and timestamps.
- Batch transactions (5,000 items/commit) achieve >25,000 files/sec on NVMe drives.
- Non-blocking pause, resume, and cancellation.

### 3. Developer Storage Intelligence
Specialized detection engine identifying deep developer build caches and tooling artifacts:
- **Node.js**: `node_modules`, `.next`, `.nuxt`, `.output`, `.vite`, `.turbo`, `.parcel-cache`, npm/pnpm/Yarn caches.
- **Python**: `.venv`, `venv`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, pip/uv/Conda caches.
- **Rust**: `target`, `.cargo/registry`, `.cargo/git`.
- **C/C++**: `cmake-build-debug`, `cmake-build-release`, `.vs`, compiler outputs.
- **Java**: `.m2/repository`, `.gradle/caches`, Android SDKs, and build directories.
- **Docker**: Inspects images, containers, volumes, and build cache via local engine API without touching internal files.

### 4. Advanced Storage Analysis
- **Staged Duplicate Finder**: 4-stage pipeline (Size match → Metadata check → SHA-256 hash match → Hash grouping).
- **Large Files Engine**: Configurable size threshold, extension filters, and sorting.
- **Hidden & System Items**: Distinguishes user-hidden files (`.dotfiles`, hidden attribute) from OS-protected components.
- **Forgotten Files**: Heuristic detection of stale installers, old downloads, and abandoned build trees.
- **Unknown Storage**: Discovers unclassified binary blobs and isolated subtrees.

### 5. D3 Visualizations & Analytics
- **D3 Squarified Treemap**: Proportional area representation with category color palettes, hover tooltips, and click drilldown.
- **D3 Radial Sunburst**: Multi-level concentric ring partition mapping directory depth and structural weight.
- **Analytics Dashboard**: Real-time progress bars and distribution charts for categories, file types, and top directories.

### 6. Safe Cleanup Center & Audit Trail
- **System Safeguards**: Automatically blocks deletion of Windows boot components (`bootmgr`, `bcd`), drivers (`*.sys`), kernel memory (`pagefile.sys`), and core directories (`WinSxS`).
- **Cloud Sync Warning**: Flags files inside OneDrive, Google Drive, Dropbox, or iCloud to alert users about deletion propagation.
- **Reversible Workflow**: Moves items to the Windows Recycle Bin / Trash by default.
- **Explicit Confirmation**: Requires typing `CONFIRM_CLEANUP` before executing destructive actions.
- **Cryptographic Audit Trail**: Records all actions into `cleanup_audit_events` with timestamps, methods, and statuses.

---

## 🛠️ Technology Stack

| Layer | Component | Version / Description |
| :--- | :--- | :--- |
| **Desktop Shell** | [Tauri](https://tauri.app) | 2.2.0 (Rust desktop runtime) |
| **Backend Core** | [Rust](https://www.rust-lang.org) | 2021 Edition, `tokio`, `sysinfo`, `trash`, `sha2`, `hex` |
| **Persistence** | [SQLite 3](https://www.sqlite.org) | `rusqlite` bundled, WAL mode, embedded migrations |
| **Frontend UI** | [React](https://react.dev) | 19.0.0 with [TypeScript](https://www.typescriptlang.org) |
| **Bundler** | [Vite](https://vitejs.dev) | 6.4.5 tuned for desktop IPC |
| **Visualizations** | [D3.js](https://d3js.org) | D3 Hierarchy, Squarify Treemap, Radial Sunburst |
| **Styling** | [Tailwind CSS](https://tailwindcss.com) | Custom theme adhering to dark enterprise palette |
| **State** | [Zustand](https://zustand-demo.pmnd.rs) | Decoupled feature stores |
| **Tables** | [TanStack Table](https://tanstack.com/table) | v8 virtualized data tables |

---

## 📁 Repository Structure

```
DiskAtlas/
├── src/                                   # Frontend Application (React 19 + TypeScript)
│   ├── app/                               # App entry (App.tsx, main.tsx, index.css)
│   ├── components/
│   │   ├── cleanup/                       # Cleanup modal, results banner
│   │   ├── common/                        # Card, Badge, Modal, DataTable primitives
│   │   ├── layout/                        # Sidebar, Header, StatusBar
│   │   └── visualizations/                # D3 Treemap, Sunburst, Analytics Dashboard
│   ├── features/
│   │   ├── analysis/                      # Storage Analysis & category breakdowns
│   │   ├── applications/                  # Application footprint inspector
│   │   ├── cleanup/                       # Safe Cleanup Center
│   │   ├── dashboard/                     # Executive storage dashboard
│   │   ├── developer/                     # Developer ecosystem storage intelligence
│   │   ├── drives/                        # Drive discovery & volume cards
│   │   ├── duplicates/                    # Staged cryptographic duplicate finder
│   │   ├── explorer/                      # D3 Treemap, Sunburst & File Registry Table
│   │   ├── hidden/                        # Hidden and system-protected items
│   │   ├── history/                       # Storage growth & scan snapshot comparisons
│   │   ├── intelligence/                  # Storage findings & recommendation cards
│   │   ├── scanner/                       # Scan initiation modal & live progress
│   │   └── settings/                      # Preferences, exclusions, database health
│   ├── stores/                            # Zustand feature state stores
│   ├── types/                             # TypeScript bindings matching Rust models
│   └── utils/                             # Formatters, byte helpers, and safeInvoke
├── src-tauri/                             # Backend Application (Rust + Tauri 2)
│   ├── capabilities/                      # Least-privilege capability configs
│   ├── migrations/                        # 8 embedded sequential SQLite migrations
│   │   ├── 0001_initial_schema.sql        # Drives, sessions, app settings
│   │   ├── 0002_files_and_directories.sql # Files, directories, scan errors
│   │   ├── 0003_storage_history...sql     # Snapshots, category summaries
│   │   ├── 0004_storage_classification... # 16 categories, overrides
│   │   ├── 0005_developer_storage.sql     # Projects, ecosystem summaries
│   │   ├── 0006_advanced_analysis.sql     # File hashes, hidden items
│   │   ├── 0007_storage_intelligence.sql  # Recommendations, findings
│   │   └── 0008_cleanup_audit_history.sql # Audit trail events table
│   └── src/
│       ├── commands/                      # Tauri IPC command controllers
│       ├── models/                        # Typed data models and queries
│       ├── services/                      # Scanner, classifier, developer, cleanup
│       ├── storage/                       # Connection pool & migrator engine
│       ├── error.rs                       # AppError enum with sanitized serialization
│       ├── lib.rs                         # Tauri builder setup & command handlers
│       └── main.rs                        # Windows/macOS binary entry point
├── docs/                                  # Documentation & Audit Artifacts
│   ├── WINDOWS_RELEASE_GUIDE.md           # NSIS/MSI packaging & code signing
│   ├── SECURITY_AUDIT.md                  # Zero-telemetry audit & safeguards
│   └── TEST_REPORT.md                     # Comprehensive 11-category QA report
├── package.json                           # Frontend dependencies & npm scripts
├── tailwind.config.js                     # Theme palette definitions
├── tsconfig.json                          # Strict TypeScript workspace configuration
└── vite.config.ts                         # Vite 6 desktop build configuration
```

---

## 🚀 Getting Started

### Prerequisites

1. **Node.js**: v20+ LTS
2. **Rust & Cargo**: v1.80+
3. **Platform Build Tools**:
   - **Windows:** Visual Studio 2022 C++ Build Tools & WebView2 runtime
   - **macOS:** Xcode Command Line Tools (`xcode-select --install`)

### Installation & Development

```bash
# Clone the repository
git clone https://github.com/Harshit17x/Diskatlas.git
cd Diskatlas

# Install dependencies
npm install

# Run frontend type-check & linter
npm run lint

# Run all 24 backend automated unit tests
cargo test --manifest-path src-tauri/Cargo.toml

# Launch desktop app in development mode
npm run tauri dev
```

### Production Build

```bash
# Build production bundle and Windows installers (NSIS & MSI)
npm run build
npm run tauri build
```

---

## 📖 Documentation & Release Guides

- 📘 [Windows Production Release Guide](docs/WINDOWS_RELEASE_GUIDE.md): NSIS packaging, MSI deployment, and Authenticode signing.
- 🛡️ [Security Architecture & Audit Report](docs/SECURITY_AUDIT.md): Zero-telemetry audit, least-privilege review, and OS safeguards.
- 📋 [Comprehensive Quality Assurance Test Report](docs/TEST_REPORT.md): 11-category test matrix covering Windows 10/11, long paths, reparse points, and database recovery.

---

## 📄 License

DiskAtlas is licensed under the **MIT License**. See [LICENSE](LICENSE) for details.
