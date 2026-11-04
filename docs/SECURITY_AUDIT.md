# DiskAtlas: Security Architecture & Audit Report

**Audit Date:** November 2026  
**Target:** DiskAtlas Production Release v1.0.0  
**Scope:** IPC Boundaries, Filesystem Operations, Least-Privilege Capabilities, Local Privacy, and Safeguards.

---

## 1. Zero Remote Telemetry & Complete Local Privacy

### Principle
DiskAtlas operates as an **air-gapped, zero-telemetry utility**. At no point during disk scanning, duplicate discovery, database persistence, visualization, or cleanup does the application transmit file paths, file contents, drive metadata, or device identifiers across the network.

### Verification Evidence
1. **Network Requests Audit**:
   - Grep analysis across `src/` and `src-tauri/src/` confirms zero instances of `fetch()`, `axios`, `reqwest`, `hyper`, `curl`, or remote analytics trackers (Google Analytics, Sentry, Mixpanel, Segment).
2. **Strict Content Security Policy (CSP)**:
   [`src-tauri/tauri.conf.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/tauri.conf.json) explicitly defines:
   ```json
   "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: asset:; font-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost"
   ```
   All outbound network connections (`connect-src`) to remote servers are completely blocked at the WebView2 rendering engine layer.

---

## 2. Least Privilege Capability Configuration

Tauri 2 enforces a strict capability permission system. In [`src-tauri/capabilities/default.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/capabilities/default.json):
- Only `core:default` is granted to the main window.
- No general filesystem plugin (`tauri-plugin-fs`), shell execution plugin (`tauri-plugin-shell`), or arbitrary process execution APIs are exposed to the web frontend.
- All filesystem mutations are restricted to typed, validated Rust command handlers with comprehensive safeguards.

---

## 3. Destructive Action Safeguards & Operating System Defense

### Core Guardrail
Filesystem scanners operate with **strictly read-only metadata handles** (`symlink_metadata`). File deletions can only be initiated through the **Safe Cleanup Center**, which enforces four defense-in-depth barriers:

### Defense 1: Operating System & Boot Whitelist
The backend protection engine in [`safeguards.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/services/cleanup/safeguards.rs) unconditionally blocks deletion of:
- **Filesystem & Volume Roots**: `C:\`, `D:\`, `/`.
- **Boot Loaders & Kernel**: `bootmgr`, `boot.ini`, `BCD`, `EFI`, `bootmgfw.efi`, `ntoskrnl.exe`, `hal.dll`, `winload.exe`.
- **Active OS Memory / Swap**: `pagefile.sys`, `hiberfil.sys`, `swapfile.sys`, `dumpstack.log`.
- **Drivers Subsystems**: `C:\Windows\System32\drivers\`, all `.sys` files under `C:\Windows\`.
- **Windows Component Store**: `C:\Windows\WinSxS\`, `C:\Windows\servicing\`.
- **Recovery Infrastructure**: `C:\Recovery\`, `WinRE.wim`, `$Recycle.Bin` root, `System Volume Information`.
- **Unix Critical Roots**: `/System`, `/System/Library`, `/usr`, `/bin`, `/sbin`, `/etc`, `/dev`, `/proc`, `/private/var`.

Any command targeting these paths is rejected with `AppError::SecurityViolation` and recorded in the audit trail as `blocked_security`.

### Defense 2: Time-of-Check to Time-of-Use (TOCTOU) Re-validation
Paths are validated twice: once when generating the candidate plan, and again immediately before invoking deletion. If a target path no longer exists or its metadata changed to match a critical system component, execution halts immediately.

### Defense 3: Reversible Default (Windows Recycle Bin)
All deletions default to using the operating system's native Recycle Bin via `trash::delete()`. This ensures that even in the event of user error, files can be restored without data loss.

### Defense 4: Explicit Confirmation Phrase
Automated or one-click accidental deletions are impossible. The frontend dialog and backend API require the exact confirmation string `CONFIRM_CLEANUP`.

---

## 4. Cloud Storage Deletion Propagation Advisory

Files detected within known cloud synchronization roots (Microsoft OneDrive, Google Drive, Dropbox, iCloud, Nextcloud, Box) are annotated with visual warning badges. This informs the user that removing a local file will synchronize deletion across cloud storage and linked devices.

---

## 5. Audit Trail & Cryptographic Accountability

Every cleanup action is permanently recorded in the SQLite database table `cleanup_audit_events`:
- **Fields Logged**: `id`, `batch_id`, `session_id`, `path`, `name`, `size_bytes`, `category`, `risk_level`, `cleanup_method`, `status`, `is_reversible`, `error_message`, `created_at`.
- **Statuses Tracked**: `success`, `blocked_security`, `permission_denied`, `in_use`, `not_found`, `failed`.
- **User Visibility**: Inspectable at any time in the "Cleanup Audit History" tab.

---

## 6. Error Sanitization & Information Leakage Prevention

Error messages emitted across the Tauri IPC bridge are sanitized through [`src-tauri/src/error.rs`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/src/error.rs):
- Internal database paths, SQL query fragments, and memory addresses are never exposed to the frontend.
- Standardized error codes (`DATABASE_ERROR`, `IO_ERROR`, `PERMISSION_DENIED`, `VALIDATION_FAILED`, `SECURITY_VIOLATION`) are returned with human-readable, safe descriptions.

