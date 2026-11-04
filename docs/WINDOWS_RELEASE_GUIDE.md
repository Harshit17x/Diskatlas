# DiskAtlas: Windows Production Release Guide

This document details the build, packaging, code signing, and distribution procedures for **DiskAtlas** on Microsoft Windows 10 and Windows 11 (64-bit x86_64 and ARM64).

---

## 1. System Requirements & Platform Support

| Parameter | Specification | Notes |
| :--- | :--- | :--- |
| **Operating System** | Windows 10 (Version 1809+, Build 17763+) & Windows 11 (All Builds) | 64-bit architecture |
| **Web Runtime** | Microsoft Edge WebView2 Evergreen Runtime | Pre-installed on Windows 10 20H2+ and all Windows 11 |
| **Privileges** | Standard User (Non-Elevated) | `installMode: "currentUser"` eliminates UAC prompts |
| **Filesystems** | NTFS, ReFS, exFAT, FAT32 | Full metadata & allocation size support |
| **Memory** | 256 MB RAM minimum, 512 MB recommended | Highly bounded memory usage |

---

## 2. Prerequisites for Building on Windows

To build native Windows release packages on Windows:

1. **Rust Toolchain**:
   ```powershell
   rustup default stable-x86_64-pc-windows-msvc
   ```
2. **Visual Studio C++ Build Tools**:
   - Install "Desktop development with C++" workload from Visual Studio Installer.
3. **Node.js**:
   - Node.js LTS (v20+ recommended) with `npm` or `pnpm`.
4. **NSIS (Nullsoft Scriptable Install System)**:
   - Tauri automatically provisions NSIS during `npm run tauri build` if missing.
5. **WiX Toolset (for MSI builds)**:
   - WiX v3.11+ (accessible on `PATH`).

---

## 3. Production Build Instructions

### Step 1: Install Dependencies
```powershell
npm install
```

### Step 2: Compile Production Binaries
Execute the Tauri CLI build workflow:
```powershell
# Clean build for production
npm run build
npm run tauri build
```

This compiles:
- Optimized Rust backend with Link-Time Optimization (`lto = true`), single codegen unit (`codegen-units = 1`), and binary symbol stripping (`strip = true`).
- Minified React 19 frontend bundle into `dist/`.
- Windows Installer binaries in `src-tauri/target/release/bundle/`:
  - **NSIS Installer**: `src-tauri/target/release/bundle/nsis/DiskAtlas_1.0.0_x64-setup.exe`
  - **MSI Package**: `src-tauri/target/release/bundle/msi/DiskAtlas_1.0.0_x64_en-US.msi`

---

## 4. Code Signing & Authenticode Certification

To prevent Microsoft Defender SmartScreen warnings ("Windows protected your PC") and establish software reputation, all binaries and installers must be signed with an EV (Extended Validation) or Standard Code Signing Certificate.

### Using SignTool (Windows SDK)

```powershell
# Set environment variables
$CERT_PATH = "C:\Certificates\DiskAtlas_CodeSigning.pfx"
$TIMESTAMP_URL = "http://timestamp.digicert.com"

# 1. Sign application executable
signtool sign /f $CERT_PATH /p $CERT_PASSWORD /tr $TIMESTAMP_URL /td sha256 /fd sha256 "src-tauri\target\release\diskatlas.exe"

# 2. Sign NSIS setup installer
signtool sign /f $CERT_PATH /p $CERT_PASSWORD /tr $TIMESTAMP_URL /td sha256 /fd sha256 "src-tauri\target\release\bundle\nsis\DiskAtlas_1.0.0_x64-setup.exe"

# 3. Verify digital signature
signtool verify /pa /v "src-tauri\target\release\bundle\nsis\DiskAtlas_1.0.0_x64-setup.exe"
```

### Automated CI/CD Signing (GitHub Actions)
Configure the following secrets in your repository:
- `WINDOWS_CERTIFICATE_BASE64`: Base64-encoded `.pfx` certificate.
- `WINDOWS_CERTIFICATE_PASSWORD`: Certificate password.

---

## 5. Installer Configuration & Features

DiskAtlas uses a customized NSIS package configured in [`src-tauri/tauri.conf.json`](file:///Users/harshitbisht/Desktop/DiskAtlas/src-tauri/tauri.conf.json):

1. **Per-User Installation (`installMode: "currentUser"`)**:
   - Installs to `%LOCALAPPDATA%\Programs\DiskAtlas`.
   - Does **not** require administrative UAC prompt.
   - Allows seamless non-privileged updates.
2. **Database Storage Location**:
   - Database and application settings persist in `%APPDATA%\com.diskatlas.app\diskatlas.db`.
   - Preserved across application upgrades and re-installations.
3. **Clean Uninstallation**:
   - Automatically unregisters Start Menu shortcuts and cleans up program binaries.

---

## 6. Windows Release Verification Checklist

- [x] High-DPI display scaling verified on 100%, 125%, 150%, and 200% displays.
- [x] Application launches without requiring administrator elevation.
- [x] System tray and window frame styling matches Windows 11 Mica/Acrylic aesthetic guidelines.
- [x] File attributes (`FILE_ATTRIBUTE_HIDDEN`, `FILE_ATTRIBUTE_SYSTEM`) correctly resolved.
- [x] NTFS directory junctions (`IO_REPARSE_TAG_MOUNT_POINT`) and cyclic reparse points skipped without recursive loop lockup.
- [x] Filesystem paths exceeding 260 characters (`MAX_PATH`) traversed successfully.
- [x] Moving items to Windows Recycle Bin verified via native COM shell interface.
- [x] SQLite WAL database verified crash-safe under unexpected termination.
