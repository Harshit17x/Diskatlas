<Codebase Structure>
**Analysis Date:** 2026-11-06

## Directory Layout

```
DiskAtlas/
├── .github/                              # GitHub actions, issue and pull request templates
│   ├── ISSUE_TEMPLATE/                   # Issue templates for bugs and feature requests
│   │   ├── bug_report.md                 # Structured bug report template
│   │   └── feature_request.md            # Structured feature proposal template
│   └── pull_request_template.md          # Standard checklist and description template for PRs
├── .planning/                            # Project specifications and architecture mapping
│   └── codebase/                         # Synthesized codebase analysis documents
│       ├── ARCHITECTURE.md               # High-level architecture, threading, and data flows
│       └── STRUCTURE.md                  # Physical directory layout, file maps, and code placement
├── docs/                                 # Engineering audit reports and packaging documentation
│   ├── SECURITY_AUDIT.md                 # Security threat model, filesystem validation, and audit
│   ├── TEST_REPORT.md                    # Backend and UI test execution records and benchmarks
│   └── WINDOWS_RELEASE_GUIDE.md          # Windows NSIS/MSI packaging and code-signing reference
├── public/                               # Static assets served directly by Vite
│   └── vite.svg                          # Default Vite favicon
├── src/                                  # React 19 / TypeScript desktop frontend application
│   ├── app/                              # Top-level application shell and global entry points
│   │   ├── App.tsx                       # Main application shell with sidebar, layout, and tabs
│   │   ├── index.css                     # Tailwind CSS imports and custom design tokens
│   │   └── main.tsx                      # React DOM root bootstrapping and StrictMode mount
│   ├── components/                       # Reusable UI component building blocks
│   │   ├── cleanup/                      # Cleanup center UI dialogs and status banners
│   │   │   ├── CleanupConfirmationModal.tsx # Destructive action confirmation and review modal
│   │   │   └── CleanupResultsBanner.tsx     # Post-cleanup execution audit summary banner
│   │   ├── common/                       # Generic design system components
│   │   │   ├── Badge.tsx                 # Risk, category, and status pill badges
│   │   │   ├── Button.tsx                # Standard, ghost, and destructive button primitives
│   │   │   ├── Card.tsx                  # Elevated surface container cards
│   │   │   ├── DataTable.tsx             # TanStack Table wrapper with sorting and pagination
│   │   │   ├── Modal.tsx                 # Accessible dialog overlay backdrop and container
│   │   │   └── Tabs.tsx                  # Tab list and panel switching component
│   │   ├── layout/                       # Structural layout navigation components
│   │   │   ├── CommandPalette.tsx        # Cmd+K floating quick-action launcher modal
│   │   │   ├── Header.tsx                # Window title bar, active drive switcher, theme toggle
│   │   │   ├── Sidebar.tsx               # Primary left-side tab navigation bar
│   │   │   └── StatusBar.tsx             # Bottom status bar with DB health, memory, and version
│   │   └── visualizations/               # Interactive D3 storage analytics charts
│   │       ├── StorageAnalyticsDashboard.tsx # Aggregated analytics view with summary cards
│   │       ├── SunburstChart.tsx         # D3 radial multilevel zoomable partition chart
│   │       └── TreemapChart.tsx          # D3 squarified hierarchical space-filling treemap
│   ├── features/                         # Feature domain view components (tab screens)
│   │   ├── analysis/                     # Storage categorization and breakdown
│   │   │   └── StorageAnalysisView.tsx   # Category percentages, subcategories, top apps view
│   │   ├── applications/                 # Application usage profiling
│   │   │   └── ApplicationsView.tsx      # Installed applications storage footprint inspector
│   │   ├── cleanup/                      # Storage reclaim and cleanup center
│   │   │   └── CleanupCenterView.tsx     # Interactive recommendations review and deletion wizard
│   │   ├── dashboard/                    # Primary overview landing screen
│   │   │   └── DashboardView.tsx         # Drive status cards, quick scan launcher, storage health
│   │   ├── developer/                    # Developer tooling storage inspector
│   │   │   └── DeveloperStorageView.tsx  # Project cache, node_modules, build artifact manager
│   │   ├── drives/                       # Storage drive management components
│   │   │   ├── DriveCard.tsx             # Storage capacity, health, and mount point card
│   │   │   ├── DriveDetailsModal.tsx     # Deep filesystem and hardware inspector modal
│   │   │   └── DriveList.tsx             # Grid of discovered storage volumes
│   │   ├── duplicates/                   # Duplicate file detection and elimination
│   │   │   └── DuplicateFinderView.tsx   # Grouped hash duplicates with side-by-side selection
│   │   ├── explorer/                     # File and directory navigation
│   │   │   └── DriveExplorerView.tsx     # Paginated file explorer with directory drill-down
│   │   ├── hidden/                       # Hidden files audit
│   │   │   └── HiddenFilesView.tsx       # Uncovers dotfiles, hidden directories, system caches
│   │   ├── history/                      # Storage snapshots and time-travel comparison
│   │   │   └── StorageHistoryView.tsx    # Scan session history, differential comparison table
│   │   ├── intelligence/                 # Storage intelligence and recommendations
│   │   │   └── StorageIntelligenceView.tsx # Actionable findings sorted by risk and confidence
│   │   ├── scanner/                      # Active scan modal and controls
│   │   │   └── ScanModal.tsx             # Path selector, exclusions configuration, live scanner
│   │   └── settings/                     # Application configuration
│   │       └── SettingsView.tsx          # Database maintenance, scan exclusions, debug tools
│   ├── stores/                           # Zustand client-side reactive state stores
│   │   ├── advancedAnalysisStore.ts      # Large, hidden, duplicate, and forgotten files state
│   │   ├── analysisStore.ts              # Classification profile, category stats, override rules
│   │   ├── cleanupStore.ts               # Cleanup candidates selection, execution, and audit logs
│   │   ├── developerStore.ts             # Developer projects, Docker and tool cache metrics
│   │   ├── driveStore.ts                 # Drive discovery, system metrics, SQLite database health
│   │   ├── historyStore.ts               # Scan history sessions, differential comparisons
│   │   ├── intelligenceStore.ts          # Storage recommendations, findings, and risk metrics
│   │   ├── scanStore.ts                  # Active scan session, live progress IPC, event listeners
│   │   ├── uiStore.ts                    # Active tab routing, theme toggle, command palette
│   │   └── visualizationStore.ts         # Hierarchical tree nodes, breadcrumb drill-down state
│   ├── types/                            # TypeScript interfaces and backend type contracts
│   │   ├── bindings.ts                   # Mirrors Rust structs from `src-tauri/src/models/`
│   │   └── ui.ts                         # UI navigation, tab types, and visualization types
│   ├── utils/                            # Shared frontend helper utilities
│   │   ├── formatters.ts                 # Byte formatting (KB, MB, GB, TB), dates, percentages
│   │   └── tauri.ts                      # Type-safe `safeInvoke` wrapper for Tauri IPC calls
│   └── vite-env.d.ts                     # Vite client type references
├── src-tauri/                            # Tauri v2 native desktop core written in Rust
│   ├── capabilities/                     # Tauri v2 security ACL and capabilities manifests
│   │   └── default.json                  # Grants core permissions to the main desktop window
│   ├── gen/                              # Tauri auto-generated build schemas and ACL manifests
│   │   └── schemas/                      # JSON schemas for desktop window capabilities
│   ├── icons/                            # Desktop application icons for all target OS platforms
│   │   ├── 128x128.png                   # Linux / web application icon
│   │   ├── 128x128@2x.png                # High-DPI application icon
│   │   ├── 32x32.png                     # Window icon
│   │   ├── icon.icns                     # macOS bundled application icon
│   │   └── icon.ico                      # Windows executable and taskbar icon
│   ├── migrations/                       # Embedded SQLite schema migration scripts
│   │   ├── 0001_initial_schema.sql       # Drives, scan_sessions, app_settings tables
│   │   ├── 0002_files_and_directories.sql # Files, directories, scan_errors tables
│   │   ├── 0003_storage_history_and_snapshots.sql # Storage snapshots & category summaries
│   │   ├── 0004_storage_classification.sql # Category overrides & classification summaries
│   │   ├── 0005_developer_storage.sql    # Developer projects & ecosystem summaries
│   │   ├── 0006_advanced_analysis.sql    # Cryptographic file hashes & lookup indexes
│   │   ├── 0007_storage_intelligence.sql # Recommendations & actionable finding tables
│   │   └── 0008_cleanup_audit_history.sql # Transactional cleanup audit events log
│   ├── src/                              # Native Rust source code
│   │   ├── commands/                     # Tauri IPC command handler entry points
│   │   │   ├── analysis.rs               # Commands: large files, hidden, duplicates, forgotten
│   │   │   ├── classification.rs         # Commands: classification profile, overrides
│   │   │   ├── cleanup.rs                # Commands: cleanup plans, execution, audit history
│   │   │   ├── database.rs               # Commands: database health and diagnostics
│   │   │   ├── developer.rs              # Commands: developer projects, docker, tool caches
│   │   │   ├── drives.rs                 # Commands: drive listing, details, and refresh
│   │   │   ├── history.rs                # Commands: session listing, delete, snapshot compare
│   │   │   ├── intelligence.rs           # Commands: storage intelligence and recommendations
│   │   │   ├── mod.rs                    # Module declarations for all command handlers
│   │   │   ├── scanner.rs                # Commands: start, pause, resume, cancel, get progress
│   │   │   ├── system.rs                 # Commands: system hardware and OS inspection
│   │   │   └── visualization.rs          # Commands: hierarchical node trees, storage analytics
│   │   ├── models/                       # Serde-serializable Rust data structures
│   │   │   ├── analysis.rs               # Request/response structs for duplicates, large files
│   │   │   ├── classification.rs         # StorageCategory enum, category statistics, overrides
│   │   │   ├── cleanup.rs                # Cleanup plan candidates, execution requests, audit
│   │   │   ├── database.rs               # Database health and status structs
│   │   │   ├── developer.rs              # Developer project models, ecosystems, and caches
│   │   │   ├── drive.rs                  # DriveInfo, DriveDetails, DriveType enum
│   │   │   ├── history.rs                # Scan sessions, snapshots, differential comparison models
│   │   │   ├── intelligence.rs           # Storage recommendation models, risk levels, findings
│   │   │   ├── mod.rs                    # Module exports for all models
│   │   │   ├── scan.rs                   # ScanOptions, ScanProgress, ScanSummary, ScanStatus enum
│   │   │   ├── system.rs                 # SystemInfo model
│   │   │   └── visualization.rs          # HierarchicalNode, tree queries, analytics data
│   │   ├── services/                     # Business logic and domain implementations
│   │   │   ├── analysis/                 # Advanced analysis algorithms (staged duplicate hasher)
│   │   │   │   └── mod.rs                # Large file filters, staged duplicate SHA-256 hasher
│   │   │   ├── classifier/               # Storage classification service
│   │   │   │   ├── mod.rs                # Session classification and database persistence
│   │   │   │   └── rules.rs              # Deterministic 16-category rule matching engine
│   │   │   ├── cleanup/                  # Cleanup execution and operating system guardrails
│   │   │   │   ├── mod.rs                # Cleanup plan generator, trash integration, audit logger
│   │   │   │   └── safeguards.rs         # Critical OS path protection, cloud sync detection
│   │   │   ├── developer/                # Developer ecosystem scanner
│   │   │   │   └── mod.rs                # node_modules, target, .venv, Docker discovery
│   │   │   ├── history/                  # Snapshots and history comparison service
│   │   │   │   └── mod.rs                # Snapshot creation, delta calculation between scans
│   │   │   ├── intelligence/             # Storage intelligence recommendation synthesis
│   │   │   │   └── mod.rs                # Heuristic rules evaluating disk waste patterns
│   │   │   ├── scanner/                  # Core filesystem scanner engine
│   │   │   │   ├── engine.rs             # Dedicated OS thread recursive scanner, batch flusher
│   │   │   │   ├── manager.rs            # Active scan session registry, atomic status manager
│   │   │   │   ├── models.rs             # Raw file/directory/error traversal item buffers
│   │   │   │   └── mod.rs                # Module entry point and integration test suites
│   │   │   ├── drive_detector.rs         # sysinfo mount point detector and filesystem inspector
│   │   │   ├── mod.rs                    # Module declarations for services
│   │   │   ├── system_inspector.rs       # OS version, hardware CPU, and memory inspector
│   │   │   └── visualization/            # Bounded hierarchical aggregation service
│   │   │       └── mod.rs                # Builds tree structures for D3 Treemap & Sunburst
│   │   ├── storage/                      # Database persistence and migrations
│   │   │   ├── connection.rs             # SQLite DbPool with WAL mode, pragmas, and mutex
│   │   │   ├── migrator.rs               # Sequential embedded migrations runner
│   │   │   └── mod.rs                    # Module re-exports for DbPool and Migrator
│   │   ├── error.rs                      # Unified AppError enum with custom Serde serialization
│   │   ├── lib.rs                        # Main Tauri application configuration and setup
│   │   └── main.rs                       # Native executable entry point
│   ├── build.rs                          # Tauri build script
│   ├── Cargo.lock                        # Locked Rust dependencies
│   ├── Cargo.toml                        # Rust package manifest, dependencies, release profile
│   └── tauri.conf.json                   # Tauri v2 application configuration and window metadata
├── dist/                                 # Compiled frontend web assets (generated by `npm run build`)
├── node_modules/                         # Installed Node.js dependencies
├── .gitignore                            # Git file ignore list
├── CODE_OF_CONDUCT.md                    # Contributor Covenant Code of Conduct
├── CONTRIBUTING.md                       # Contribution guidelines and coding conventions
├── index.html                            # Frontend HTML entry point
├── package-lock.json                     # Locked npm dependencies
├── package.json                          # Frontend package manifest, scripts, and dependencies
├── postcss.config.js                     # PostCSS configuration for Tailwind CSS
├── README.md                             # Comprehensive project documentation and features
├── tailwind.config.js                    # Tailwind styling configuration and color themes
├── tsconfig.app.json                     # TypeScript compiler configuration for application code
├── tsconfig.json                         # TypeScript root configuration
├── tsconfig.node.json                    # TypeScript compiler configuration for Vite tooling
└── vite.config.ts                        # Vite configuration with React plugin and Tauri port 1420
```

