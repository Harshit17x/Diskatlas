# DiskAtlas — Full Codebase Review Report

**Scope:** React 19 + Vite frontend (`src/`), Tauri 2 + Rust backend (`src-tauri/src/`), SQLite schema/migrations (`src-tauri/migrations/`), plus the linking features (category → files drill-down, drive → tree → file-table linkage).
**Method:** Read-only static review of all source modules, review of the uncommitted diff, `npm run lint` (tsc, 0 errors), `cargo check` (passes), and live runtime evidence against the real app DB at `~/Library/Application Support/com.diskatlas.app/diskatlas.db` (18.1 GB) using read-only `sqlite3` queries and the existing `src-tauri/tests/live_repro.rs` harness.
**No files were modified during the review.**

---

## 1. Executive Summary

The codebase compiles cleanly and the core scan/index pipeline works, but the app is **not functioning correctly at the product level**. The most serious problems are not TypeScript or compile errors — they are **data-model, session-resolution, and query-plan defects** that make major views empty, slow, or silently wrong:

1. **The post-scan "intelligence" pipeline runs synchronously before `scan://completed`, and one of its stages (developer analysis) is effectively unbounded** — a per-project `LIKE` loop that takes ~26 s per query on the live 3.8 M-file session. On a real machine with thousands of project markers this is hours to days, holding the single global DB mutex and delaying scan completion.
2. **`category_summaries` is corrupted by a SQL alias collision.** `GROUP BY category` binds to `files.category`, not the `CASE ... END AS category` alias, so the aggregate collapses into repeated categories and `INSERT OR REPLACE` + `UNIQUE(session_id, category)` keeps only one row. The live DB has exactly one row: `other | 3,829,960 | 402,018,438,381` — the entire session mislabeled as `other`. This breaks history comparison and category trend reporting.
3. **Analytics always falls back to a 55.6-second full-table scan** because `classification_summaries` has no rows with `subcategory IS NULL`; the fast summary query returns nothing.
4. **Analysis and Intelligence default to the newest session regardless of status**, which is currently a cancelled 0-file session, so those views are empty after restart.
5. **The Explorer is not linked to the latest completed scan after restart.** The treemap can resolve a session, but the file table never fetches on mount and returns early when `activeSessionId` is null. The table is also never filtered by the current tree path.
6. **macOS root scans skip `/`**, so there is no `directories` row for `/`; the treemap root falls back with `file_count = 0`, and the default 0.2% threshold (~804 MB on this session) hides almost every node behind a synthetic "Other items" tile.

**Verdict:** Functional demo, not yet reliable for real disks. The linking features are wired but disconnected in three places (session resolution, event timing, and path propagation). Fixing the six Critical items would transform the app's usability; the High/Medium items are mostly performance, data-integrity, and UX hardening.

---

## 2. What Works

- Clean compile: `npm run lint` passes with 0 TS errors; `cargo check --message-format=short` passes.
- Scan traversal, batching, pause/resume/cancel, inode cycle protection, and WAL configuration are present and coherent.
- Treemap/Sunburst rendering, drill-down callbacks, and breadcrumb navigation are implemented.
- Classification engine and category override CRUD exist and produce 16 categories on the live session.
- History sessions list, snapshots, and comparison UI exist.
- Cleanup safeguards module exists (path validation, protected-path checks).
- IPC error wrapper (`safeInvoke`) normalizes errors consistently.

---

## 3. Findings by Severity

### Quick index

