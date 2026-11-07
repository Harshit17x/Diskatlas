# Coding Conventions

**Analysis Date:** 2026-11-06

## Naming Patterns

### Files
- **React Components:** Use `PascalCase.tsx` matching the primary exported component name.
  - Examples: `src/components/common/Button.tsx`, `src/features/dashboard/DashboardView.tsx`, `src/components/layout/Header.tsx`, `src/components/visualizations/TreemapChart.tsx`.
- **Zustand State Stores:** Use `camelCaseStore.ts` under `src/stores/`.
  - Examples: `src/stores/driveStore.ts`, `src/stores/scanStore.ts`, `src/stores/cleanupStore.ts`, `src/stores/uiStore.ts`.
- **Frontend Utilities & Helpers:** Use `camelCase.ts` under `src/utils/`.
  - Examples: `src/utils/formatters.ts`, `src/utils/tauri.ts`.
- **TypeScript Type Definitions:** Use `camelCase.ts` under `src/types/`.
  - Examples: `src/types/bindings.ts`, `src/types/ui.ts`.
- **Rust Backend Modules:** Use `snake_case.rs` throughout `src-tauri/src/`.
  - Examples: `src-tauri/src/services/drive_detector.rs`, `src-tauri/src/services/system_inspector.rs`, `src-tauri/src/services/cleanup/safeguards.rs`, `src-tauri/src/error.rs`.
- **Database Migrations:** Use sequential zero-padded numeric prefixes with snake_case descriptions: `000N_description.sql` under `src-tauri/migrations/`.
  - Examples: `src-tauri/migrations/0001_initial_schema.sql`, `src-tauri/migrations/0008_cleanup_audit_history.sql`.

### Functions
- **TypeScript Functions & Hooks:** Use `camelCase`.
  - Examples: `formatBytes(bytes: number)`, `formatPercent(percent: number)`, `safeInvoke<T>(command, args)`, `useDriveStore()`, `toggleCandidateSelection(id: string)`.
- **React Functional Components:** Use `PascalCase` named exports.
  - Example: `export const Button: React.FC<ButtonProps> = ({ ... }) => { ... };` in `src/components/common/Button.tsx`.
- **Rust Functions & Methods:** Use `snake_case`.
  - Examples: `get_drives()`, `detect_all()`, `sync_to_db()`, `is_system_critical_path()`.
- **Tauri Command Handlers:** Use `snake_case` annotated with `#[tauri::command]` or `#[command]`.
  - Examples: `pub async fn get_cleanup_plan(...)`, `pub async fn execute_cleanup(...)` in `src-tauri/src/commands/cleanup.rs`.

### Variables
- **TypeScript Variables:** Use `camelCase` for variables, properties, and store state.
  - Examples: `selectedDriveId`, `isLoading`, `cleanupPlan`, `confirmationModalOpen`.
- **TypeScript Constants:** Use `SCREAMING_SNAKE_CASE` for file-level or lookup constants.
  - Example: `const CATEGORY_COLORS: Record<string, string> = { ... };` in `src/components/visualizations/TreemapChart.tsx`.
- **Rust Variables & Fields:** Use `snake_case` for local variables and struct fields.
  - Examples: `session_id`, `files_scanned`, `size_bytes`, `is_removable`.
- **Rust Constants:** Use `SCREAMING_SNAKE_CASE`.
  - Example: `pub const STATUS_RUNNING: u8 = 1;` in `src-tauri/src/services/scanner/engine.rs`.

### Types
- **TypeScript Interfaces & Types:** Use `PascalCase`. Do not prefix with `I`.
  - Examples: `interface ButtonProps`, `interface DriveInfo`, `type DriveType = ...`, `interface CleanupPlan`.
- **Rust Structs & Enums:** Use `PascalCase`.
  - Examples: `pub enum AppError`, `pub struct DriveInfo`, `pub enum ScanStatus`.