## Directory Purposes

**`src/`:**
- **Purpose:** Houses the complete frontend presentation layer.
- **Contains:** React 19 view components, layout shell, visual charts, Zustand stores, and TypeScript bindings.
- **Key files:** `src/app/App.tsx`, `src/app/main.tsx`, `src/types/bindings.ts`, `src/utils/tauri.ts`.

**`src/app/`:**
- **Purpose:** Coordinates the top-level application root, theme synchronization, keyboard shortcuts, and structural framing.
- **Contains:** `App.tsx` (the central router/view container), `main.tsx` (DOM mount), and `index.css` (Tailwind design tokens).
- **Key files:** `src/app/App.tsx`.

**`src/components/`:**
- **Purpose:** Reusable atomic and layout UI components shared across views.
- **Contains:** Layout elements (`Header.tsx`, `Sidebar.tsx`, `StatusBar.tsx`, `CommandPalette.tsx`), common design primitives (`Badge.tsx`, `Button.tsx`, `Card.tsx`, `DataTable.tsx`, `Modal.tsx`, `Tabs.tsx`), cleanup modals, and D3 visualization wrappers (`SunburstChart.tsx`, `TreemapChart.tsx`).
- **Key files:** `src/components/layout/Sidebar.tsx`, `src/components/visualizations/TreemapChart.tsx`.