| ID | Severity | Area | Summary |
|----|----------|------|---------|
| C1 | Critical | Scan completion / Developer | Unbounded per-project `LIKE` loop runs before completion; hours–days; blocks DB |
| C2 | Critical | Aggregation / History | `category_summaries` alias collision collapses all categories into one row |
| C3 | Critical | Analytics | Category distribution always does a 55 s full-table fallback |
| C4 | Critical | Analysis / Intelligence | Newest session chosen regardless of status → empty views after restart |
| C5 | Critical | Explorer / Linking | File table not linked to completed session or current path |
| C6 | Critical | Treemap / macOS | No `/` directory row; root file_count = 0; default threshold hides nodes |
| H1 | High | Post-scan pipeline | Errors swallowed; status marked completed before pipeline |
| H2 | High | DB concurrency | One global `Mutex<Connection>`; blocking work in async commands |
| H3 | High | Classification | Rewrites every file row on every classification run |
| H4 | High | Developer | Deletes and recomputes all project rows on every view open |
| H5 | High | Intelligence | Recommendations regenerated every call; duplicates depend on empty cache |
| H6 | High | Cleanup | Deletes index rows without recomputing rollups/totals/summaries |
| H7 | High | Scan events | Foreign-session progress accepted while scanning; concurrent scans allowed |
| H8 | High | UX / Progress | No "finalizing" state; progress appears stuck after traversal |
| M1 | Medium | Linking | `fetchTree` effect omits `currentPath`; path/session mismatch |
| M2 | Medium | Linking | Drill-down table ignores tree path (`parentPath` always undefined) |
| M3 | Medium | Data model | Two category vocabularies (`code`/`executables` vs `developer`/`applications`) |
| M4 | Medium | Session resolution | Requested session ID is never validated |
| M5 | Medium | DB lifecycle | No retention/VACUUM; 18.1 GB DB grows monotonically |
| M6 | Medium | Dev CSP | CSP blocks Vite HMR/React refresh in `tauri dev` |
| M7 | Medium | Scan store | Listener/timer lifecycle leaks on unmount/cancel paths |
| M8 | Medium | Classification | View open can auto-trigger a 3.8 M-row classification |
| M9 | Medium | Cleanup | Unescaped SQL `LIKE` wildcards in path prefixes |
| M10 | Medium | History | Comparison relies on the broken `category_summaries` |
| L1 | Low | UI | `total_pages` forced to 1 when empty; page-size clamp mismatch |
| L2 | Low | UI | "Other items" synthetic node is not drillable |
| L3 | Low | Errors | Silent pipeline failures leave partial data with no user signal |
| L4 | Low | Repo | Large uncommitted diff + untracked `.planning/`, `tui/`, `tests/` |
| L5 | Low | Data | 37.7% of bytes classified `unknown`; 929k files have null extension |
| L6 | Low | Config | Bundle targets are Windows-only (`nsis`, `msi`) despite macOS dev |

---

## 4. Critical Findings (Detailed)

### C1 — Developer analysis makes scan completion effectively unbounded

**Where:** `src-tauri/src/services/scanner/engine.rs:648-663`, `src-tauri/src/services/developer/mod.rs:79-100`

The scan finishes traversal, updates the session to completed (`engine.rs:625`), then runs:

1. classification,
2. developer analysis,
3. intelligence recommendations,

**before** emitting `scan://completed` (`engine.rs:689-691`).

Developer analysis iterates every detected project root and runs a separate query:

```sql
SELECT path, size_bytes, modified_at
FROM files
WHERE session_id = ?1
  AND (path = ?2 OR path LIKE ?3)
```

**Live evidence:**
- `EXPLAIN QUERY PLAN` → `SEARCH files USING INDEX idx_files_session_mod (session_id=?)` — SQLite does **not** use `idx_files_session_path`; it scans the whole session for each project.
- Measured three runs of one `LIKE` prefix query on session `85db2339` (3,829,960 files): **26.26 s, 26.04 s, 25.36 s**.
- The harness derives on the order of ~19k marker-based project roots; at ~26 s each that is roughly **137 hours** of finalization.

**Impact:** A scan on a developer machine can appear complete in the DB but never fire `scan://completed`; the UI sits on "Finalizing…"; the global DB mutex is held for the whole loop, so every other view hangs. Re-opening the Developer tab re-runs the same analysis (`commands/developer.rs:11-15`, `DeveloperStorageView.tsx:45-48`).

**Fix direction:**
- Rewrite project attribution as a single set-based query (join markers to files by `path` prefix) or precompute a `project_root` column during scanning.
- Use `path GLOB ?` with a case-sensitive/prefix-friendly form, or `substr(path,1,length(?)) = ?`, and ensure an index is usable (`session_id, path`).
- Move the pipeline off the completion path: emit `scan://completed` first, run enrichment asynchronously with per-stage progress/status, and surface failures.
- Cache developer results per session and invalidate only on new scan.

---

### C2 — `category_summaries` aggregation is wrong and collapses to one row

**Where:** `src-tauri/src/services/scanner/engine.rs:823-844`, `src-tauri/migrations/0003_storage_history_and_snapshots.sql:19-27`

