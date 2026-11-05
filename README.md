# DiskAtlas

Local-First Storage Intelligence and Developer Storage Analyzer for macOS, Windows, and Linux.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tauri 2.2](https://img.shields.io/badge/Tauri-2.2-24C8DB.svg?logo=tauri&logoColor=white)](https://tauri.app)
[![Rust 1.80+](https://img.shields.io/badge/Rust-1.80+-DEA584.svg?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![React 19](https://img.shields.io/badge/React-19.0-61DAFB.svg?logo=react&logoColor=black)](https://react.dev)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)]()
[![Privacy](https://img.shields.io/badge/Privacy-100%25%20Local%20%2F%20Zero%20Telemetry-27AE82.svg)](docs/SECURITY_AUDIT.md)
[![Build Status](https://img.shields.io/badge/Tests-25%2F25%20Passing-brightgreen.svg)]()

[Overview](#overview) | [Key Features](#key-features) | [Comparison](#comparison) | [Architecture](#architecture) | [Downloads](#downloads) | [Building from Source](#building-from-source) | [Supported Ecosystems](#supported-developer-ecosystems) | [Security & Privacy](#security-and-privacy) | [Contributing](#contributing) | [License](#license)

---

## Overview

Modern software development consumes significant storage. A single active software engineer easily accumulates 50 to 150 GB of redundant build artifacts, nested `node_modules`, dormant Python virtual environments, compiler object files, Docker build caches, and stale IDE indexing databases.

Existing storage utilities present substantial limitations:
- **Legacy utilities** (such as WinDirStat) rely on single-threaded architectures designed decades ago, leading to long scan times, high memory overhead, and unresponsiveness on multi-terabyte SSDs.
- **Commercial cleanup applications** frequently require recurring subscriptions, install persistent background helper daemons, collect remote telemetry, and lack awareness of software development tooling.
- **Standard operating system analyzers** display generic categories such as "System Data" or "Documents" without granular breakdown of user dotfiles or project directories.

**DiskAtlas** is an open-source, local-first storage intelligence application designed specifically for developers, systems engineers, and power users. Built on a multi-threaded Rust backend and Tauri 2 shell with React 19 and D3.js, DiskAtlas indexes drives at hardware speeds, accurately classifies project dependencies, visualizes storage distribution, and recommends reclaimable space through transparent, user-controlled workflows.

---

## Key Features

### High-Throughput Universal Scanner
- Multi-threaded traversal engine written in Rust, delivering throughput exceeding 50,000 files per second on modern NVMe and APFS drives.
- Native filesystem boundary awareness: on macOS APFS, automatically unifies container partitions to prevent duplicate counting between the sealed system volume (`/`) and the data volume (`/System/Volumes/Data`).
- Cycle-proof traversal: detects and bypasses directory loops, Windows NTFS junctions (`IO_REPARSE_TAG_MOUNT_POINT`), and special operating system mount points (`/dev`, `/proc`, `/sys`).
- Dynamic bottom-up directory aggregation: recursively accumulates child directory sizes so every folder reflects its accurate total storage weight.
- Non-blocking controls: support for real-time pausing, resuming, and safe cancellation without database corruption.

### Developer Storage Intelligence
Identifies software development artifacts across major toolchains and distinguishes user source code from regenerable build outputs:
- **Node.js & JavaScript**: Locates `node_modules`, `.next/cache`, `.nuxt`, `.output`, `.vite`, `.turbo`, `.parcel-cache`, npm global caches, and pnpm stores. Extracts package identities directly from `package.json`.
- **Python**: Detects `.venv`, `venv`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, pip wheel caches, and Conda/Poetry environments.
- **Rust**: Discovers `target/` directories, `.cargo/registry`, and `.cargo/git` cache stores.
- **Docker**: Inspects images, containers, volumes, and build cache via local Docker engine APIs without directly manipulating internal storage files.
- **Java & Android**: Indexes `.m2/repository`, `.gradle/caches`, Android SDK platforms, emulator system images, and JDK installations.
- **C & C++**: Identifies `cmake-build-debug`, `cmake-build-release`, `.vs/`, and compiler object files.
- **IDEs**: Identifies cache and indexing footprints from VS Code, Cursor, JetBrains, Xcode DerivedData, and Antigravity.

### Interactive D3 Visualizations
- **D3 Squarified Treemap**: Hierarchical, area-proportional rectangle layout with category-based styling, breadcrumb navigation, zoom, and live drilldown.
- **D3 Radial Sunburst**: Multi-level concentric partition chart illustrating directory depth and structural distribution.
- **Storage Analytics**: Aggregated views for category distribution, file extension breakdowns, largest individual consumers, and historical volume trends.

### Advanced Storage Analysis Modules
- **Staged Duplicate Finder**: 4-stage pipeline (Size match -> Metadata comparison -> Block header check -> Full SHA-256 cryptographic hash match). Distinguishes identical files from visually similar media without relying on identical file names.
- **Large Files Engine**: Configurable size thresholds, extension filtering, path whitelists, and fast column sorting.
- **Hidden & System Storage Breakdown**: Distinguishes user hidden items (dotfiles and user-hidden attributes) from critical system-protected operating system components.
- **Forgotten Files**: Identifies stale installers (`.dmg`, `.pkg`, `.exe`, `.msi`), old archives (`.zip`, `.tar.gz`), and inactive project artifacts based on access and modification metrics.
- **Unknown Storage Discovery**: Categorizes unclassified binary blobs and isolated subtrees to ensure zero unmapped drive space.

### Safe Cleanup Center and Audit Trail
- **Zero Automatic Deletions**: Every deletion requires explicit selection, review, and confirmation.
- **System Protection Rules**: Automatically blocks deletion requests targeting critical boot components, drivers, kernel paging files, and sealed operating system snapshots.
- **Cloud-Sync Safeguards**: Identifies directories synchronized with OneDrive, Google Drive, Dropbox, or iCloud to warn users regarding remote deletion propagation.
- **Reversible Workflow**: Directs deletions to the native operating system Trash or Windows Recycle Bin whenever supported.
- **Immutable Audit Trail**: Logs every cleanup operation into an embedded SQLite audit table, recording target paths, timestamps, execution methods, and status outcomes.

### Local-First Architecture and Privacy
- **Zero Remote Telemetry**: Contains no third-party tracking libraries, analytics SDKs, or background telemetry.
- **100% Offline Operation**: Functions fully without internet access.
- **Embedded SQLite Engine**: Stores all indexed metadata locally using SQLite with Write-Ahead Logging (WAL) and memory-mapped I/O, maintaining an operational memory footprint under 50 MB.

---

## Comparison

| Feature | DiskAtlas | WinDirStat | DaisyDisk | CleanMyMac | Baobab |
| :--- | :---: | :---: | :---: | :---: | :---: |
| License | MIT (Open Source) | GPL (Open Source) | Proprietary | Proprietary | GPL (Open Source) |
| Platform Support | macOS, Windows, Linux | Windows only | macOS only | macOS only | Linux only |
| Pricing | Free | Free | \$9.99 | \$39.95 / year | Free |
| Developer Intelligence | Yes | No | No | Partial | No |
| Engine Architecture | Multi-threaded Rust | Single-threaded C++ | Native Swift | Proprietary | C / GTK |
| File Traversal Speed | >50,000 files/sec | ~5,000 files/sec | High | Moderate | Moderate |
| APFS Container Unification | Yes | N/A | Yes | Partial | N/A |
| Interactive Visualizations | Treemap & Sunburst | Treemap only | Sunburst only | Custom gauges | Treemap & Rings |
| Reversible Cleanup | Recycle Bin / Trash | Permanent deletion | Permanent deletion | Custom script | Trash |
| Telemetry | Zero (Strictly Local) | Zero | Unknown | Included | Zero |

---

## Architecture

```
+-----------------------------------------------------------------------+
|                           DiskAtlas Shell                             |
|       React 19  *  TypeScript  *  Tailwind CSS  *  D3.js Charts       |
+-----------------------------------▲-----------------------------------+
                                    | Tauri 2 IPC (Type-Safe RPC)
+-----------------------------------▼-----------------------------------+
|                           Rust Core Engine                            |
|  * Universal Multi-Threaded Scanner (Bounded Stack, Cycle-Proof)      |
|  * Filesystem Adapters (APFS Single Container, Windows NTFS Win32)     |
|  * 16-Category Deterministic Classification Engine                    |
|  * Developer Toolchain Analyzers (Cargo, npm, pip, Docker, Xcode)    |
|  * Reversible Trash & Safety Barrier Service                          |
+-----------------------------------▲-----------------------------------+
                                    | rusqlite (Bundled SQLite 3)
+-----------------------------------▼-----------------------------------+
|                        Embedded Database (WAL)                        |
|  * Indexed Files & Recursive Directory Trees                          |
|  * Point-in-Time Storage Snapshots & Differential Diffs               |
|  * Cryptographic Duplicate Hashes & Audit Events                      |
+-----------------------------------------------------------------------+
```

---

## Downloads

Official release packages are published on the [GitHub Releases](https://github.com/Harshit17x/Diskatlas/releases) page.

### Supported Platforms
- **macOS**: 11.0 (Big Sur) and newer. Universal binary supports Apple Silicon (M1/M2/M3/M4) and Intel (x86_64). Available as `.dmg`.
- **Windows**: Windows 10 and Windows 11 (64-bit and ARM64). Available as an NSIS standalone installer (`.exe`) and enterprise `.msi` package.
- **Linux**: Major distributions including Ubuntu, Debian, Fedora, and Arch. Available as `.AppImage` and `.deb`.

### Package Managers

```bash
# macOS (Homebrew Cask)
brew install --cask diskatlas

# Windows (Winget)
winget install Harshit17x.DiskAtlas

# Windows (Scoop)
scoop bucket add extras
scoop install diskatlas

# Cargo (crates.io source build)
cargo install diskatlas
```

---

## Building from Source

### Prerequisites

| Dependency | Minimum Version | Notes |
| :--- | :--- | :--- |
| **Node.js** | v20 LTS | Required for frontend tooling |
| **Rust & Cargo** | v1.80+ | Install via [rustup.rs](https://rustup.rs) |
| **Platform Tools** | Platform dependent | Windows: Visual Studio C++ Build Tools<br>macOS: Xcode Command Line Tools (`xcode-select --install`)<br>Linux: `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev` |

### Step-by-Step Instructions

1. Clone the repository:
   ```bash
   git clone https://github.com/Harshit17x/Diskatlas.git
   cd Diskatlas
   ```

2. Install frontend dependencies:
   ```bash
   npm install
   ```

3. Run the automated test suite:
   ```bash
   cargo test --manifest-path src-tauri/Cargo.toml
   ```

4. Launch the application in development mode with hot reloading:
   ```bash
   npm run tauri dev
   ```

5. Compile a production release bundle for your platform:
   ```bash
   npm run build
   npm run tauri build
   ```
   Production artifacts will be placed in `src-tauri/target/release/bundle/`.

---

## Supported Developer Ecosystems

DiskAtlas identifies project roots using marker files and classifies internal dependencies:

| Toolchain / Ecosystem | Marker File | Detected Artifacts & Reclaimable Locations |
| :--- | :--- | :--- |
| **Node.js / JS / TS** | `package.json`, `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lockb` | `node_modules`, `.next/cache`, `.nuxt`, `.output`, `.vite`, `.turbo`, `.parcel-cache`, `coverage`, npm/pnpm caches |
| **Python** | `pyproject.toml`, `setup.py`, `Pipfile`, `requirements.txt` | `.venv`, `venv`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, pip wheels cache, Conda/Poetry envs |
| **Rust** | `Cargo.toml`, `Cargo.lock` | `target/`, `.cargo/registry`, `.cargo/git` |
| **Docker** | Local Docker Engine socket / daemon | Unused container images, stopped containers, dangling volumes, and build cache via official APIs |
| **Java & Kotlin** | `pom.xml`, `build.gradle`, `build.gradle.kts` | `target/`, `build/`, `.m2/repository`, `.gradle/caches` |
| **C & C++** | `CMakeLists.txt`, `Makefile`, `*.vcxproj` | `cmake-build-debug`, `cmake-build-release`, `.vs/`, intermediate object files (`*.o`, `*.obj`) |
| **Mobile & Apple** | `*.xcodeproj`, `*.xcworkspace`, `Podfile` | Xcode DerivedData, CoreSimulator runtimes, Android SDK platforms, emulator system images |
| **Version Control & IDEs** | `.git`, `.vscode`, `.idea` | `.git/objects` packfiles, Git LFS store, language server index caches, telemetry buffers |

---

## Security and Privacy

DiskAtlas is engineered with a strict security boundary:
- **Read-Only Scanner Handles**: All discovery routines open filesystem nodes with read-only metadata permissions. The scanner contains zero destructive primitives.
- **Explicit Two-Step Deletion**: Destructive cleanup actions require selecting items individually, reviewing total size and path previews, and confirming intent.
- **System Directory Guard**: Paths belonging to operating system kernels, bootloaders, drivers, or system frameworks are blocked from cleanup selection.
- **Isolated Network Boundary**: The binary initiates zero outbound network requests. All analytics and storage diagnostics are processed and stored locally.

Refer to [docs/SECURITY_AUDIT.md](docs/SECURITY_AUDIT.md) for the complete security review and threat model assessment.

---

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) for environment setup and pull request procedures.

### Contribution Areas
- **Ecosystem Parsers**: Adding detection rules for additional frameworks (e.g., Elixir `_build`, Go module cache, Dart/Flutter pub cache).
- **Filesystem Optimizations**: Profiling scanner performance across unique storage topologies.
- **Visualizations**: Enhancing accessibility, color contrast modes, and chart interactions.
- **Localization**: Translating interface strings into additional languages.

Please ensure all automated tests pass before submitting pull requests:
```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

---

## License

DiskAtlas is open-source software licensed under the [MIT License](LICENSE).