**`src/features/`:**
- **Purpose:** Feature-specific tab views rendered inside the main application viewport.
- **Contains:** 11 tab screens: `dashboard`, `explorer`, `analysis`, `intelligence`, `developer`, `hidden`, `applications`, `duplicates`, `cleanup`, `history`, `settings`.
- **Key files:** `src/features/dashboard/DashboardView.tsx`, `src/features/cleanup/CleanupCenterView.tsx`, `src/features/developer/DeveloperStorageView.tsx`.

**`src/stores/`:**
- **Purpose:** Reactive state management isolated by domain, preventing unnecessary component re-renders.
- **Contains:** 10 Zustand stores handling scan progress, drives, classification, analytics, intelligence recommendations, cleanup plans, and visualization drill-down.
- **Key files:** `src/stores/scanStore.ts`, `src/stores/driveStore.ts`, `src/stores/cleanupStore.ts`.

**`src-tauri/`:**
- **Purpose:** Native Rust backend core and Tauri desktop integration.
- **Contains:** Cargo crate configuration, migration scripts, capability permissions, and Rust source tree.
- **Key files:** `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs`.

**`src-tauri/src/commands/`:**
- **Purpose:** Tauri IPC command handler bridge.
- **Contains:** Rust functions tagged with `#[tauri::command]` that receive invocations from the frontend, extract arguments, access Tauri managed state, and return serializable results.
- **Key files:** `src-tauri/src/commands/scanner.rs`, `src-tauri/src/commands/cleanup.rs`, `src-tauri/src/commands/analysis.rs`.