- **Cross-Boundary Serde Serialization:** Always apply `#[serde(rename_all = "camelCase")]` on Rust models so struct fields automatically serialize into camelCase JSON properties matching TypeScript interfaces in `src/types/bindings.ts`.
  - Example from `src-tauri/src/models/drive.rs`:
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DriveInfo {
        pub drive_letter: Option<String>,
        pub mount_point: String,
        pub is_removable: bool,
    }
    ```

---

## Code Style

### Formatting
- **Rust:** Formatted with `cargo fmt` adhering to standard Rust 2021 edition formatting rules (4-space indentation, trailing commas in multi-line struct instantiations and pattern matches).
- **TypeScript / React:** 2-space indentation, single quotes for strings in TypeScript, double quotes in JSX attributes, semicolons enabled, trailing commas on multi-line objects.
- **Styling Architecture:** Tailwind CSS utility classes merged via `clsx` and `tailwind-merge` (`twMerge(clsx(...))`) to prevent class collisions. Custom design system tokens (`canvas`, `panel`, `elevated`, `border`, `accent`, `danger`) configured in `tailwind.config.js`.
  - Example from `src/components/common/Button.tsx`:
    ```typescript
    import { clsx } from 'clsx';
    import { twMerge } from 'tailwind-merge';

    export const Button: React.FC<ButtonProps> = ({ className, variant = 'secondary', size = 'md', ...props }) => {
      return (
        <button
          className={twMerge(clsx(baseStyles, variantStyles[variant], sizeStyles[size], className))}
          {...props}
        />
      );
    };
    ```

### Linting
- **TypeScript:** Enforced by TypeScript compiler in strict mode using `npm run lint` (`tsc -b --noEmit`) and `npm run build`.
  - Configured in `tsconfig.app.json` with strict compiler flags:
    - `"strict": true`
    - `"noUnusedLocals": true`
    - `"noUnusedParameters": true`
    - `"noFallthroughCasesInSwitch": true`
    - `"noUncheckedSideEffectImports": true`
- **Rust:** Linted via `cargo clippy`. No unhandled compiler warnings are allowed during build.
- **Notice on Config Files:** Dedicated `.eslintrc` / `eslint.config.js` and `.prettierrc` files are not currently present in the project root; linting is strictly enforced by `tsc` type-checking and compiler verification.

---

## Import Organization

### TypeScript / React
Imports follow a top-down order from external foundations to internal primitives:
1. **React Core:** `react`, `react-dom`
2. **Third-Party Libraries:** `lucide-react`, `d3`, `clsx`, `tailwind-merge`, `zustand`, `@tauri-apps/api/core`
3. **Type Imports:** `import type { ... } from '...'` (explicit type imports preferred)
4. **Zustand State Stores:** `import { useDriveStore } from '../stores/driveStore'`
5. **Internal Components:** Layout components, feature components, common UI primitives (`Button`, `Card`)
6. **Local Utilities & Formatters:** `import { formatBytes } from '../utils/formatters'`

Example from `src/components/visualizations/TreemapChart.tsx`:
```typescript
import React, { useEffect, useRef, useState } from 'react';
import * as d3 from 'd3';
import type { HierarchicalNode } from '../../types/bindings';
import { formatBytes } from '../../utils/formatters';
import { Button } from '../common/Button';
import { RotateCcw, ArrowUp, ChevronRight, FolderTree } from 'lucide-react';
```

### Rust
Imports in `src-tauri/src/` follow standard Rust crate grouping:
1. **Standard Library:** `std::path::PathBuf`, `std::sync::Arc`, `std::fs`
2. **External Crates:** `serde`, `tauri`, `thiserror`, `tokio`, `tracing`, `rusqlite`
3. **Internal Crate Modules:** `crate::error::Result`, `crate::models::*`, `crate::services::*`, `crate::storage::*`

Example from `src-tauri/src/services/cleanup/mod.rs`:
```rust
use std::fs;
use std::path::Path;
use rusqlite::{params, Connection};
use tracing::{error, info, warn};

