# Codebase Concerns & Technical Risk Audit

<Codebase Concerns>
**Analysis Date:** 2026-11-06

This document presents a comprehensive technical debt, security, performance, and architectural risk assessment for the DiskAtlas codebase. Findings are derived from code inspection across `src/`, `src-tauri/`, database migrations, IPC boundaries, build configurations, and test reports.

---

## Tech Debt

**Fake Timestamp Generation in Recommendation Engine:**
- Issue: Hardcoded pseudo-ISO timestamp generator using integer math (`1970 + (now / 31536000)`) produces inaccurate timestamps (`YYYY-01-01T00:00:00Z`), ignoring months, days, leap years, and time components.
- Files: [`src-tauri/src/services/intelligence/mod.rs#L727-L733`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/intelligence/mod.rs#L727-L733)
- Impact: Every recommendation generated in 2026 receives `2026-01-01T00:00:00Z` as its creation timestamp, corrupting chronological sorting and audit fidelity.
- Fix approach: Replace custom string formatting with standard `chrono` crate (`chrono::Utc::now().to_rfc3339()`) or SQLite's native `CURRENT_TIMESTAMP`.

**Memory Leak in ScanManager Session Registry:**
- Issue: `ScanManager::start_scan` stores active scan handles into `self.sessions: Arc<Mutex<HashMap<String, Arc<ActiveScanHandle>>>>`, but entries are never removed or evicted upon scan completion, cancellation, or failure.
- Files: [`src-tauri/src/services/scanner/manager.rs#L95-L98`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/manager.rs#L95-L98)
- Impact: In long-running desktop sessions with repeated scans, `ActiveScanHandle` instances accumulate indefinitely in heap memory. Furthermore, `get_progress()` repeatedly returns stale in-memory progress objects instead of querying historical states.
- Fix approach: Implement an explicit cleanup lifecycle hook or weak-reference cache that removes session handles once `status` reaches terminal states (`STATUS_COMPLETED`, `STATUS_CANCELLED`, `STATUS_FAILED`).

**Dead/Unused Tokio Notify & Spin-Sleep Pause Loop:**
- Issue: `pause_notify: Arc<tokio::sync::Notify>` is initialized and passed into `run_scan` as `_pause_notify`, but never awaited. The dedicated scan thread instead implements pause via an atomic poll and sleep loop: `while status.load(Ordering::SeqCst) == STATUS_PAUSED { std::thread::sleep(Duration::from_millis(50)); }`.
- Files: [`src-tauri/src/services/scanner/manager.rs#L21`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/manager.rs#L21), [`src-tauri/src/services/scanner/engine.rs#L79`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/engine.rs#L79), [`src-tauri/src/services/scanner/engine.rs#L252-L256`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/engine.rs#L252-L256)
- Impact: Dead code in parameter signatures, while pause resumption incurs unnecessary latency (up to 50ms per check) and redundant CPU context switching.
- Fix approach: Use standard thread synchronization (`std::sync::Condvar` or crossbeam channels) suited for dedicated OS threads rather than combining tokio `Notify` with OS thread spin-sleeping.

**Non-Functional Placeholder UI Buttons:**
- Issue: "Verify Database Integrity" and "VACUUM Database" action buttons in the Settings UI render without any `onClick` event handlers or IPC bindings.
- Files: [`src/features/settings/SettingsView.tsx#L153-L159`](file:///Users/harshitbisht/Desktop/DiskAtlas/src/features/settings/SettingsView.tsx#L153-L159)
- Impact: Users attempting maintenance operations receive no response or feedback, creating an impression of broken functionality.
- Fix approach: Bind buttons to backend commands that invoke SQLite `PRAGMA integrity_check;` and `VACUUM;`.

**Hardcoded Path Traversal Separators in Multi-Platform Queries:**
- Issue: Child path queries in developer storage and cleanup deletion use hardcoded forward-slash concatenation: `format!("{}/%", root_path.trim_end_matches('/'))`.
- Files: [`src-tauri/src/services/developer/mod.rs#L85`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/developer/mod.rs#L85), [`src-tauri/src/services/cleanup/mod.rs#L325`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs#L325), [`src-tauri/src/services/cleanup/mod.rs#L414`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs#L414)
- Impact: On Windows systems where paths are indexed using backslashes (e.g., `C:\Users\User\Project\file.txt`), the SQL pattern `C:\Users\User\Project/%` fails to match child records. Consequently, developer project storage metrics return 0 bytes, and cleanup operations fail to synchronize the SQLite index after directory deletion.
- Fix approach: Normalize path separators uniformly to forward-slashes upon database ingestion or use `std::path::MAIN_SEPARATOR` when building prefix patterns.

---

## Known Bugs

**SQL String Interpolation in Audit History Queries:**
- Symptoms: Passing unescaped characters or SQL fragments in `session_id` or `status` parameters crashes queries or permits arbitrary query injection.
- Files: [`src-tauri/src/services/cleanup/mod.rs#L521-L557`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs#L521-L557)
- Trigger: Opening the Cleanup Audit History tab with a maliciously crafted query filter or corrupted session string.
- Workaround: Bind parameters using `rusqlite::params![]` rather than formatting parameters directly into query strings.

**SQL String Interpolation in Duplicate File Search:**
- Symptoms: Providing an unescaped file extension in the Duplicate Search query triggers SQL syntax errors or query injection.
- Files: [`src-tauri/src/services/analysis/mod.rs#L254-L265`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/analysis/mod.rs#L254-L265)
- Trigger: Executing duplicate discovery filtered by an extension like `' OR 1=1 --`.
- Workaround: Use parameterized binding (`AND LOWER(extension) = ?3`) and pass extension as a parameter value.

**macOS Localized Capacity Failure in APFS Container Detection:**
- Symptoms: On macOS installations configured for languages other than English (e.g. Japanese, German, French, Spanish), drive capacity resolution falls back to sealed snapshot sizes (~15 GB) rather than full physical container size.
- Files: [`src-tauri/src/services/drive_detector.rs#L359-L402`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/drive_detector.rs#L359-L402)
- Trigger: Running DiskAtlas on any non-English macOS operating system.
- Workaround: Invoke `diskutil info -plist /` to parse structured XML/plist key `TotalSize` / `FreeSpace` rather than relying on English string matching (`Container Total Space:`).

---

## Security Considerations

**Direct SQL Injection Vulnerabilities (Severity: HIGH):**
- Risk: Unsanitized user inputs passed from the frontend over the Tauri IPC channel are directly formatted into raw SQL queries without parameter binding.
- Files:
  - [`src-tauri/src/services/cleanup/mod.rs#L524-L555`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs#L524-L555) (`query.session_id` and `query.status` interpolated into `count_sql` and `query_sql`).
  - [`src-tauri/src/services/analysis/mod.rs#L254-L265`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/analysis/mod.rs#L254-L265) (`query.extension` interpolated into `stage1_sql`).
- Current mitigation: The frontend is local and air-gapped, preventing external network injection; however, IPC boundaries must treat all frontend payloads as untrusted.
- Recommendations: Replace all raw string interpolations (`format!("SELECT ... WHERE session_id = '{}'", sid)`) with parameterized statements using `stmt.query_map(params![sid, ...])`.

**Path Traversal & Relative Path Safeguard Bypass (Severity: HIGH):**
- Risk: Security safeguards check string prefixes on uncanonicalized `Path` objects. Relative traversal segments (`..`) or symbolic links pointing to protected directories can evade prefix verification.
- Files: [`src-tauri/src/services/cleanup/safeguards.rs#L5-L109`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/safeguards.rs#L5-L109), [`src-tauri/src/services/cleanup/safeguards.rs#L158-L173`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/safeguards.rs#L158-L173)
- Current mitigation: Basic lowercase string comparisons match against a static list of roots (`/System`, `C:\Windows`, etc.).
- Recommendations: Canonicalize and resolve all paths using `std::fs::canonicalize` prior to safeguard evaluation, reject any path containing unresolved `..` components, and inspect file metadata with `symlink_metadata` rather than following links.

**Destructive Recursive Deletion on Symlinks & Junctions (Severity: HIGH):**
- Risk: When permanent cleanup is selected, `path_obj.is_dir()` is evaluated. In Rust's standard library, `is_dir()` follows symbolic links and Windows directory junctions. If a candidate item is a directory symlink or junction, calling `std::fs::remove_dir_all(path_obj)` can delete the target directory's contents rather than simply unlinking the link itself.
- Files: [`src-tauri/src/services/cleanup/mod.rs#L376-L380`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs#L376-L380)
- Current mitigation: Safeguard re-validation blocks designated critical paths.
- Recommendations: Always inspect `path_obj.symlink_metadata()`. If `file_type().is_symlink()` is true, use `std::fs::remove_file` (or platform-specific `remove_dir` for junctions) to unlink the pointer without traversing into child contents.

**Discrepancy Between Security Audit Claims and Configuration (Severity: MEDIUM):**
- Risk: [`docs/SECURITY_AUDIT.md#L18-L22`](file:///Users/harshitbisht/Desktop/DiskAtlas/docs/SECURITY_AUDIT.md#L18-L22) claims that `tauri.conf.json` enforces a strict CSP blocking remote outbound network connections (`connect-src 'self' ipc:`). However, [`src-tauri/tauri.conf.json#L25`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/tauri.conf.json#L25) sets `"csp": null`.
- Files: [`src-tauri/tauri.conf.json#L25`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/tauri.conf.json#L25), [`docs/SECURITY_AUDIT.md#L18-L22`](file:///Users/harshitbisht/Desktop/DiskAtlas/docs/SECURITY_AUDIT.md#L18-L22)
- Current mitigation: The frontend currently contains no external dependencies or remote fetch calls.
- Recommendations: Align `tauri.conf.json` with the documented CSP specification:
  ```json
  "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: asset:; font-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost"
  ```

---

## Performance Bottlenecks

**Global SQLite Mutex Contention (Single Connection Architecture):**
- Problem: All database operations across all threads share a single `Arc<Mutex<Connection>>` in `DbPool`.
- Files: [`src-tauri/src/storage/connection.rs#L8-L11`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/storage/connection.rs#L8-L11)
- Cause: Even though SQLite is configured with WAL mode (`journal_mode = WAL`), which permits concurrent readers alongside a writer, wrapping a single connection in a standard `Mutex` serializes all database queries. Long-running batch inserts (2,500 records per transaction) completely freeze concurrent UI queries (analytics, file browsing, health checks).
- Improvement path: Introduce a true connection pool (e.g. `r2d2-sqlite`) with dedicated read connections and a single dedicated write connection.

**Unindexed Full Table Scans for Large Files & Duplicate Groups:**
- Problem: Queries in `LargeFilesQuery` and `find_duplicates` (Stage 1) execute multi-second full table scans over millions of rows.
- Files: [`src-tauri/src/services/analysis/mod.rs#L28-L74`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/analysis/mod.rs#L28-L74), [`src-tauri/src/services/analysis/mod.rs#L258-L272`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/analysis/mod.rs#L258-L272), [`src-tauri/migrations/0002_files_and_directories.sql`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/migrations/0002_files_and_directories.sql)
- Cause: The `files` table contains indexes for `(session_id, parent_path)`, `(session_id, path)`, and `(session_id, modified_at)`, but lacks an index on `(session_id, size_bytes)`.
- Improvement path: Add an index:
  ```sql
  CREATE INDEX idx_files_session_size ON files(session_id, size_bytes DESC);
  ```

**In-Memory Hidden Files Table Scan & Rust Pagination:**
- Problem: Opening the Hidden Files view triggers multi-second latency and memory spikes.
- Files: [`src-tauri/src/services/analysis/mod.rs#L136-L219`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/analysis/mod.rs#L136-L219)
- Cause: The query matches wildcard path patterns (`%AppData%`, `%.cache%`, `%/Library/%`), pulling hundreds of thousands of rows into an in-memory `Vec<HiddenItem>`. Sorting, search filtering, and pagination are performed in Rust after loading the entire candidate dataset.
- Improvement path: Push filtering, sorting, and `LIMIT`/`OFFSET` pagination down into SQL queries using parameterized indexed conditions.

**Synchronous Subprocess Execution on Command Threads:**
- Problem: Checking developer Docker storage can hang the thread if the Docker daemon is unresponsive.
- Files: [`src-tauri/src/services/developer/mod.rs#L301-L304`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/developer/mod.rs#L301-L304)
- Cause: `Command::new("docker").output()` executes blocking I/O without a timeout. If the Docker daemon pipe freezes, the command thread blocks indefinitely.
- Improvement path: Execute process checks with bounded timeouts (e.g. `tokio::process::Command` with `tokio::time::timeout(Duration::from_secs(3), ...)`).

---

## Fragile Areas

**Bottom-Up Directory Sizing Rollup Algorithm:**
- Files: [`src-tauri/src/services/scanner/engine.rs#L811-L849`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/engine.rs#L811-L849)
- Why fragile: Parent directory sizes are aggregated in-memory by sorting all directory paths descending by string length (`dir_paths.sort_by(|a, b| b.len().cmp(&a.len()))`). While child paths are strictly longer than their parent paths, string-length ordering does not guarantee topological ordering across disparate branch depths. Furthermore, if a directory is omitted from `dir_map` due to permission errors, child byte counts are dropped from root totals.
- Safe modification: When adjusting directory traversal, verify path hierarchy accumulation with deep directory structures and verify that permission-denied subtrees do not orphan byte counts.
- Test coverage: Tested with shallow synthetic folders (`test_scanner_basic_traversal`); untested on deep trees (>50 levels) or paths with unicode normalization differences.

**Recycle Bin / Trash Integration Across Volumes:**
- Files: [`src-tauri/src/services/cleanup/mod.rs#L292-L373`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs#L292-L373)
- Why fragile: `trash::delete(path)` relies on OS-specific trash infrastructure. On Linux without a FreeDesktop `.Trash` on external partitions, or on Windows network drives/FAT32 volumes lacking `$Recycle.Bin`, `trash::delete` fails with permission or unsupported operation errors.
- Safe modification: Catch trash errors and prompt the user with a distinct warning that the volume does not support reversible recycling before falling back to permanent deletion.
- Test coverage: Automated tests mock deletion with synthetic files in temp directories; cross-volume trash failures are not covered in automated suites.

**Windows MAX_PATH & Canonicalization in Traversal Loop:**
- Files: [`src-tauri/src/services/scanner/engine.rs#L322-L343`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/engine.rs#L322-L343)
- Why fragile: Windows cycle detection calls `fs::canonicalize(&current_dir)` on every directory. On NTFS, canonicalization issues Win32 `GetFinalPathNameByHandle` requests, which fail if any parent directory lacks read-attribute permissions, prematurely halting traversal for that branch.
- Safe modification: On Windows, use volume serial number and file index (`BY_HANDLE_FILE_INFORMATION` or `FileIdInfo`) rather than string canonicalization.
- Test coverage: Automated test verifies non-traversal of junctions, but does not test paths exceeding 260 characters without canonicalization.

---

## Scaling Limits

**Filesystem Traversal Inode Scale:**
- Current capacity: Efficiently traverses up to ~500,000 files in ~20-30 seconds on NVMe storage.
- Limit: At 2,000,000+ files and 500,000+ directories:
  - `dir_map: HashMap<String, RawDirItem>` retains all directories in memory until scan completion, consuming 250+ MB RAM.
  - Flushing 500,000 directories at the end in batch chunks of 2,500 causes a several-second database write pause before completion.
- Scaling path: Flush directories incrementally during traversal and calculate recursive sizes via SQL recursive CTEs or parent-child rollup passes rather than holding the entire tree in heap memory.

**SQLite Database File Size & Session Accumulation:**
- Current capacity: Supports multiple scans; typical database size ~50-100 MB per scan session.
- Limit: Without an automated pruning policy, indexing a 2 TB SSD 10 times accumulates ~10 million file rows, bloating `diskatlas.db` to multiple gigabytes and slowing down SQLite B-tree indexes.
- Scaling path: Implement automatic retention (e.g., retain the last 5 scans per drive, archive old snapshots, and expose a working VACUUM/Prune feature in Settings).

**Tauri IPC Event Emission Frequency:**
- Current capacity: Progress events are throttled to 100ms in Rust and polled at 200ms by the frontend store.
- Limit: If the emission throttle is bypassed or reduced below 50ms, Tauri's IPC serialization and WebView2 event loops saturate, causing UI thread lag and frame drops.
- Scaling path: Maintain throttling at >=150ms and emit batch summaries rather than real-time per-file events.

---

## Dependencies at Risk

**`trash` (v5):**
- Risk: Cross-platform trash operations have variable behavior across Linux desktop environments, headless setups, and non-root external storage volumes.
- Impact: Silent cleanup failure (`status: failed` or `permission_denied`) when users attempt to recycle items on network drives, USB sticks, or non-standard Linux mounts.
- Migration plan: Handle `trash::Error` variants explicitly with descriptive user feedback, and provide a verified fallback path with explicit user consent.

**`sysinfo` (v0.33):**
- Risk: Rapid breaking API changes across minor version upgrades (e.g. `Disks::new_with_refreshed_list()` vs `Disks::refresh()`).
- Impact: Breaking compilation changes when updating dependencies.
- Migration plan: Pin `sysinfo` versions and isolate all usage within `src-tauri/src/services/drive_detector.rs`.

---

## Missing Critical Features

**Database Maintenance & VACUUM Pipeline:**
- Problem: No backend command or automated background task executes SQLite integrity verification, index defragmentation, or `VACUUM`.
- Blocks: Users cannot reclaim disk space occupied by deleted scan sessions, causing unbounded database file growth.

**macOS Full Disk Access (FDA) Guidance & Entitlements:**
- Problem: No `Info.plist` usage strings (`NSSystemAdministrationUsageDescription`) or proactive checks detect whether DiskAtlas has been granted Full Disk Access.
- Blocks: Scanning `/Users/<username>/Library` silently fails on protected user directories (`Mail`, `Messages`, `Safari`), yielding incomplete storage statistics.

**Exportable Audit Reports:**
- Problem: Cleanup events are permanently recorded in `cleanup_audit_events`, but there is no feature to export audit history to CSV or JSON.
- Blocks: System administrators and users cannot export proof of deletion or compliance logs for external audits.

---

## Test Coverage Gaps

**SQL Injection Scenarios in Audit and Duplicate Queries:**
- What's not tested: Parameter boundaries for `session_id`, `status`, and `extension` filters containing SQL meta-characters.
- Files: [`src-tauri/src/services/cleanup/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs), [`src-tauri/src/services/analysis/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/analysis/mod.rs)
- Risk: Unnoticed regressions or SQL syntax errors introduced during query refactoring.
- Priority: High

**Windows Path Traversal and Backslash Matching:**
- What's not tested: Path prefix queries (`path LIKE 'C:\...'`) with backslash path separators on real Windows environments.
- Files: [`src-tauri/src/services/developer/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/developer/mod.rs), [`src-tauri/src/services/cleanup/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs)
- Risk: Developer storage discovery and cleanup synchronization fail silently on Windows.
- Priority: High

**Path Traversal with Relative Components (`..`) in Safeguards:**
- What's not tested: Paths containing relative traversal tokens (`/tmp/../System/Library`) passed into `is_system_critical_path`.
- Files: [`src-tauri/src/services/cleanup/safeguards.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/safeguards.rs)
- Risk: Malicious or accidental relative paths evading system file deletion blocks.
- Priority: High

**High-Inode Traversal Scale & Memory Stress:**
- What's not tested: Scanning simulated filesystem trees with >1,000,000 nodes and measuring memory consumption of `dir_map` and transaction batching.
- Files: [`src-tauri/src/services/scanner/engine.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/scanner/engine.rs)
- Risk: Out-of-memory crashes on constrained systems during whole-disk scans.
- Priority: Medium

---

*Concerns audit: 2026-11-06*