**`src-tauri/src/models/`:**
- **Purpose:** Strongly-typed domain models and Serde data contracts shared between commands, services, and the frontend.
- **Contains:** Structs and enums for scans, drives, categories, intelligence recommendations, cleanup candidates, and visualization nodes.
- **Key files:** `src-tauri/src/models/scan.rs`, `src-tauri/src/models/classification.rs`, `src-tauri/src/models/cleanup.rs`.

**`src-tauri/src/services/`:**
- **Purpose:** Core business logic, hardware interaction, and background operations.
- **Contains:** Filesystem traversal engine, classification rule matcher, developer tool cache scanner, staged duplicate hasher, and cleanup safeguards.
- **Key files:** `src-tauri/src/services/scanner/engine.rs`, `src-tauri/src/services/cleanup/safeguards.rs`, `src-tauri/src/services/classifier/rules.rs`.

**`src-tauri/src/storage/`:**
- **Purpose:** SQLite connection pooling, PRAGMA configuration, and schema migration management.
- **Contains:** `DbPool` (mutex-wrapped rusqlite connection) and `Migrator` (runs transactional embedded SQL migrations).
- **Key files:** `src-tauri/src/storage/connection.rs`, `src-tauri/src/storage/migrator.rs`.

**`src-tauri/migrations/`:**
- **Purpose:** Version-controlled SQLite schema migrations.
- **Contains:** Plain SQL scripts executed sequentially on startup to establish and evolve the database structure.
- **Key files:** `src-tauri/migrations/0001_initial_schema.sql` through `0008_cleanup_audit_history.sql`.

