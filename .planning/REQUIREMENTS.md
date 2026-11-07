# DiskAtlas: Project Requirements

## Current Core Capabilities (v1.0 Baseline)

- [x] **REQ-SCAN-01**: Multi-threaded filesystem traversal engine achieving >50,000 files/sec on modern SSDs.
- [x] **REQ-SCAN-02**: Cycle detection preventing infinite recursion on symlinks, hardlinks, and Windows NTFS junctions.
- [x] **REQ-SCAN-03**: Interactive scan controls supporting non-blocking pause, resume, and cancellation without SQLite database corruption.
- [x] **REQ-SCAN-04**: macOS APFS container unification preventing duplicate accounting of sealed system vs data volumes.
- [x] **REQ-DEV-01**: Developer artifact classification across 7+ ecosystems (Node.js, Python, Rust, Docker, Java/Android, C/C++, IDE caches).
- [x] **REQ-VIZ-01**: D3 Squarified Treemap with smooth breadcrumb drilldowns, hover tooltips, and category color codings.
- [x] **REQ-VIZ-02**: D3 Radial Sunburst visualization showing multi-level hierarchical storage distribution.
- [x] **REQ-DUP-01**: Staged duplicate file detection pipeline (Size -> Metadata -> Block Header -> Cryptographic SHA-256).
- [x] **REQ-CLEAN-01**: Reversible cleanup routing deletion to native OS Trash / Windows Recycle Bin by default.
- [x] **REQ-CLEAN-02**: System safeguards engine blocking deletion of operating system roots, boot loaders, kernel binaries, and drivers.
- [x] **REQ-CLEAN-03**: Immutable SQLite audit log recording every cleanup attempt with timestamps, paths, and status codes.
- [x] **REQ-PRIV-01**: Air-gapped architecture with zero network calls, zero tracking analytics, and strictly local SQLite persistence.

---

## Targeted Remediation Requirements (Milestone 1: Hardening & Reliability)

### Security Safeguards & Integrity (Phase 1)
- [x] **REQ-SEC-01**: **Parameterize SQL Queries in Audit & Analysis Services**
  - Replace raw SQL string formatting (`format!("... AND status = '{}'", status)`) with parameterized Rusqlite statements (`?1, ?2`) in `src-tauri/src/services/cleanup/mod.rs` and `src-tauri/src/services/analysis/mod.rs`.
- [x] **REQ-SEC-02**: **Canonicalize Paths in System Safeguards**
  - Enforce `std::fs::canonicalize()` before prefix checking in `src-tauri/src/services/cleanup/safeguards.rs` to prevent path traversal bypasses (`..`).
- [x] **REQ-SEC-03**: **Safely Unlink Directory Symlinks / Junctions**
  - When performing permanent deletion on a directory symlink or junction, invoke `std::fs::remove_file()` rather than `std::fs::remove_dir_all()` to prevent following links and deleting target tree contents.
- [x] **REQ-SEC-04**: **Restore Webview Content Security Policy (CSP)**
  - Re-enable strict CSP in `src-tauri/tauri.conf.json` restricting `connect-src 'self' ipc: http://ipc.localhost` and matching security audit specifications.

### Cross-Platform & OS Compatibility (Phase 2)
- [ ] **REQ-PLAT-01**: **Cross-Platform Path Separator Normalization in SQL Queries**
  - Fix hardcoded Unix forward-slash pattern matches (`/%`) in `developer/mod.rs` and `cleanup/mod.rs` so that Windows backslash paths (`\`) are correctly categorized and inspected.
- [ ] **REQ-PLAT-02**: **macOS Full Disk Access (FDA) Detection & Prompting**
  - Detect when the app lacks Full Disk Access on macOS (e.g. inability to read `~/Library/Application Support`) and present a friendly guidance dialog directing user to System Settings.
- [ ] **REQ-PLAT-03**: **Locale-Agnostic APFS Container Capacity Resolution**
  - Replace English regex string matching (`Capacity In Use By Volumes`) in `drive_detector.rs` with programmatic diskutil plist parsing or `sysinfo` volume metrics.

### Concurrency & Database Performance (Phase 3)
- [ ] **REQ-PERF-01**: **Compound Indexes for File Queries**
  - Add database migration providing index `idx_files_session_size ON files(session_id, size_bytes)` to eliminate table scans on large drives during duplicate and large-file queries.
- [ ] **REQ-PERF-02**: **Reader/Writer Database Concurrency Isolation**
  - Resolve mutex contention on `Arc<Mutex<Connection>>` during high-speed scans by establishing separate reader connection pooling or read-write locks for UI requests.
- [ ] **REQ-PERF-03**: **Bounded In-Memory Directory Map**
  - Stream or periodically persist nested directory records during whole-drive indexing to eliminate unbounded heap growth on filesystems with millions of folders.

### Technical Debt & Maintenance (Phase 4)
- [ ] **REQ-DEBT-01**: **Chrono ISO-8601 Timestamp Generation**
  - Replace mock `chrono_now_iso` function in `intelligence/mod.rs` with real dynamic ISO-8601 UTC timestamp formatting.
- [ ] **REQ-DEBT-02**: **Prune Stale Scan Sessions**
  - Implement session cleanup policy in `ScanManager` to evict completed or cancelled scan state and prevent memory leaks over extended app uptime.
- [ ] **REQ-DEBT-03**: **Wire Up Settings Database Maintenance**
  - Connect UI actions in `SettingsView.tsx` ("Verify Database Integrity", "VACUUM Database") to real Tauri IPC backend commands.

---

## Future Functional Requirements (Milestone 2: Ecosystems & Automation)

- [ ] **REQ-ECO-01**: Additional ecosystem classifications: Go (`~/go/pkg`), .NET/NuGet (`~/.nuget/packages`), Ruby (`~/.bundle`), Swift/SPM (`.build/`), Flutter/Dart.
- [ ] **REQ-AUTO-01**: User-defined rule engine for scheduled background warnings when developer caches exceed specified thresholds.
- [ ] **REQ-EXPORT-01**: Export scan summary and audit history to CSV, JSON, and PDF report formats.

