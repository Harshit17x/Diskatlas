<div align="center">

# 🧭 DiskAtlas

### The modern, private, blazing-fast disk space analyzer & developer storage cleaner.
**Cross-platform for macOS, Windows 10/11, and Linux.**

Built with **Rust**, **Tauri 2**, **React 19**, **TypeScript**, **SQLite (WAL)**, and **D3.js**.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Tauri 2.0](https://img.shields.io/badge/Tauri-2.2-24C8DB.svg?logo=tauri&logoColor=white)](https://tauri.app)
[![Rust 1.80+](https://img.shields.io/badge/Rust-1.80+-DEA584.svg?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![React 19](https://img.shields.io/badge/React-19.0-61DAFB.svg?logo=react&logoColor=black)](https://react.dev)
[![Zero Telemetry](https://img.shields.io/badge/Privacy-100%25%20Local-27AE82.svg)](docs/SECURITY_AUDIT.md)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

[Features](#-key-features) • [Why DiskAtlas?](#-why-diskatlas-vs-alternatives) • [Installation](#-installation) • [Architecture](#-architecture) • [Contributing](#-contributing) • [Roadmap](#-roadmap)

---

</div>

## 💡 The Problem & The Vision

Every developer has run into the dreaded **"Disk Space Almost Full"** warning. 

You run classic disk analyzers, but:
- **WinDirStat** was built in 2005—it's single-threaded, slow, and chokes on modern multi-terabyte SSDs.
- **CleanMyMac** and proprietary tools require expensive \$40/year subscriptions, install invasive background daemons, and phone home with analytics.
- **DaisyDisk** is great, but it's closed-source, Mac-only, and treats all files as generic blobs.
- Generic tools don't understand that your drive is full because of **15 orphaned `node_modules` folders**, **20 GB of stale `.gemini` / IDE caches**, **forgotten Docker build caches**, **abandoned Python venvs**, and **Xcode DerivedData**.

### 🧭 Enter DiskAtlas
**DiskAtlas** is an open-source, local-first storage intelligence application engineered specifically for developers, power users, and system administrators. It combines a hardware-level multi-threaded Rust scanning engine with intelligent ecosystem detection and interactive D3 visualizations to help you understand and safely optimize your storage in seconds.

---

## ⚡ Key Features

### 🚀 1. Ultra-Fast Universal Scanner
- **50,000+ files/second** multi-threaded traversal powered by Rust and memory-mapped SQLite WAL.
- **Hardware & Container Aware**: Intelligently unifies macOS APFS Container partitions to eliminate phantom duplicate volumes (`/` vs `/System/Volumes/Data`), and handles Windows NTFS reparse points and drive letters dynamically.
- **Loop & Cycle Proof**: Bypasses symlink loops, junction points (`IO_REPARSE_TAG_MOUNT_POINT`), and special pseudo filesystems (`/dev`, `/proc`, `/sys`).
- **Dynamic Bottom-Up Directory Aggregation**: Parent directories accurately reflect the true recursive weight of their subdirectories in real-time.
- **Non-blocking Controls**: Seamlessly Pause, Resume, or Cancel scans with zero database corruption.

### 🧑‍💻 2. Developer Storage Intelligence
The major differentiator: DiskAtlas identifies software projects and separates user code from regenerable build artifacts:
- **Node.js / JavaScript**: Detects `node_modules`, `.next/cache`, `.nuxt`, `.output`, `.vite`, `.turbo`, `.parcel-cache`, npm cache, and pnpm stores. Automatically extracts project names from `package.json`.
- **Python**: Identifies `.venv`, `venv`, `__pycache__`, `.pytest_cache`, `.ruff_cache`, pip wheel caches, and Conda environments.
- **Rust**: Discovers `target/` directories, `.cargo/registry`, and `.cargo/git` cache bloat.
- **Docker**: Safely inspects local images, containers, volumes, and build cache through official Docker engine APIs—never touching raw internal files.
- **Java / Mobile**: Identifies `.m2/repository`, `.gradle/caches`, Android SDK platforms, emulator system images, and JDK installs.
- **C / C++**: Flags `cmake-build-debug`, `cmake-build-release`, `.vs/`, and compiler object outputs.
- **IDEs**: Unpacks storage from VS Code, Cursor, JetBrains, Xcode DerivedData, and Antigravity.

### 📊 3. Interactive D3 Visualizations
- **Squarified Treemaps**: Area-proportional rectangles colored by category with interactive drilldown, breadcrumb navigation, and hover diagnostics.
- **Radial Multi-Level Sunburst**: Concentric ring partitions visualising directory depth and hierarchy balance.
- **Real-Time Storage Distribution**: Live breakdown of file extensions, categories, and top storage consumers.

### 🔍 4. Advanced Storage Analysis
- **Staged Cryptographic Duplicate Finder**: 4-stage pipeline (Size match → Metadata validation → Initial block check → Full SHA-256 cryptographic verification). Distinguishes true identical binaries from visually similar media.
- **Large Files Engine**: Configurable size filters, extension whitelists, and instant sorting.
- **Hidden & System Storage Demystifier**: Accurately distinguishes user dotfiles (`.config`, `.git`) from protected OS components.
- **Forgotten Files**: Identifies stale installers (`.dmg`, `.pkg`, `.exe`, `.msi`), old archives (`.zip`, `.tar.gz`), and dormant downloads.

### 🛡️ 5. Safe Cleanup Center with Audit Log
- **Zero Automatic Deletions**: Every deletion requires explicit user selection and confirmation.
- **Critical OS Safeguards**: Automatically blocks deletion of Windows boot components (`bootmgr`, `WinSxS`, drivers) and sealed macOS APFS snapshots.
- **Cloud-Sync Awareness**: Detects and warns when target folders reside inside OneDrive, iCloud, Google Drive, or Dropbox to prevent accidental remote deletion propagation.
- **Reversible Workflow**: Prioritizes the native OS Trash / Windows Recycle Bin by default.
- **Audit Trail**: Maintains an immutable SQLite audit log recording every cleanup event, timestamp, and recovered byte count.

### 🔒 6. 100% Private & Local-First
- **Zero Telemetry**: No tracking, no Google Analytics, no PostHog, no phone-home pings.
- **Fully Offline**: Operates seamlessly without an active internet connection.
- **Local SQLite Engine**: All data stays encrypted on your local hardware in an embedded SQLite database.

---

## 🥊 Why DiskAtlas vs Alternatives?

| Feature | **DiskAtlas** 🧭 | **WinDirStat** 🪟 | **DaisyDisk** 🍏 | **CleanMyMac** 🧹 | **Baobab** 🐧 |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Open Source** | **Yes (MIT)** | Yes (GPL) | No (Proprietary) | No (Proprietary) | Yes (GPL) |
| **Cross-Platform** | **macOS, Windows, Linux** | Windows only | macOS only | macOS only | Linux only |
| **Cost** | **100% Free** | Free | \$9.99 | \$39.95 / year | Free |
| **Developer Intelligence** | **Yes (Node, Rust, Python, Docker)** | No | No | Partial | No |
| **Multi-Threaded Rust Scanner** | **Yes (>50k files/s)** | No (Legacy C++) | Yes (Swift) | No | No (C/GTK) |
| **Modern UI / UX** | **React 19 + Tailwind** | 2005 Win32 UI | Modern macOS | Modern macOS | GTK4 |
| **Interactive Treemap & Sunburst** | **Both (D3.js)** | Treemap only | Sunburst only | No | Both |
| **Safe Reversible Cleanup** | **Recycle Bin / Trash** | Permanent delete | Permanent delete | Custom cleaner | Trash |
| **Telemetry / Tracking** | **Zero (100% Local)** | Zero | Unknown | Telemetry included | Zero |

---

## 🛠️ Architecture

DiskAtlas is architected with a strict separation between low-level systems execution, embedded persistence, and high-density user interface:

```
┌─────────────────────────────────────────────────────────────┐
│                    DiskAtlas Desktop App                    │
│   (React 19 • TypeScript • Tailwind CSS • D3.js Visuals)   │
└──────────────────────────────▲──────────────────────────────┘
                               │ Tauri 2 IPC (Type-Safe RPC)
┌──────────────────────────────▼──────────────────────────────┐
│                    Rust Core Engine                         │
│  ├── Multi-Threaded Scanner (Bounded Stack, Cycle-Proof)    │
│  ├── APFS & NTFS Filesystem Abstraction                     │
│  ├── Deterministic 16-Category Classification Rules         │
│  ├── Developer Ecosystem Analyzers (Cargo, npm, pip, etc.) │
│  └── Reversible Trash / Safeguard Engine                    │
└──────────────────────────────▲──────────────────────────────┘
                               │ rusqlite
┌──────────────────────────────▼──────────────────────────────┐
│                  Embedded SQLite 3 (WAL)                    │
│  ├── Indexed Files & Directories (Recursive Rollups)        │
│  ├── Storage Snapshots & Historical Diffs                   │
│  └── Audit Trail Records                                    │
└─────────────────────────────────────────────────────────────┘
```

---

## 📦 Installation

### Pre-Built Binaries
Download the latest signed release for your platform from the [**Releases Page**](https://github.com/Harshit17x/Diskatlas/releases):

- **macOS (Apple Silicon & Intel)**: `.dmg` or `.app`
- **Windows 10/11 (x64 / ARM64)**: `.msi` or `.exe` installer
- **Linux**: `.AppImage` or `.deb`

### Package Managers *(Coming Soon)*

```bash
# macOS (Homebrew)
brew install --cask diskatlas

# Windows (Winget)
winget install Harshit17x.DiskAtlas

# Cargo (Build from source via crates.io)
cargo install diskatlas
```

---

## 💻 Building From Source

### Prerequisites
1. **Node.js**: v20+ LTS
2. **Rust & Cargo**: v1.80+ ([rustup.rs](https://rustup.rs))
3. **Build Essentials**:
   - **macOS:** Xcode Command Line Tools (`xcode-select --install`)
   - **Windows:** Visual Studio 2022 C++ Build Tools & WebView2 runtime
   - **Linux:** `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libssl-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

### Development Setup

```bash
# 1. Clone the repository
git clone https://github.com/Harshit17x/Diskatlas.git
cd Diskatlas

# 2. Install frontend dependencies
npm install

# 3. Run automated test suite (25 Rust unit/integration tests)
cargo test --manifest-path src-tauri/Cargo.toml

# 4. Start the application in development mode with hot-reload
npm run tauri dev
```

### Packaging Production Release

```bash
# Compile optimized frontend assets and native desktop bundle
npm run build
npm run tauri build
```

---

## 🤝 Contributing

We welcome contributions from developers, designers, and documentation writers of all experience levels! DiskAtlas is built to be a friendly, welcoming open-source project.

### How You Can Help
- 🌟 **Star this repository** to help others discover the project.
- 🐛 **Report bugs** or suggest new features via [GitHub Issues](https://github.com/Harshit17x/Diskatlas/issues).
- 🧑‍💻 **Add new developer ecosystem detectors** (e.g., Elixir `_build`, Go module caches, Flutter/Dart caches, Gradle Kotlin DSL).
- 🌐 **Translate the UI** into additional languages.
- 🎨 **Improve visualizations** and accessibility.

### Getting Started with Code Contributions
1. Fork the repository and clone your fork.
2. Create a new branch: `git checkout -b feature/my-cool-feature`.
3. Make your changes and verify tests pass: `cargo test --manifest-path src-tauri/Cargo.toml`.
4. Commit your changes: `git commit -m "feat(developer): add support for Elixir _build detector"`.
5. Push to your branch and open a Pull Request.

Please review our [**Contributing Guide**](CONTRIBUTING.md) and [**Code of Conduct**](CODE_OF_CONDUCT.md).

---

## 🗺️ Roadmap

- [x] High-throughput universal scanner with cycle and junction protection.
- [x] APFS single-container unified geometry detection.
- [x] Dynamic bottom-up directory size rollups.
- [x] 16-Category deterministic classification engine.
- [x] Developer ecosystem intelligence (Node, Python, Rust, Java, C++, Docker).
- [x] D3 Squarified Treemap and Radial Sunburst visualizations.
- [x] Safe Cleanup Center with Recycle Bin / Trash integration and audit logs.
- [ ] **v1.1**: CLI mode (`diskatlas scan /path` with terminal interactive TUI).
- [ ] **v1.2**: Scheduled automated background alerts when reclaimable cache exceeds 20 GB.
- [ ] **v1.3**: Plugin system for community-contributed developer cache detectors.
- [ ] **v1.4**: Cloud storage remote scanner (S3, Google Drive, Backblaze B2 metadata inspection).

---

## 📄 License

DiskAtlas is licensed under the [**MIT License**](LICENSE). Feel free to use, modify, and distribute it freely.

---

<div align="center">

Made with ❤️ by [Harshit Bisht](https://github.com/Harshit17x) and the open-source community.  
**If you find DiskAtlas useful, please consider giving it a ⭐ on GitHub!**

</div>
