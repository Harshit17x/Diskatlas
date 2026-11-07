# Phase 1: Security Safeguards & Integrity — Research & Strategy

**Phase:** Phase 1 (Security Safeguards & Integrity)  
**Target Milestone:** Milestone 1 (v1.0.1)  
**Status:** Complete  

---

## 1. Objectives & Scope

Phase 1 addresses critical security, integrity, and safety findings identified during the codebase audit:
1. **SQL Query Parameterization (`REQ-SEC-01`)**:
   - Eliminate raw SQL string interpolation in [`src-tauri/src/services/cleanup/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs) and [`src-tauri/src/services/analysis/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/analysis/mod.rs).
   - Use Rusqlite parameterized queries (`params![]` or `?1, ?2`).
2. **Path Canonicalization in Security Safeguards (`REQ-SEC-02`)**:
   - In [`src-tauri/src/services/cleanup/safeguards.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/safeguards.rs), enforce path canonicalization before prefix checking to eliminate directory traversal evasion (`..`).
   - Reject paths containing unresolved `Component::ParentDir`.
3. **Symlink / Junction Deletion Unlinking (`REQ-SEC-03`)**:
   - In [`src-tauri/src/services/cleanup/mod.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/mod.rs), ensure permanent cleanup checks `symlink_metadata()`. If an item is a symlink or junction, invoke `std::fs::remove_file()` (or platform unlinking) rather than `std::fs::remove_dir_all()`.
4. **WebView Content Security Policy (`REQ-SEC-04`)**:
   - Align [`src-tauri/tauri.conf.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/tauri.conf.json) with [`docs/SECURITY_AUDIT.md`](file:///Users/harshitbisht/Desktop/DiskAtlas/docs/SECURITY_AUDIT.md) to enforce strict local CSP (`connect-src 'self' ipc: http://ipc.localhost`).
5. **Drive Scanner Initialization & Run Script Fix**:
   - Ensure `openScanModal` resets completion state when starting a new scan on selected drives.
   - Add `"drive"` and `"tauri:dev"` scripts to `package.json` to allow `npm run drive`.

---

## 2. Technical Evidence & Vulnerability Details

### Vulnerability 1: SQL String Interpolation
- **Cleanup Audit Filter (`src-tauri/src/services/cleanup/mod.rs:524-555`)**:
  ```rust
  // CURRENT VULNERABLE PATTERN:
  if let Some(ref sid) = query.session_id {
      count_sql.push_str(&format!(" AND session_id = '{}'", sid));
  }
  if let Some(ref st) = query.status {
      count_sql.push_str(&format!(" AND status = '{}'", st));
  }
  ```
  *Remediation*: Dynamically construct parameter arrays (`Vec<Box<dyn ToSql>>`) and query Rusqlite using parameterized bindings (`AND session_id = ?`, `AND status = ?`).
- **Duplicate Search Filter (`src-tauri/src/services/analysis/mod.rs:254-265`)**:
  ```rust
  // CURRENT VULNERABLE PATTERN:
  if let Some(ref ext) = query.extension {
      stage1_sql.push_str(&format!(" AND LOWER(extension) = '{}'", ext.to_lowercase()));
  }
  ```
  *Remediation*: Bind extension via Rusqlite query parameter `?3`.

### Vulnerability 2: Safeguards Traversal Bypass
- In `safeguards.rs`:
  Checks like `path.starts_with("/System")` fail when given `/Users/user/../../System` or non-canonical symlinks.
  *Remediation*:
  Resolve paths using `std::fs::canonicalize(path)`. If canonicalization fails (e.g. file does not exist yet), check components:
  ```rust
  if path.components().any(|c| c == std::path::Component::ParentDir) {
      return Err(AppError::SecurityViolation("Path traversal attempt detected".into()));
  }
  ```

### Vulnerability 3: Symlink / Junction Recursive Deletion
- In `src-tauri/src/services/cleanup/mod.rs:376-380`:
  ```rust
  if path_obj.is_dir() {
      fs::remove_dir_all(path_obj)?;
  }
  ```
  `is_dir()` follows symlinks! If `path_obj` is a symlink pointing to a directory, `remove_dir_all` deletes the contents of the target directory!
  *Remediation*:
  ```rust
  let is_symlink = fs::symlink_metadata(path_obj)
      .map(|m| m.file_type().is_symlink())
      .unwrap_or(false);
  if is_symlink {
      fs::remove_file(path_obj)?;
  } else if path_obj.is_dir() {
      fs::remove_dir_all(path_obj)?;
  } else {
      fs::remove_file(path_obj)?;
  }
  ```

### Vulnerability 4: Tauri CSP Misconfiguration
- In `src-tauri/tauri.conf.json`:
  ```json
  "security": {
    "csp": null,
    "capabilities": ["default"]
  }
  ```
  *Remediation*: Set:
  ```json
  "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: asset:; font-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost"
  ```

---

## 3. Plan Wave Ordering

- **Wave 1 (Tracer / Core Fixes)**:
  - `01-01-PLAN.md`: SQL query parameterization in `cleanup/mod.rs` & `analysis/mod.rs`.
  - `01-02-PLAN.md`: Path canonicalization & traversal safeguards in `safeguards.rs`.
  - `01-03-PLAN.md`: Symlink & junction deletion fix in `cleanup/mod.rs`.
- **Wave 2 (Configuration & Verification)**:
  - `01-04-PLAN.md`: WebView CSP configuration in `tauri.conf.json`.
  - `01-05-PLAN.md`: Drive Scanner target initialization and `npm run drive` validation.