use crate::error::{AppError, Result};
use crate::models::cleanup::{CleanupCandidate, CleanupPlan};
use crate::services::cleanup::safeguards::is_system_critical_path;
```

### Path Aliases
- Path aliases (e.g. `@/*`) are **not configured** in `tsconfig.json` or `vite.config.ts`.
- All frontend imports use explicit relative paths (`../`, `../../`).

---

## Error Handling

### Rust Backend
All backend operations route through a centralized error enum defined in `src-tauri/src/error.rs`:
- Uses `thiserror::Error` for deriving error messages and source error wrapping.
- Implements custom `serde::Serialize` converting errors into a standardized JSON shape `{ code: string, message: string, details?: string }`.
- Defines universal type alias: `pub type Result<T> = std::result::Result<T, AppError>;`.

```rust
// src-tauri/src/error.rs
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Filesystem I/O error on '{path}': {source}")]
    Io { path: String, #[source] source: std::io::Error },

    #[error("Drive not found: {0}")]
    DriveNotFound(String),

    #[error("Permission denied for path: {0}")]
    PermissionDenied(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),
}
```

### Frontend IPC Communication
Tauri commands are invoked via a type-safe wrapper in `src/utils/tauri.ts` that catches exceptions and guarantees an `AppError` contract:

```typescript
// src/utils/tauri.ts
export async function safeInvoke<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<{ ok: true; data: T } | { ok: false; error: AppError }> {
  try {
    const data = await invoke<T>(command, args);
    return { ok: true, data };
  } catch (err: unknown) {
    if (typeof err === 'object' && err !== null && 'code' in err && 'message' in err) {
      return { ok: false, error: err as AppError };
    }
    return {
      ok: false,
      error: {
        code: 'IPC_ERROR',
        message: typeof err === 'string' ? err : 'Unknown error occurred while calling backend',
      },
    };
  }
}
```

### UI Error Presentation
- Zustand stores maintain `error: string | null` state.
- Stores surface backend failure messages to users through dedicated alert banners (`src/components/cleanup/CleanupResultsBanner.tsx`), modals (`src/components/cleanup/CleanupConfirmationModal.tsx`), or disabled state triggers.
- For high-consequence operations (such as disk cleanup), destructive actions are guarded by strict phrase validation (`"PERMANENTLY DELETE"` or `"CLEANUP DISK"`) and security safeguard checks before execution.

---

## Logging

### Backend Logging (Rust)
- Managed via `tracing` and `tracing-subscriber`.
- Initialized in `src-tauri/src/lib.rs` with `EnvFilter`:
  ```rust
  tracing_subscriber::registry()
      .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,diskatlas=debug".into()))
      .with(tracing_subscriber::fmt::layer())
      .init();
  ```
- Use structured logging macros: `tracing::info!`, `tracing::warn!`, `tracing::error!`.
- Always log operational context (paths, session IDs, durations, error strings).

### Frontend Logging
- Restrict to `console.error` inside store catch blocks for unexpected IPC failures during development.
- **Privacy Policy:** Per `CONTRIBUTING.md`, no remote analytics, external tracking, or telemetry SDKs may ever be added.

---

## Comments

### When to Comment
- **Filesystem Safety Rules:** Detail why specific paths or reparse tags are excluded (e.g. symlink recursion loops, Windows junction points, critical boot files).
- **Complex Aggregations:** Explain hierarchical tree bounding, D3 rollups, or bitwise disk flag manipulations.
- **Step-by-Step Orchestration:** Use numbered comment blocks for multi-step processes (e.g. `// 1. Initialize DB`, `// 2. Scan candidates`, `// 3. Apply audit record`).

### JSDoc / TSDoc
- Exported utility functions in `src/utils/formatters.ts` and `src/utils/tauri.ts` must have complete JSDoc describing parameters, unit conversions (e.g., base-1000 decimal byte standard vs base-1024 binary), and return values.

---

## Function Design

### Component & Hook Design
- Functional React components using `React.FC<Props>` or standard typed function declarations.
- Keep components focused on UI rendering; delegate business logic, caching, and Tauri communication to Zustand stores.
- Explicit TypeScript interfaces for all props, extending standard HTML attributes where appropriate (`interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement>`).

### Backend Command Design
- Tauri commands in `src-tauri/src/commands/` act as thin controller endpoints:
  - Inject managed state via `State<'_, DbPool>` or `State<'_, Arc<ScanManager>>`.
  - Validate parameters.
  - Delegate long-running or blocking database/filesystem tasks to worker threads via `tokio::task::spawn_blocking`.
  - Return `Result<T>` directly.

---

## Module Design

### Directory Structure & Responsibilities
- `src/features/<feature>/`: Feature-specific views (e.g., `DashboardView.tsx`, `CleanupCenterView.tsx`, `DeveloperStorageView.tsx`).
- `src/components/common/`: Reusable primitive UI widgets (`Button.tsx`, `Card.tsx`, `DataTable.tsx`, `Modal.tsx`, `Badge.tsx`, `Tabs.tsx`).
- `src/components/visualizations/`: D3-based analytical charts (`TreemapChart.tsx`, `SunburstChart.tsx`, `StorageAnalyticsDashboard.tsx`).
- `src/stores/`: Domain-separated Zustand state management modules.
- `src-tauri/src/services/`: Pure business logic, filesystem walkers, platform inspectors, and disk scanners.
- `src-tauri/src/commands/`: Tauri IPC command endpoints.
- `src-tauri/src/models/`: Shared Rust data structures with Serde serialization.
- `src-tauri/src/storage/`: SQLite database pooling and migration engine.

### Barrel Files
- Rust uses `mod.rs` barrel exports to expose clean module interfaces (e.g. `src-tauri/src/commands/mod.rs`, `src-tauri/src/services/mod.rs`).
- Frontend avoids barrel files (`index.ts`) in favor of direct file imports (`import { Button } from '../common/Button'`).

---

## Git & Contribution Conventions

### Commit Messages
Follow Conventional Commits specification enforced per `CONTRIBUTING.md`:
- Format: `<type>(<scope>): <description>`
- Examples:
  - `feat(developer): add detection for Elixir _build directory`
  - `fix(scanner): prevent traversal into pseudo device nodes`
  - `docs: update Windows release guide`

### Branch Naming
- Features: `feat/<feature-name>`
- Bug Fixes: `fix/<bug-description>`
- Documentation: `docs/<topic-name>`

### Pull Request Quality Gate
All pull requests must pass the checklist defined in `.github/pull_request_template.md`:
1. Full backend test suite passing: `cargo test --manifest-path src-tauri/Cargo.toml`
2. Frontend build and type checks clean: `npm run build` and `npm run lint`
3. Scans remain read-only and non-destructive
4. Zero remote telemetry or network tracking calls

---

*Convention analysis: 2026-11-06*