**`docs/`:**
- **Purpose:** Technical documentation, security architecture audits, test verification reports, and release build manuals.
- **Contains:** Security threat model (`SECURITY_AUDIT.md`), benchmark verification (`TEST_REPORT.md`), and packaging instructions (`WINDOWS_RELEASE_GUIDE.md`).
- **Key files:** `docs/SECURITY_AUDIT.md`, `docs/TEST_REPORT.md`.

## Key File Locations

**Entry Points:**
- `src-tauri/src/main.rs`: Native desktop executable entry point. Configures Windows subsystem and hands off to `diskatlas_lib::run()`.
- `src-tauri/src/lib.rs`: Tauri desktop core setup, initializes logging, SQLite connection, scanner manager, and registers all 34 IPC commands.
- `src/app/main.tsx`: Webview frontend bootstrapping, mounts React 19 root into `#root` DOM element.
- `src/app/App.tsx`: Central frontend UI shell, handles window layout, global shortcuts, and view switching.

**Configuration:**
- `src-tauri/tauri.conf.json`: Tauri v2 configuration, window sizing (1280x840), capabilities, and packaging metadata.
- `src-tauri/Cargo.toml`: Rust dependencies, edition 2021, optimization profile (LTO = true, codegen-units = 1, panic = "abort", strip = true).
- `package.json`: Node dependencies (React 19, Vite 6, Tailwind 3.4, Zustand 5, D3 7, TanStack Table 8).
- `vite.config.ts`: Vite build tooling, sets development server port to `1420` to coordinate with Tauri.
- `tailwind.config.js`: Custom styling tokens (colors: `canvas`, `surface`, `accent`, `border`, `text`).
- `src-tauri/capabilities/default.json`: Tauri v2 application permission capabilities.

