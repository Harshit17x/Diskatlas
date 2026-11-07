# DiskAtlas: Project Roadmap

## Milestone 1: Production Hardening, Security & Concurrency (v1.0.1)

Goal: Resolve security and safety vulnerabilities, eliminate cross-platform path incompatibilities, optimize database concurrency, and clean up technical debt identified in the brownfield codebase audit.

---

### Phase 1: Security Safeguards & Integrity (Completed)
**Goal:** Harden file deletion operations, prevent path traversal bypasses, eliminate SQL injection vectors, enforce Content Security Policy, and ensure drive scanner execution and state resets.
- **Requirements Covered:** `REQ-SEC-01`, `REQ-SEC-02`, `REQ-SEC-03`, `REQ-SEC-04`, `REQ-SCAN-01`
- **Plans:**
  - [x] `01-01`: SQL Query Parameterization (Audit log queries and duplicate filter queries in Rusqlite).
  - [x] `01-02`: Path Canonicalization & Traversal Safeguards in `safeguards.rs`.
  - [x] `01-03`: Symlink/Junction Deletion Logic Fix (unlink symlinks without recursive tree deletion).
  - [x] `01-04`: WebView CSP Configuration in `tauri.conf.json`.
  - [x] `01-05`: Drive Scanner Target Initialization, Script Support & Modal State Reset.
- **Success Criteria:**
  - Zero raw SQL string interpolation with user/status input in `src-tauri/src/services/`.
  - Attempts to target system paths via `..` relative traversals are rejected by safeguards.
  - Deleting a junction or directory symlink leaves the target directory contents untouched.
  - Strict CSP configured and active in WebView.
  - `npm run drive` starts application dev server; drive scanner launches reliably without getting locked in previous completion states.

---

### Phase 2: Cross-Platform & OS Compatibility
**Goal:** Ensure seamless operation on Windows, macOS, and Linux without platform-specific bugs.
- **Requirements Covered:** `REQ-PLAT-01`, `REQ-PLAT-02`, `REQ-PLAT-03`
- **Plans:**
  - `02-01`: Cross-Platform Path Normalization in Developer Intelligence SQL queries.
  - `02-02`: macOS Full Disk Access (FDA) Permission Detection & Guidance Dialog.
  - `02-03`: Locale-Agnostic APFS Container Resolution.
- **Success Criteria:**
  - Developer artifact queries return correct project artifacts on Windows file paths.
  - macOS users without Full Disk Access receive clear, non-intrusive setup instructions.
  - APFS container size detection works reliably on non-English system locales.

---

### Phase 3: Concurrency & Database Performance
**Goal:** Eliminate UI lag during high-throughput scans and optimize query performance on multi-terabyte drives.
- **Requirements Covered:** `REQ-PERF-01`, `REQ-PERF-02`, `REQ-PERF-03`
- **Plans:**
  - `03-01`: Database Migration for Composite Performance Indexes.
  - `03-02`: Reader/Writer SQLite Connection Decoupling.
  - `03-03`: Bounded In-Memory Directory Map Buffering.
- **Success Criteria:**
  - Duplicate detection queries execute with index lookups rather than sequential table scans.
  - UI queries (category stats, large files) complete without blocking on active scanner write transactions.
  - Peak memory footprint remains under 50 MB during whole-drive scans with >1M entries.

---

### Phase 4: Technical Debt Remediation & Settings Maintenance
**Goal:** Clean up lingering TODOs, implement missing maintenance routines, and ensure long-term stability.
- **Requirements Covered:** `REQ-DEBT-01`, `REQ-DEBT-02`, `REQ-DEBT-03`
- **Plans:**
  - `04-01`: Dynamic Chrono UTC ISO Timestamp Integration.
  - `04-02`: Scan Session Memory Management & Eviction.
  - `04-03`: Backend IPC Handlers for Database Integrity Verification & VACUUM.
- **Success Criteria:**
  - Real ISO-8601 timestamps generated in intelligence reports.
  - Memory reclaimed after scan completion; no session retention leak in `ScanManager`.
  - Database Maintenance buttons in `SettingsView` trigger real operations and display status feedback.

---

## Milestone 2: Developer Ecosystems & Storage Automation (v1.1.0)

Goal: Expand tooling detection to additional languages and frameworks, and provide user-configurable storage automation rules.

- **Phase 5: Extended Developer Ecosystems** (`REQ-ECO-01`)
  - Go module cache (`~/go/pkg/mod`), .NET NuGet packages, Ruby Bundler gems, Swift Package Manager, Flutter/Dart cache.
- **Phase 6: Storage Profiles & Proactive Alerts** (`REQ-AUTO-01`, `REQ-EXPORT-01`)
  - Automated threshold alerts (e.g. notify when Docker cache exceeds 30 GB).
  - Export capabilities (JSON, CSV, PDF report generator).