```sql
INSERT OR REPLACE INTO category_summaries (session_id, category, file_count, total_bytes)
SELECT session_id,
       CASE ... END AS category,
       COUNT(*), COALESCE(SUM(size_bytes),0)
FROM files
WHERE session_id = ?1
GROUP BY category;
```

SQLite resolves `GROUP BY category` to the **base-table column** `files.category` (the classifier's category), not the `CASE` alias. The SELECT list then emits the `CASE` value, so rows are grouped by one thing and labeled by another.

**Live reproduction:** the equivalent query returned **16 groups with repeated `other` and `code` categories** (e.g. `other` appears for unknown, gaming, temporary, vm_emulators, browsers, cloud, hidden, backups).

Then `UNIQUE(session_id, category)` + `INSERT OR REPLACE` keeps only the last row per label. The live DB confirms:

```
category_summaries for 85db2339:
other | 3829960 | 402018438381
```

The session total is 3,829,960 files / 402,018,438,381 bytes, so **100% of the session is recorded as `other`**.

**Impact:** History comparison (`history/mod.rs:423-440`) computes category deltas from this table, so every comparison is meaningless. Any future trend/dashboard built on `category_summaries` inherits the corruption.

**Fix direction:**
- Use a subquery/CTE to force the alias: `SELECT ... FROM (SELECT CASE ... END AS category, size_bytes FROM files WHERE session_id = ?1) GROUP BY category`.
- Or repeat the `CASE` in `GROUP BY` (SQLite allows ordinal `GROUP BY 2`).
- Add a migration/backfill to rebuild summaries for existing completed sessions.
- Consider deriving `category_summaries` from `classification_summaries` so there is one source of truth.

---

### C3 — Analytics category distribution always falls back to a 55-second scan

**Where:** `src-tauri/src/services/visualization/mod.rs:231-267`

```sql
SELECT category, file_count, total_bytes
FROM classification_summaries
WHERE session_id = ?1 AND subcategory IS NULL
ORDER BY total_bytes DESC
```

The classifier writes **only subcategory rows** — live check shows 16 categories but **0 rows with `subcategory IS NULL`** for `85db2339`. So `category_distribution` is always empty and the fallback runs:

```sql
SELECT COALESCE(category,'unknown'), COUNT(*), SUM(size_bytes)
FROM files WHERE session_id = ?1 GROUP BY cat ...
```

**Live timing:** fallback **55.59 s**; the summary query **0.01 s**. `EXPLAIN QUERY PLAN` shows `SEARCH files USING INDEX idx_files_session_mod (session_id=?)` + `USE TEMP B-TREE FOR GROUP BY`, i.e. it reads all 3.8 M rows.

**Impact:** The Analytics tab takes ~1 minute on first load and blocks the global DB mutex, freezing other views.

**Fix direction:**
- Change the query to aggregate `classification_summaries` by `category` (`GROUP BY category`, summing file_count/total_bytes) instead of filtering `subcategory IS NULL`.
- If a top-level row is desired, have the classifier insert one (`subcategory = NULL`) per category.
- Add a covering index `(session_id, category)` on `classification_summaries` if not present.

---

### C4 — Analysis and Intelligence resolve the newest session regardless of status

**Where:** `src-tauri/src/services/analysis/mod.rs:616-627`, `src-tauri/src/services/intelligence/mod.rs:705-716`

```sql
SELECT id FROM scan_sessions ORDER BY started_at DESC LIMIT 1
```

**Live session order (newest first):**
```
7a02a2db | cancelled | 0 files | 2026-11-06 13:50:30
1b24ee7b | cancelled | 0 files
08fa8389 | cancelled | 0 files
85db2339 | completed_with_errors | 3,829,960 files
```

So after restart, Analysis and Intelligence target the cancelled 0-file session and render empty, even though a valid completed session exists.

Visualization does it correctly (`visualization/mod.rs:446-467`: only `completed`, `completedWithErrors`, `completed_with_errors`, `interrupted`). The inconsistency is the bug.

**Fix direction:**
- Centralize session resolution in one helper used by all services.
- Prefer `completed`/`completed_with_errors`; only fall back to `interrupted`/`cancelled` if it has `total_files > 0`.
- Validate a requested session ID belongs to a real, non-empty session.

---

### C5 — Explorer is not linked to the latest completed session or current path

**Where:** `src/features/explorer/DriveExplorerView.tsx:40, 69-71, 113-119`; `src/stores/scanStore.ts:326-335`

Several disconnects:

1. **Initial path** is the selected drive's mount point (`DriveExplorerView.tsx:40`), which may not equal the session's `root_path`.
2. **Tree fetch effect** depends only on `[fetchTree, activeSessionId]` (`69-71`), omitting `currentPath`. Path changes rely on manual refetches (drive change at `254-257`) or store-driven `drillDown`.
3. **File table never fetches on mount.** `fetchScanFiles` is only called from sort/page handlers (`113-119`) and scan completion. On a fresh app launch the File Registry table stays empty.
4. **Early return when no session:** `fetchScanFiles` does `if (!targetSessionId) return;` (`scanStore.ts:334-335`), so even if it were called on mount it would no-op after restart when `activeSessionId` is null.
5. **Table is never path-filtered:** both calls pass `undefined` for `parentPath` (`115`, `119`), so the table always shows the whole session, never the drilled-down directory.
6. **Drill-down updates the store, not the table:** `TreemapChart` calls `drillDown` (`TreemapChart.tsx:124-126`), which fetches the tree but never triggers `fetchScanFiles` for that path.

**Impact:** The "linking" feature — click a tile → see its files — does not actually exist end-to-end. The table and the tree are two independent views.

**Fix direction:**
- On mount, resolve a session (same helper as C4) and fetch both tree and files.
- Include `currentPath`/`currentRootPath` in the effect dependencies and pass it as `parentPath` to `fetchScanFiles`.
- When `drillDown(path)` runs, also call `fetchScanFiles(sessionId, 1, pageSize, path, ...)`.
- Clear the table on session/drive change to avoid stale rows.

---

### C6 — macOS root scan has no `/` directory row; treemap root metrics are lost

**Where:** `src-tauri/src/services/scanner/engine.rs:131-180`, `src-tauri/src/services/visualization/mod.rs:33-57, 181-192`

On macOS, a scan of `/` is seeded with the mutable firmlink roots instead of `/` itself (`engine.rs:132-180`). That is a reasonable speed optimization, but it means **no `directories` row is ever inserted for `/`** — live check: `COUNT(*) FROM directories WHERE session_id='85db2339' AND path='/'` → **0**.

The treemap then tries:

```sql
SELECT COALESCE(total_size_bytes,0), COALESCE(file_count,0)
FROM directories WHERE session_id=?1 AND path=?2
```

and on failure falls back to `scan_sessions`, but **reads only `row.get(0)` and hardcodes `file_count = 0`** (`visualization/mod.rs:43-50`). So the root node reports 0 files.

Worse, the default threshold is **0.2% of root size** (`54-57`): on a 402 GB session that is ~804 MB. Any directory/file below 804 MB is hidden, and `build_children` collapses the remainder into a synthetic `Other items` node whose path is `.../[other]` (`181-192`) — not a real directory, so it cannot be drilled into (`TreemapChart.tsx:125` requires `isDirectory`).

**Impact:** The treemap looks nearly empty or shows one giant "Other items" tile; root file counts are wrong; users cannot navigate to most of the disk.

**Fix direction:**
- Insert a synthetic root `directories` row (`path = root_path`, parent `NULL`) at scan start/end with the correct totals.
- Fix the fallback to read both columns: `|row| Ok((row.get(0)?, row.get(1)?))`.
- Lower the default threshold or make it absolute (e.g. 1 MB) and expose it in the UI.
- Mark `Other items` as non-drillable in the UI (already effectively true) and label it clearly.

---

## 5. High Findings

### H1 — Post-scan pipeline swallows errors and marks completion too early
`engine.rs:625` sets status to completed before classification/developer/intelligence. `engine.rs:651, 655, 662` discard results with `let _ =`. A failed developer stage leaves the session "completed" with no developer rows (live: `developer_projects = 0`, `developer_ecosystem_summaries = 0`, `storage_recommendations = 0`). Users see empty tabs with no explanation.

### H2 — Single global connection mutex + blocking work in async commands
`storage/connection.rs:8-9, 56-58` exposes one `Arc<Mutex<Connection>>` for the whole app. Every command locks it. Commands are `async fn` but perform synchronous SQLite work (`commands/analysis.rs:7-15`, `commands/developer.rs:10-16`, `commands/intelligence.rs:9-17`), so long queries block the Tauri async runtime and all other commands. The 26 s developer query and 55 s analytics fallback each freeze the app.

**Fix direction:** use a connection pool (e.g. `r2d2_sqlite` or a small read pool), or at minimum `tauri::async_runtime::spawn_blocking` around long work. Consider a dedicated read-only connection for analytics.

### H3 — Classification rewrites every file row
`classifier/mod.rs:90-137` loads all files, then issues an `UPDATE files ... WHERE id = ?` per row inside one transaction. On 3.8 M rows this is a multi-minute write that grows the WAL and blocks readers. It also re-runs whenever `classification_summaries` is empty (`180-193`), including on Analysis view open.

**Fix direction:** classify in SQL where possible, or batch updates; add a `classified_at`/version marker per session to avoid recompute; never auto-classify from a read path.

### H4 — Developer analysis deletes and recomputes on every view open
`developer/mod.rs:66-72` deletes all `developer_projects` and `developer_ecosystem_summaries` for the session, then rebuilds. `DeveloperStorageView.tsx:45-48` calls it on mount and the refresh button calls it again. Combined with C1 this makes the tab unusable and destroys prior results if the rebuild is interrupted.

### H5 — Intelligence regenerates recommendations every call; duplicates never fire
`intelligence/mod.rs:14-33` clears and regenerates recommendations on every `get_storage_intelligence` call. The duplicate rule (`104-159`) reads `file_hashes`, which is **empty** in the live DB (`file_hashes = 0`), so duplicate-based recommendations never appear. Hashing only happens in the separate `find_duplicates` flow.

**Fix direction:** persist recommendations with a generation timestamp and only regenerate when the session changes; run hashing as an explicit or background stage; add a status indicator.

### H6 — Cleanup leaves aggregates stale
`cleanup/mod.rs:323-333` and `431-440` delete `files`/`directories` rows and the recommendation, but do not update:
- `scan_sessions.total_files/total_bytes`,
- parent `directories` rollups (`total_size_bytes`, `file_count`, `dir_count`),
- `classification_summaries` / `category_summaries`,
- `developer_*` tables,
- `storage_snapshots`.

After cleanup, dashboards and history still show pre-cleanup sizes.

**Fix direction:** recompute affected aggregates in the same transaction (or mark the session "stale" and recompute lazily). Never leave rollups inconsistent.

### H7 — Foreign scan events accepted; concurrent scans allowed
`scanStore.ts:367-398`: while `isScanning` is true, a progress event from a **different** session ID is accepted and overwrites `activeSessionId`/`progress`. `ScanManager` supports multiple concurrent sessions (`manager.rs:36-41`), so two scans can interleave events. The UI has no per-session routing.

**Fix direction:** key progress state by session, or reject events that don't match the active session; disable concurrent scans unless explicitly supported.

### H8 — No "finalizing" UX; progress appears stuck
Traversal ends, then the pipeline runs for minutes/hours before `scan://completed`. The UI sets `current_path = "Finalizing storage categories and summaries..."` (`engine.rs:635-646`) but there is no progress within the pipeline, no cancel check, and no timeout. A cancel during finalization does not stop the developer loop.

**Fix direction:** emit stage-level progress events (`classifying`, `analyzing_developer`, `generating_recommendations`) and check the cancel flag between stages/projects.

---

## 6. Medium Findings

- **M1 — Tree effect omits `currentPath`** (`DriveExplorerView.tsx:69-71`). Manual drive change refetches, but other path changes (breadcrumbs, reset) rely on store calls; any future local path change won't refetch. Also the initial mount point may not match the session root.
- **M2 — Drill-down table ignores the tree path** (see C5.5). `parentPath` is always `undefined` in `fetchScanFiles` calls.
- **M3 — Two category vocabularies.** `category_summaries` uses `system/code/documents/images/videos/audio/archives/executables/other` (`engine.rs:827-837`), while `classification_summaries` uses `applications/archives/audio/backups/browsers/cloud/developer/documents/gaming/hidden/images/os/temporary/unknown/videos/vm_emulators` (live). History comparisons and analytics mix incompatible taxonomies.
- **M4 — Requested session IDs are never validated.** `analysis/mod.rs:617-619`, `intelligence/mod.rs:706-708`, `visualization/mod.rs:447-450` return any non-empty string, even if the session doesn't exist or has 0 files. Errors then surface as empty views.
- **M5 — No retention/VACUUM.** DB is 18.1 GB with 14.3 M file rows across sessions. `deleteSession` (`history/mod.rs:258`) removes the session but there is no `VACUUM`/prune policy, so the file never shrinks. Old sessions (`dc705218` 4.4 M files, `a8cb1a57` 1.9 M) remain.
- **M6 — Dev CSP breaks HMR.** `tauri.conf.json:25` sets `script-src 'self'` and `connect-src 'self' ipc: http://ipc.localhost`. Vite's dev server injects an inline React refresh preamble and uses a WebSocket (`ws://localhost:1420`), both blocked. Production build is likely fine; `tauri dev` may show a blank window or no HMR.
- **M7 — Listener/timer lifecycle.** `scanStore.ts:445-449` returns unlisteners but doesn't stop module-level polling/elapsed timers on unmount. Cancel paths stop timers (`300-311`), but a component unmount mid-scan can leave intervals running.
- **M8 — Read paths trigger classification.** `classifier/mod.rs:180-193` auto-classifies when summaries are missing, from `get_storage_analysis`. Opening Analysis on an unclassified 3.8 M-file session triggers the full rewrite (H3).
- **M9 — Unescaped SQL `LIKE` wildcards.** `cleanup/mod.rs:327-331, 434-438` builds `format!("{}/%", path)` and uses `LIKE`. A path containing `%` or `_` will match unintended rows. Use `ESCAPE` or `GLOB`/prefix comparison.
- **M10 — History comparison inherits C2.** `history/mod.rs:423-440` builds deltas from `category_summaries`, which is currently a single `other` row, so comparisons report one giant `other` delta.

---

## 7. Low Findings

- **L1** — `total_pages` forced to 1 when empty (`manager.rs:325-332`, `classifier/mod.rs:392-399`); harmless but hides "0 results".
- **L2** — Synthetic `Other items` node (`visualization/mod.rs:182-191`) is not drillable; users may think data is missing.
- **L3** — Silent pipeline failures (`let _ =`) give no toast/log surfaced to the UI.
- **L4** — 20 modified files + untracked `.planning/`, `src-tauri/src/tui/`, `src-tauri/tests/` are uncommitted; risk of losing work.
- **L5** — 37.7% of bytes (341,255 files / 151.6 GB) classify as `unknown`; 929,188 files have `extension IS NULL` totaling 106.6 GB. The classifier has weak coverage for extensionless binaries/bundles.
- **L6** — `bundle.targets` is `["nsis","msi"]` (Windows only) while development is on macOS; no `.dmg`/`.app` target configured.

---

## 8. Linking Feature — End-to-End Assessment

The app advertises three linked flows. Here is the actual state of each.

### 8.1 Category card → file drill-down
**Expected:** Click a category card in Analysis → see the files in that category, paginated.

**Actual:**
- `StorageAnalysisView.tsx:480-491` calls `selectCategory(cat.category)`.
- `analysisStore.ts:42-47` sets state and immediately calls `fetchCategoryFiles(undefined, category, 1, pageSize)`.
- `fetchCategoryFiles` (`analysisStore.ts:100-111`) resolves `sid = sessionIdOverride || activeProfile?.sessionId` and invokes `get_category_files`.
- Backend `classifier/mod.rs:340-401` counts and pages `WHERE session_id=? AND category=?`.

**Works:** the click → query → paginated table path is functional, and pagination (`currentPage - 1` for Prev, `currentPage + 1` for Next, Prev disabled at page 1) is correct. I initially flagged `currentPage - 1` but confirmed it is guarded by `disabled={currentPage <= 1}` (`StorageAnalysisView.tsx:735`), so **not a bug**.

**Broken/weak:**
- Drill-down is by **top-level category only**; clicking a subcategory row is not wired to a filtered file list.
- The count query (`classifier/mod.rs:353-357`) and page query (`359-380`) both filter on `category`; there is an index `idx_files_session_cat`, so this is acceptable, but on `unknown` (341k files) the count can still be slow.
- If `activeProfile` is null (e.g. failed load) `selectCategory` no-ops silently.
- The selected card is highlighted but the tab does not automatically switch to `drilldown`; the user must click the tab (`StorageAnalysisView.tsx:460`). Minor UX gap.

### 8.2 Drive → tree → file table
**Expected:** Select a drive, see its treemap, click a directory, see that directory's files in the table.

**Actual:**
- Drive change sets `currentPath` and fetches the tree (`DriveExplorerView.tsx:251-257`).
- Tree drill-down updates `currentRootPath` and refetches the tree via the store (`visualizationStore.ts:48-51, 113-125`).
- **The file table is never fetched on mount** and **never receives `parentPath`** (`DriveExplorerView.tsx:113-119`).
- After restart `activeSessionId` is null, so `fetchScanFiles` returns immediately (`scanStore.ts:334-335`).
- The treemap backend may resolve a completed session, but the table does not share that resolution.

**Result:** The table is disconnected from the tree. This is the single biggest "linking" defect.

### 8.3 Scan completion → all views
**Expected:** After a scan, all tabs reflect the new session.

**Actual:**
- `scan://completed` is delayed by the pipeline (C1).
- On completion the frontend fetches files (`scanStore.ts:420`) and the summary, but Analysis/Intelligence/Developer each independently resolve their own session and may pick a different (newest, cancelled) one.
- `activeSessionId` is not persisted, so restart loses the link; views fall back inconsistently (visualization filters status, analysis/intelligence do not).

**Result:** "Linking" is per-view and inconsistent. A shared `SessionContext` (resolved once, persisted, validated) is needed.

---

## 9. UI / Runtime Failure Modes Observed

| Symptom | Root cause | Finding |
|---------|------------|---------|
| Scan never reaches "Completed"; UI stuck on "Finalizing…" | Developer pipeline before `scan://completed` | C1, H8 |
| App freezes / all tabs hang after a scan or on Analytics | Global DB mutex held by 26–55 s queries | C1, C3, H2 |
| Analytics category chart takes ~1 minute | `subcategory IS NULL` filter empty → full-table fallback | C3 |
| Analysis tab empty after restart | Newest session is cancelled 0-file | C4 |
| Intelligence tab empty / no recommendations | Same + `file_hashes` empty | C4, H5 |
| Explorer treemap nearly empty / one "Other items" tile | Threshold 0.2% + root row missing | C6 |
| File Registry table empty on launch | No fetch on mount + early return | C5 |
| Clicking a treemap directory does not change the table | `parentPath` always undefined | C5.5, M2 |
| History comparison shows one giant `other` delta | `category_summaries` collapsed | C2, M10 |
| Developer tab never loads | Per-project `LIKE` loop | C1, H4 |
| `tauri dev` blank/HMR broken | CSP blocks inline script + ws | M6 |
| Cleanup frees disk but dashboard sizes unchanged | Aggregates not recomputed | H6 |

---

## 10. Performance & Database Health

**Live DB snapshot (read-only):**
- Size: **18,145,939,456 bytes (18.1 GB)**; WAL 0 bytes.
- `files`: **14,366,752**; `directories`: **1,405,623**; `scan_sessions`: 7; `scan_errors`: 3,001; `classification_summaries`: 4,514; `file_hashes`: **0**.
- Latest session `7a02a2db`: `cancelled`, 0 files.
- Latest completed: `85db2339-877c-4cc2-85b8-7383fd8eb996`, root `/`, `completed_with_errors`, **3,829,960 files / 402,018,438,381 bytes**.
- Root `/` has **no** `directories` row.
- `category_summaries` for that session: **one row** `other | 3,829,960 | 402,018,438,381`.
- `classification_summaries`: 16 categories, **0 rows with `subcategory IS NULL`**.
- `developer_projects`, `developer_ecosystem_summaries`, `storage_recommendations`: **all 0**.
- `unknown`: 341,255 files / 151,640,927,038 bytes (**37.7%** of session bytes); `extension IS NULL`: 929,188 files / 106,565,378,935 bytes.

**Measured query timings:**
- Analytics fallback `GROUP BY COALESCE(category,'unknown')` over 3.8 M files: **55.59 s**.
- Equivalent `classification_summaries` query: **0.01 s**.
- Developer per-project `LIKE` query: **25.36–26.48 s** (3 runs).
- `EXPLAIN QUERY PLAN` for both long queries: `SEARCH files USING INDEX idx_files_session_mod (session_id=?)` — the wrong index, followed by a temp B-tree.

**Index inventory on `files`:** `idx_files_session_mod`, `_hidden`, `_app`, `_subcat`, `_cat`, `_path`, `_ext`, `_size`, `_parent`. A usable `(session_id, path)` index exists but the `LIKE`/`GROUP BY` forms prevent its use.

**DB growth:** 7 sessions, 14.3 M file rows, no retention policy, no `VACUUM`. Deleting a session does not shrink the file. At this rate the DB will keep growing and slow every full-table operation.

---

## 11. Recommended Remediation Plan

### Phase 1 — Stop the bleeding (Critical, highest ROI)
1. **Decouple enrichment from completion.** Emit `scan://completed` immediately after traversal + `update_session_status`; run classification/developer/intelligence in a background task with stage events and cancel checks. (C1, H1, H8)
2. **Fix `category_summaries` SQL.** Use a CTE/subquery or ordinal `GROUP BY`, then backfill existing sessions. Verify one row per real category. (C2)
3. **Fix analytics query.** Aggregate `classification_summaries` by `category` (no `subcategory IS NULL`), or write top-level rows. (C3)
4. **Unify session resolution.** One helper: prefer completed sessions with `total_files > 0`; validate requested IDs; use it in analysis, intelligence, visualization, developer, and the frontend. (C4)
5. **Wire Explorer.** Resolve session on mount, fetch tree + files, pass `parentPath`, refetch on drill-down/path change, clear stale table. (C5, M1, M2)
6. **Fix root/treemap metrics.** Insert a root `directories` row on scan; read both fallback columns; lower/expose threshold. (C6)

### Phase 2 — Performance & correctness (High)
7. Rewrite developer project attribution as one set-based query with a prefix-friendly predicate and correct index; cache per session. (C1, H4)
8. Add a read connection pool or `spawn_blocking` for long queries; stop holding the global mutex across analytics/developer/intelligence. (H2)
9. Batch classification updates; add a per-session `classified_at` marker; remove auto-classify from read paths. (H3, M8)
10. Persist intelligence recommendations and only regenerate on session change; run hashing as an explicit/background stage. (H5)
11. Recompute rollups/totals/summaries after cleanup; escape `LIKE` wildcards. (H6, M9)
12. Harden scan event routing and either disable concurrent scans or key state by session. (H7)

### Phase 3 — Data model & UX (Medium/Low)
13. Consolidate category taxonomy into one canonical enum used by classifier, summaries, and history. (M3)
14. Add retention policy + `VACUUM`/`PRAGMA incremental_vacuum`, and a "delete old sessions" flow. (M5)
15. Fix dev CSP (`script-src 'self' 'unsafe-inline'` in dev only, add `ws://localhost:1420` to `connect-src`) or use a dev-specific CSP. (M6)
16. Fix listener/timer cleanup on unmount. (M7)
17. Add a session context/store persisted across restarts. (8.3)
18. Improve classifier coverage for extensionless files/bundles; surface `unknown` percentage. (L5)
19. Add macOS bundle target if macOS distribution is intended. (L6)
20. Commit or stash the current working tree. (L4)

---

## 12. Verification Plan (after fixes)

1. **Build:** `npm run lint` and `cargo check` stay green; add `cargo test` for the new SQL.
2. **Category aggregation:** run the fixed query on `85db2339`; assert 16 distinct rows summing to 3,829,960 files / 402,018,438,381 bytes.
3. **Analytics latency:** `get_storage_analytics` for `85db2339` should be < 1 s (target ~10 ms) and must not use the `files` fallback.
4. **Session resolution:** with the live DB (newest = cancelled), Analysis/Intelligence/Explorer must resolve `85db2339`, not `7a02a2db`.
5. **Scan completion:** start a scan; `scan://completed` must fire within seconds of traversal; enrichment runs in background and emits stage events; cancel during enrichment stops it.
6. **Explorer linking:** launch fresh, open Explorer → tree and table populate; click a directory → table filters to that `parentPath`; breadcrumbs navigate back.
7. **Treemap root:** root node `file_count` equals 3,829,960; default view shows major directories, not a single "Other items" tile.
8. **Developer:** with a cached session, opening the tab returns in < 1 s; a full recompute completes in a bounded time (target < 60 s) and does not block other commands.
9. **Cleanup:** delete a recommendation; verify session totals, parent rollups, and summaries decrease consistently.
10. **History:** compare two completed sessions; category deltas must be per-category, not one `other` row.