**Core Logic:**
- `src-tauri/src/services/scanner/engine.rs`: High-performance filesystem traversal engine running on dedicated OS thread with APFS fast-path seeding, sparse bounding, and batch persistence.
- `src-tauri/src/services/scanner/manager.rs`: Scan lifecycle coordinator managing atomic flags and session tracking.
- `src-tauri/src/services/classifier/rules.rs`: Deterministic 16-category rule matcher evaluating files inline during traversal.
- `src-tauri/src/services/cleanup/safeguards.rs`: Operating system protection rails preventing deletion of critical system, boot, and driver files.
- `src-tauri/src/services/analysis/mod.rs`: Staged duplicate detection (exact size -> 4KB prefix hash -> full SHA-256) and large/forgotten file queries.
- `src-tauri/src/services/intelligence/mod.rs`: Storage recommendation synthesis engine scoring disk waste by risk and recoverable bytes.
- `src/stores/scanStore.ts`: Frontend scan execution store with live throttled event listeners and polling heartbeat.
- `src/utils/tauri.ts`: Safe IPC invocation wrapper with structured error handling.

**Testing:**
- `src-tauri/src/services/scanner/mod.rs`: Integration tests for synthetic directory scanning and live progress reporting.
- `src-tauri/src/storage/migrator.rs`: Unit tests for idempotent SQLite migrations on empty and populated databases.
- `docs/TEST_REPORT.md`: Comprehensive test report documenting scan throughput, duplicate accuracy, and benchmark metrics.

## Naming Conventions

**Files:**
- React components: PascalCase with `.tsx` extension (`DashboardView.tsx`, `TreemapChart.tsx`, `DataTable.tsx`).
- Zustand stores: camelCase with `Store.ts` suffix (`scanStore.ts`, `driveStore.ts`, `cleanupStore.ts`).
- Utility modules: camelCase with `.ts` extension (`formatters.ts`, `tauri.ts`).
- Type definition files: camelCase with `.ts` extension (`bindings.ts`, `ui.ts`).
- Rust modules & source files: snake_case with `.rs` extension (`engine.rs`, `drive_detector.rs`, `safeguards.rs`).
- SQL migration scripts: 4-digit sequential numeric prefix followed by snake_case description (`0001_initial_schema.sql`, `0008_cleanup_audit_history.sql`).

**Directories:**
- Frontend directories: kebab-case or single lowercase words (`features/`, `components/visualizations/`, `stores/`, `utils/`).
- Rust modules: snake_case or single lowercase words (`commands/`, `models/`, `services/scanner/`, `storage/`).

