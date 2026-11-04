# DiskAtlas 🧭

> Premium Local-First Storage Intelligence for Windows & macOS.

DiskAtlas is an ultra-fast, local-first disk intelligence desktop application engineered with **Tauri 2**, **Rust**, **React 19**, **TypeScript**, and **SQLite**. It analyzes disk consumption, classifies system and developer storage, discovers duplicates, and provides safe storage optimization recommendations with zero remote uploads and read-only safety guarantees.

---

## 🌟 Philosophy: Discover → Understand → Analyze → Recommend → Optimize

- **Local-First & Private:** Zero remote telemetry or file uploads. All scans and indexings are computed and stored on your device.
- **Non-Destructive & Read-Only:** Scanners operate with read-only metadata inspection handles (`symlink_metadata`).
- **Loop & Symlink Protection:** Windows NTFS junctions (`C:\Users\All Users`) and cyclic symlinks are cycle-checked and skipped.
- **Predictable Performance:** Multi-threaded filesystem traversal powered by Rust with Write-Ahead Log (WAL) SQLite storage.

---

## 🛠️ Technology Stack

- **Desktop Framework:** Tauri 2
- **Backend:** Rust, `sysinfo`, `rusqlite` (bundled SQLite 3), `tokio`, `tracing`
- **Database:** SQLite with embedded SQL migrations (`WAL` mode enabled)
- **Frontend:** React 19, TypeScript, Vite 6, Tailwind CSS, Zustand, Lucide Icons

---

## 📁 Repository Structure

```
DiskAtlas/
├── src/                               # Frontend Application (React 19 + TypeScript)
│   ├── app/                           # Main entry (App.tsx, main.tsx, index.css)
│   ├── components/
│   │   ├── common/                    # Card, Badge, Modal primitives
│   │   └── layout/                    # Header, Sidebar, StatusBar
│   ├── features/
│   │   ├── dashboard/                 # Summary dashboard and hardware overview
│   │   └── drives/                    # Storage volume cards and selector
│   ├── stores/                        # Zustand stores (driveStore, uiStore)
│   ├── types/                         # TypeScript interfaces & Rust IPC bindings
│   └── utils/                         # Formatters (bytes, percent) & safeInvoke
├── src-tauri/                         # Backend Application (Rust + Tauri 2)
│   ├── capabilities/                  # Tauri 2 security capability configurations
│   ├── migrations/                    # Embedded SQL migrations
│   │   └── 0001_initial_schema.sql    # Baseline schema (_migrations, drives, scan_sessions)
│   └── src/
│       ├── commands/                  # Tauri IPC command controllers
│       ├── models/                    # Data structs & serialization models
│       ├── services/                  # Drive detection & system diagnostics
│       ├── storage/                   # SQLite connection pool & migrator engine
│       ├── error.rs                   # Custom AppError enum with JSON serialization
│       ├── lib.rs                     # Tauri builder setup & command registration
│       └── main.rs                    # Desktop binary entry point
├── package.json                       # Node dependencies & npm scripts
├── tailwind.config.js                 # Tailwind CSS theme extension
├── tsconfig.json                      # Strict TypeScript workspace configuration
└── vite.config.ts                     # Vite 6 bundler config tuned for Tauri 2
```

---

## 🚀 Getting Started

### Prerequisites

1. **Node.js**: v20+ (recommended v22+ or v25)
2. **Rust & Cargo**: v1.80+ (tested on Rust 1.99)
3. **OS Build Tools**:
   - **macOS:** Xcode Command Line Tools (`xcode-select --install`)
   - **Windows:** Visual Studio 2022 C++ Build Tools & WebView2 runtime

### Installation

```bash
# Clone the repository
git clone https://github.com/Harshit17x/Diskatlas.git
cd Diskatlas

# Install frontend dependencies
npm install
```

### Development

```bash
# Run type checks
npm run lint

# Build frontend production bundle
npm run build

# Run Rust unit tests
cd src-tauri && cargo test && cd ..

# Launch Desktop Application in development mode
npm run tauri dev
```

---

## 🗄️ Database Architecture (Phase 1 Foundation)

SQLite operates in Write-Ahead Log (`WAL`) mode with automatic foreign key enforcement and 5000ms busy timeouts.

### Foundational Schema (`0001_initial_schema.sql`)
- `_migrations`: Tracks applied schema migrations.
- `drives`: Discovered storage drives and partitions (ID, label, mount point, filesystem, total/available space).
- `scan_sessions`: Audit trail of historical scan operations (drive ID, root path, status, file counts, error counts).
- `app_settings`: Key-value storage for local user preferences and scan exclusions.

---

## 🗺️ Engineering Roadmap

- [x] **Phase 1: Project Foundation & Architecture** (Tauri 2, Rust backend, SQLite WAL migrations, React 19 UI shell)
- [ ] **Phase 2: High-Throughput Filesystem Scanner** (Rayon parallel walker, cycle detection, throttled progress events)
- [ ] **Phase 3: Storage Intelligence & Space Breakdown** (Treemap visualization, extension grouping, AppData/Library mapping)
- [ ] **Phase 4: Duplicates & Large Files Engine** (Two-pass size/hash scanner, TanStack table)
- [ ] **Phase 5: Recommendations & Safe Optimization** (Rule engine, stale caches, safety confirmation workflows)
