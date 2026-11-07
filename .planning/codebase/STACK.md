# Technology Stack

**Analysis Date:** 2026-11-06

## Languages

**Primary:**
- **TypeScript 5.9.3** (`typescript` ^5.7.3 in [`package.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package.json#L34), `5.9.3` resolved in [`package-lock.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package-lock.json)): Powers all frontend desktop views, Zustand reactive state stores, D3 visualization components, and strongly typed Rust-to-TypeScript IPC bindings located under [`src/`](file:///Users/harshitbisht/Desktop/DiskAtlas/src/).
- **Rust 2021 Edition** (rustc 1.99.1, Cargo 1.99.1; configured in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L6)): Drives the desktop core engine, recursive filesystem scanner, Win32/macOS system diagnostics, embedded SQLite persistence, file safety safeguards, and Tauri command handlers in [`src-tauri/src/`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/).

**Secondary:**
- **SQL (SQLite 3.47.2 dialect)**: Embedded relational database schemas and 8 sequential migration scripts located in [`src-tauri/migrations/`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/migrations/).
- **CSS / PostCSS**: Utility styling engine and dynamic design tokens defining Light/Dark theme CSS variables in [`src/app/index.css`](file:///Users/harshitbisht/Desktop/DiskAtlas/src/app/index.css), [`tailwind.config.js`](file:///Users/harshitbisht/Desktop/DiskAtlas/tailwind.config.js), and [`postcss.config.js`](file:///Users/harshitbisht/Desktop/DiskAtlas/postcss.config.js).
- **HTML5**: Single-page desktop window host container located at [`index.html`](file:///Users/harshitbisht/Desktop/DiskAtlas/index.html).

## Runtime

**Environment:**
- **Tauri Core Runtime v2** (`2.12.5`): Native cross-platform desktop wrapper utilizing OS webviews (macOS WKWebView, Windows WebView2, Linux WebKitGTK) without Chromium overhead.
- **Node.js**: Active environment `v25.2.1` (requires Node.js >= 18.0.0) used for executing Vite dev server and TypeScript compilation.

**Package Manager:**
- **npm 11.9.0**: Node package manager; lockfile [`package-lock.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package-lock.json) is present and committed.
- **Cargo 1.99.1**: Rust package manager; lockfile [`src-tauri/Cargo.lock`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.lock) is present and committed.

## Frameworks

**Core:**
- **Tauri v2** (`tauri` crate `2.12.5` in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L19); `@tauri-apps/api` `2.12.4` in [`package.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package.json#L15)): Inter-Process Communication (IPC) bridge, capability permissions, and native OS window hosting.
- **React 19** (`react` `19.3.1`, `react-dom` `19.3.1` in [`package.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package.json#L20-L21)): Frontend component architecture and UI rendering in [`src/app/App.tsx`](file:///Users/harshitbisht/Desktop/DiskAtlas/src/app/App.tsx).
- **Zustand 5** (`zustand` `5.0.16` in [`package.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package.json#L23)): Lightweight, reactive state management across 10 modular domain stores in [`src/stores/`](file:///Users/harshitbisht/Desktop/DiskAtlas/src/stores/).
- **Tailwind CSS 3** (`tailwindcss` `3.4.19` in [`package.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package.json#L33)): Utility-first styling with custom palette extensions and dark-mode support in [`tailwind.config.js`](file:///Users/harshitbisht/Desktop/DiskAtlas/tailwind.config.js).

**Testing:**
- **Rust Built-in Test Harness (`#[cfg(test)]`)**: 25 inline unit tests verifying database migrations, filesystem safeguards, file classification heuristics, and duplicate hashing in [`src-tauri/src/`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/). Executed via `cargo test`.
- **Frontend Testing**: Not detected / not configured (no Jest, Vitest, or Cypress test configurations present). Static validation is performed via `npm run lint` (`tsc -b --noEmit`).

**Build/Dev:**
- **Vite 6** (`vite` `6.4.5` in [`package.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package.json#L35)): Development server and production bundling configured in [`vite.config.ts`](file:///Users/harshitbisht/Desktop/DiskAtlas/vite.config.ts).
- **@vitejs/plugin-react** (`4.7.0`): React Fast Refresh and JSX transformation support.
- **@tauri-apps/cli** (`2.12.4`): Tauri command-line tool for desktop build orchestration.
- **tauri-build** (`2.7.4`): Build-time Cargo compilation script in [`src-tauri/build.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/build.rs).

## Key Dependencies

**Critical:**
- **`rusqlite` (0.33.0, feature: `bundled`)** in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L23): Compiles embedded SQLite 3.47.2 directly into the native binary, eliminating external database dependencies. Configured with WAL mode, memory caching, and busy timeouts in [`src-tauri/src/storage/connection.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/storage/connection.rs).
- **`tokio` (1.54.1, features: `sync`, `macros`, `rt-multi-thread`)** in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L25): Asynchronous runtime powering Tauri command invocations, thread notifications (`tokio::sync::Notify`), and non-blocking background SQLite tasks.
- **`sysinfo` (0.33.1)** in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L24): Inspects hardware host specifications, disk volumes, mount points, total/available space, and CPU memory in [`src-tauri/src/services/system_inspector.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/system_inspector.rs) and [`src-tauri/src/services/drive_detector.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/drive_detector.rs).
- **`trash` (5.2.9)** in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L31): Provides native, reversible deletion via the operating system Recycle Bin / Trash (macOS Finder Trash, Windows Shell IFileOperation / Recycle Bin, and Linux FreeDesktop Trash) in [`src-tauri/src/services/cleanup/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs#L292).
- **`d3` (7.9.2) & `@types/d3` (7.4.3)** in [`package.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package.json#L16-L18): Computes hierarchical partitions, treemap layouts, and sunburst arc projections in [`src/components/visualizations/TreemapChart.tsx`](file:///Users/harshitbisht/Desktop/DiskAtlas/src/components/visualizations/TreemapChart.tsx) and [`src/components/visualizations/SunburstChart.tsx`](file:///Users/harshitbisht/Desktop/DiskAtlas/src/components/visualizations/SunburstChart.tsx).
- **`@tanstack/react-table` (8.21.3)** in [`package.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package.json#L14): Drives high-performance paginated tabular views with column sorting and filtering in [`src/components/common/DataTable.tsx`](file:///Users/harshitbisht/Desktop/DiskAtlas/src/components/common/DataTable.tsx).

**Infrastructure:**
- **`serde` (1.0.229) & `serde_json` (1.0.154)** in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L20-L21): Serialization and deserialization of typed IPC payloads between Rust and TypeScript.
- **`uuid` (1.29.0, feature: `v4`)** in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L26): Generates unique identifiers for scan sessions, snapshot records, cleanup batches, and recommendations.
- **`sha2` (0.10.9) & `hex` (0.4.3)** in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L29-L30): Streaming cryptographic SHA-256 hash generation for duplicate file detection in [`src-tauri/src/services/analysis/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/analysis/mod.rs#L302).
- **`thiserror` (2.0.21)** in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L22): Structured domain error classification in [`src-tauri/src/error.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/error.rs).
- **`tracing` (0.1.44) & `tracing-subscriber` (0.3.23)** in [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml#L27-L28): Structured log event collection and filtering initialized in [`src-tauri/src/lib.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/lib.rs#L39-L42).
- **`lucide-react` (0.475.0)** in [`package.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package.json#L19): Icon set used across desktop navigation, drive cards, status indicators, and modal banners.
- **`clsx` (2.1.1) & `tailwind-merge` (3.7.0)** in [`package.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package.json#L17-L22): Utility functions for conditional CSS class joining and deduplication.

## Configuration

**Environment:**
- **Zero Runtime Env Vars Required**: DiskAtlas operates completely local-first and offline without requiring secrets, API keys, or cloud endpoints.
- **`TAURI_DEV_HOST`**: Optional development environment variable read in [`vite.config.ts`](file:///Users/harshitbisht/Desktop/DiskAtlas/vite.config.ts#L4) to configure HMR bindings for remote network testing.
- **`RUST_LOG`**: Optional log level filter (defaults to `"info,diskatlas=debug"` in [`src-tauri/src/lib.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/lib.rs#L40)).

**Build:**
- [`package.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/package.json): Defines npm build scripts (`dev`, `build`, `lint`, `preview`, `tauri`).
- [`vite.config.ts`](file:///Users/harshitbisht/Desktop/DiskAtlas/vite.config.ts): Configures port `1420`, strict port adherence, and watch exclusions for `**/src-tauri/**`.
- [`tsconfig.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/tsconfig.json), [`tsconfig.app.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/tsconfig.app.json), [`tsconfig.node.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/tsconfig.node.json): Strict TypeScript project compilation settings.
- [`tailwind.config.js`](file:///Users/harshitbisht/Desktop/DiskAtlas/tailwind.config.js) & [`postcss.config.js`](file:///Users/harshitbisht/Desktop/DiskAtlas/postcss.config.js): Content paths and CSS variable mappings.
- [`src-tauri/Cargo.toml`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/Cargo.toml): Release profile optimizations (`opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`).
- [`src-tauri/tauri.conf.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/tauri.conf.json): Bundle targets (`nsis`, `msi`), window dimensions (1280x840, min 960x640), capabilities, and asset paths.
- [`src-tauri/capabilities/default.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/capabilities/default.json): Declares Tauri v2 capability configuration granting `core:default` permissions to the `"main"` window.

## Platform Requirements

**Development:**
- **Operating Systems**: macOS (11+), Windows 10/11, or Linux (Ubuntu 20.04+, Fedora 38+).
- **Runtimes & Compilers**:
  - Node.js >= 18.0.0 (active: `v25.2.1`), npm >= 9.0.0 (active: `11.9.0`).
  - Rust 1.77+ (active: `1.99.1`) with `cargo` installed.
  - Native C/C++ toolchain (Xcode Command Line Tools on macOS; Visual Studio 2022 C++ Build Tools on Windows; `build-essential`, `libwebkit2gtk-4.1-dev` on Linux).

**Production:**
- **macOS**: Universal binary (Apple Silicon `aarch64` & Intel `x86_64`) packaged as `.app` or `.dmg`. Requires macOS 11.0 Big Sur or later.
- **Windows**: 64-bit executable packaged as NSIS installer (`.exe`) or Windows Installer (`.msi`). Requires Windows 10 (version 1809+) or Windows 11 with Microsoft Edge WebView2 runtime.
- **Linux**: Distributed as AppImage or `.deb` / `.rpm` utilizing system WebKitGTK 4.1.

---

*Stack analysis: 2026-11-06*