**Code Symbols:**
- Rust structs and enums: PascalCase (`ScanOptions`, `ScanProgress`, `StorageCategory`, `AppError`).
- Rust functions and methods: snake_case (`start_scan`, `get_allocated_bytes`, `emit_progress_if_needed`).
- Rust constants: UPPER_SNAKE_CASE (`STATUS_RUNNING`, `STATUS_PAUSED`, `MIGRATIONS`).
- TypeScript interfaces and types: PascalCase (`DriveInfo`, `ScanSummary`, `CleanupPlan`).
- React component functions: PascalCase (`Header`, `Sidebar`, `ScanModal`).
- Zustand store hooks: camelCase prefixed with `use` (`useScanStore`, `useUIStore`, `useCleanupStore`).

## Where to Add New Code

**New Feature View (Tab):**
- Create view component: `src/features/<new_feature>/<NewFeature>View.tsx`.
- Register tab identifier in `src/types/ui.ts` under `NavigationTab` type union.
- Add navigation item in `src/components/layout/Sidebar.tsx` with appropriate Lucide icon and hotkey.
- Add rendering switch case in `src/app/App.tsx:renderContent()`.
- Create corresponding Zustand store in `src/stores/<newFeature>Store.ts`.

**New Backend Domain Service & IPC Command:**
- Define domain models: `src-tauri/src/models/<domain>.rs` with `#[derive(Serialize, Deserialize)]`.
- Re-export models in `src-tauri/src/models/mod.rs`.
- Implement business logic: `src-tauri/src/services/<domain>/mod.rs`.
- Declare service in `src-tauri/src/services/mod.rs`.
- Implement Tauri command handler: `src-tauri/src/commands/<domain>.rs` using `#[tauri::command]`.
- Re-export command in `src-tauri/src/commands/mod.rs`.
- Register command in `src-tauri/src/lib.rs` inside `tauri::generate_handler![...]`.
- Mirror TypeScript interfaces in `src/types/bindings.ts`.
- Call command from frontend store via `safeInvoke('<command_name>', args)` in `src/stores/`.

**New Database Migration:**
- Create migration script: `src-tauri/migrations/000X_<descriptive_name>.sql` with `CREATE TABLE` and `CREATE INDEX` statements.
- Register migration in `src-tauri/src/storage/migrator.rs` inside the `MIGRATIONS` array constant with incremented version number.
- Update migration count assert in `src-tauri/src/storage/migrator.rs` test suite.

**New Visual Chart / Visualization:**
- Create D3 chart component: `src/components/visualizations/<NewChart>.tsx`.
- Define data requirements in `src/stores/visualizationStore.ts` or add new query command in `src-tauri/src/commands/visualization.rs`.
- Embed chart inside `src/features/dashboard/DashboardView.tsx` or `src/features/analysis/StorageAnalysisView.tsx`.

**Shared Utility Function:**
- Formatting helpers (numbers, bytes, strings): `src/utils/formatters.ts`.
- Native platform IPC helpers: `src/utils/tauri.ts`.

## Special Directories

**`src-tauri/migrations/`:**
- **Purpose:** Contains embedded schema migration scripts.
- **Generated:** No (hand-authored SQL).
- **Committed:** Yes.
- **Behavior:** Bundled directly into the native compiled binary using Rust's `include_str!` macro.

**`src-tauri/capabilities/`:**
- **Purpose:** Tauri v2 capability security configuration declaring permission manifests.
- **Generated:** No.
- **Committed:** Yes.

**`src-tauri/target/`:**
- **Purpose:** Rust compilation artifacts, intermediate object files, and compiled binaries.
- **Generated:** Yes (by `cargo build` / `cargo tauri dev`).
- **Committed:** No (ignored by `.gitignore`).

**`dist/`:**
- **Purpose:** Production-optimized frontend build output (HTML, JS, CSS, bundled SVG).
- **Generated:** Yes (by `npm run build`).
- **Committed:** No (ignored by `.gitignore`).

**`.diskatlas_data/`:**
- **Purpose:** Local fallback storage folder created on disk when OS application data directory is unavailable, containing `diskatlas.db`.
- **Generated:** Yes (at runtime by `diskatlas_lib`).
- **Committed:** No (ignored by `.gitignore`).

---

*Structure analysis: 2026-11-06*
