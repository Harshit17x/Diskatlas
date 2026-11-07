# Testing Patterns

**Analysis Date:** 2026-11-06

## Test Framework

### Runner
- **Backend (Rust):** Built-in Rust test harness (`cargo test`) with `tokio` multi-threaded test runtime (`#[tokio::test]`).
  - Config: `src-tauri/Cargo.toml`
  - Edition: Rust 2021
- **Frontend (TypeScript/React):** **Not configured.** No testing runner (Vitest, Jest, React Testing Library, Playwright, or Cypress) is currently installed in `package.json`. Frontend quality verification currently relies entirely on the TypeScript compiler type-checking (`tsc -b --noEmit`) and Vite production bundle compilation (`vite build`).

### Assertion Library
- **Rust:** Standard library assertion macros (`assert!`, `assert_eq!`, `assert_ne!`).

### Run Commands
```bash
# Run all backend unit and integration tests (26 tests)
cargo test --manifest-path src-tauri/Cargo.toml

# Run a specific test suite or test name
cargo test --manifest-path src-tauri/Cargo.toml test_cleanup_plan_and_execution_with_synthetic_files

# Run tests with stdout output enabled
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture

# Frontend type check & lint validation
npm run lint

# Frontend production compilation check
npm run build
```

---

## Test File Organization

### Location
- **Backend Tests:** Co-located inside the corresponding source files using `#[cfg(test)] mod tests { ... }` blocks at the end of each module.
- **Frontend Tests:** Currently none present in `src/`.

### Naming
- In-source test modules: `mod tests`.
- Test functions: prefixed with `test_` describing the scenario and expected outcome (e.g. `test_system_critical_path_detection`, `test_staged_duplicate_detection_groups_matching_hashes`).

### Structure
Tests are organized across 13 core service and storage modules in `src-tauri/src/`:
```
src-tauri/src/
├── storage/
│   └── migrator.rs                 # Migration integrity & idempotence tests
├── commands/
│   └── database.rs                 # Health check & connection pooling tests
└── services/
    ├── drive_detector.rs           # Drive enumeration & system drive detection
    ├── system_inspector.rs         # OS metadata extraction tests
    ├── scanner/
    │   └── mod.rs                  # Synthetic filesystem crawler & live progress tests
    ├── classifier/
    │   └── mod.rs                  # Classification determinism & override precedence
    ├── analysis/
    │   └── mod.rs                  # Duplicate detection, forgotten files, hidden files
    ├── visualization/
    │   └── mod.rs                  # Hierarchical tree depth bounding & analytics aggregation
    ├── developer/
    │   └── mod.rs                  # Project detection, CLI output parsing (Docker/Git)
    ├── history/
    │   └── mod.rs                  # Scan diffing, growth analysis & recovery
    ├── intelligence/
    │   └── mod.rs                  # Recommendation engine & profile generation
    └── cleanup/
        ├── safeguards.rs           # OS path protection & cloud sync warning tests
        └── mod.rs                  # Synthetic file deletion & audit record verification
```

---

## Test Structure

### Suite Organization
Backend tests are isolated within `#[cfg(test)] mod tests` in their respective modules.

Example from `src-tauri/src/services/cleanup/safeguards.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_system_critical_path_detection() {
        // Windows system directories and drivers
        assert!(is_system_critical_path(Path::new("C:\\Windows")).0);
        assert!(is_system_critical_path(Path::new("C:\\Windows\\System32")).0);
        assert!(is_system_critical_path(Path::new("C:\\Windows\\System32\\drivers\\ntfs.sys")).0);
        assert!(is_system_critical_path(Path::new("C:\\bootmgr")).0);
        assert!(is_system_critical_path(Path::new("C:\\pagefile.sys")).0);
        assert!(is_system_critical_path(Path::new("C:\\System Volume Information")).0);
        assert!(is_system_critical_path(Path::new("C:\\Windows\\WinSxS\\manifest.xml")).0);
        assert!(is_system_critical_path(Path::new("C:\\")).0);

        // Unix / macOS system paths
        assert!(is_system_critical_path(Path::new("/System")).0);
        assert!(is_system_critical_path(Path::new("/System/Library/Extensions/AppleKext.kext")).0);
        assert!(is_system_critical_path(Path::new("/usr/bin")).0);
        assert!(is_system_critical_path(Path::new("/etc/hosts")).0);
        assert!(is_system_critical_path(Path::new("/")).0);

        // Safe user paths (must NOT be marked critical)
        assert!(!is_system_critical_path(Path::new("C:\\Users\\User\\Downloads\\old_installer.exe")).0);
        assert!(!is_system_critical_path(Path::new("C:\\Projects\\node_modules")).0);
        assert!(!is_system_critical_path(Path::new("/Users/user/Desktop/test_project/target")).0);
    }
}
```

### Patterns

#### Setup Pattern (In-Memory Database & Synthetic Filesystem)
Tests initialize an ephemeral in-memory SQLite database and apply real schema migrations using `Migrator::run(&mut conn)` to ensure schema parity:
```rust
let mut conn = Connection::open_in_memory().unwrap();
Migrator::run(&mut conn).unwrap();

// Create isolated synthetic file structure in system temp directory
let temp_dir = std::env::temp_dir().join(format!("diskatlas_clean_test_{}", Uuid::new_v4()));
std::fs::create_dir_all(&temp_dir).unwrap();
```

#### Teardown Pattern
- In-memory databases are allocated on stack/local variables and automatically dropped when test scope exits.
- Synthetic temporary directories use randomized UUID paths to prevent collision across parallel tests.

#### Assertion Pattern
- Strict boolean assertions (`assert!(condition)`) for safeguard policies.
- Equality assertions (`assert_eq!(result.status, "completed")`) for state transitions and record counts.
- Error matching asserting specific `AppError` variants (`match res { Err(AppError::ValidationFailed(_)) => ... }`).

---

## Mocking

### Framework
- No external mocking framework (such as `mockall` or `sinon`) is used.
- Tests utilize **stateful test doubles**:
  1. Real in-memory SQLite instances (`Connection::open_in_memory()`) instead of mocking SQL queries.
  2. Synthetic filesystem directory hierarchies created in the OS temporary directory (`std::env::temp_dir()`).
  3. Raw terminal string simulation for external CLI parsers (e.g. testing `parse_docker_df_output` against synthetic Docker output strings).

### What to Mock
- External CLI command output strings (Docker CLI, package manager outputs) to verify parsing logic without requiring Docker or third-party tools to be installed on test machines.
- Drive metadata inputs to test OS-specific edge cases (e.g., simulated unformatted drives or null volume capacities).

### What NOT to Mock
- Database schema and migrations: Always execute tests against actual SQLite connections with all migrations applied.
- Path classification and security guards: Always validate against real file path strings and actual filesystem permissions.

---

## Fixtures and Factories

### Test Data
Fixtures are created inline within test helper blocks or setup routines:

Example from `src-tauri/src/services/scanner/mod.rs`:
```rust
// 1. Setup temporary database with migrations
let test_run_id = Uuid::new_v4().to_string();
let temp_base = std::env::temp_dir().join(format!("diskatlas_test_{}", test_run_id));
fs::create_dir_all(&temp_base).unwrap();

let db_path = temp_base.join("test_diskatlas.db");
let db_pool = DbPool::new(&db_path).unwrap();

// 2. Create synthetic test directory hierarchy
let root = temp_base.join("synthetic_root");
fs::create_dir_all(&root).unwrap();

// 2.1 Nested folders
let nested_dir = root.join("level1").join("level2");
fs::create_dir_all(&nested_dir).unwrap();
let mut f1 = File::create(nested_dir.join("sample.txt")).unwrap();
f1.write_all(b"Hello from nested file!").unwrap();

// 2.2 Hidden files
let hidden_file = root.join(".hidden_config");
let mut f2 = File::create(&hidden_file).unwrap();
f2.write_all(b"secret=true").unwrap();
```

### Location
- Fixtures are generated dynamically within each test using standard temporary directories (`std::env::temp_dir()`). There are no static asset fixtures committed to the repository.

---

## Coverage

### Current Coverage Status
- **Backend (Rust):** High test coverage across core business logic. **26 unit and integration tests** currently execute and pass with 0 failures:
  - Migration execution & idempotence (2 tests)
  - Database health & deadlock avoidance (1 test)
  - Drive detector & system drive validation (3 tests)
  - System inspector OS metadata extraction (1 test)
  - Filesystem scanner traversal & progress events (2 tests)
  - Storage classifier & user override precedence (2 tests)
  - Analysis features: duplicates, forgotten files, hidden files, large files (4 tests)
  - Visualization tree aggregation & depth bounding (2 tests)
  - Developer storage detection & Docker parsing (3 tests)
  - Storage history snapshot comparison & recovery (2 tests)
  - Storage intelligence profiling & recommendations (1 test)
  - Cleanup safeguards & cloud sync detection (2 tests)
  - Cleanup execution with synthetic files & audit logging (1 test)
- **Frontend (TypeScript/React):** **0% automated test coverage.** No unit or component tests exist for React views, D3 chart rendering, or Zustand store actions.

### View Coverage
Code coverage instrumentation (e.g. `cargo-tarpaulin` or `llvm-cov`) is not currently configured in CI or `Cargo.toml`. To view test results:
```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## Test Types

### Unit Tests
- Fast, isolated function tests verifying:
  - System-critical path protection (`services::cleanup::safeguards::tests::test_system_critical_path_detection`)
  - Cloud storage synchronization detection (`services::cleanup::safeguards::tests::test_cloud_sync_detection`)
  - Size parsing and format strings (`services::developer::tests::test_parse_size_bytes`)
  - Output parsers (`services::developer::tests::test_parse_docker_df_output`)
  - Database migration idempotence (`storage::migrator::tests::test_migrations_are_idempotent`)

### Integration Tests
- End-to-end subsystem tests that combine database, filesystem, and business services:
  - Complete synthetic filesystem traversal and database indexing (`services::scanner::tests::test_synthetic_filesystem_scanner`)
  - Multi-stage duplicate file grouping and SHA-256 hash validation (`services::analysis::tests::test_staged_duplicate_detection_groups_matching_hashes`)
  - Cleanup plan creation, critical path rejection, and deletion execution (`services::cleanup::tests::test_cleanup_plan_and_execution_with_synthetic_files`)
  - Scan history snapshot comparison and growth differential tracking (`services::history::tests::test_compare_scans_differential_growth_and_file_deltas`)

### E2E / UI Tests
- **Not implemented.** No Playwright, Tauri WebDriver, or Cypress tests are present for verifying the compiled desktop GUI or frontend interactions.

---

## Common Patterns

### Async Testing
Tokio async tests are decorated with `#[tokio::test]`:
```rust
#[tokio::test]
async fn test_scan_manager_live_progress() {
    let test_run_id = Uuid::new_v4().to_string();
    let temp_base = std::env::temp_dir().join(format!("diskatlas_pm_test_{}", test_run_id));
    fs::create_dir_all(&temp_base).unwrap();

    let db_path = temp_base.join("test_pm.db");
    let db_pool = DbPool::new(&db_path).unwrap();

    // Run async scanner manager
    let manager = ScanManager::new(db_pool.clone());
    // ... test progress events and cancellation ...
}
```

### Error & Security Testing
Testing that operations fail safely when encountering forbidden inputs or validation errors:
```rust
// Verify that invalid confirmation phrase aborts cleanup
let invalid_phrase_req = ExecuteCleanupRequest {
    batchId: plan.batch_id.clone(),
    sessionId: session_id.to_string(),
    itemIds: vec![rec_id.to_string()],
    useRecycleBin: true,
    confirmationPhrase: "wrong phrase".to_string(),
};
let res = CleanupService::execute_cleanup(&conn, invalid_phrase_req);
assert!(res.is_err());
match res {
    Err(AppError::ValidationFailed(msg)) => {
        assert!(msg.contains("Confirmation phrase does not match"));
    }
    _ => panic!("Expected ValidationFailed error"),
}
```

---

## CI/CD & Automated Testing Gaps

1. **Missing CI Pipeline:**
   - The `.github/workflows/` directory does not exist. Tests currently run only on developer workstations before pull requests are opened.
   - Recommended remedy: Create `.github/workflows/ci.yml` running `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo clippy`, `npm run lint`, and `npm run build` on all PRs.

2. **Frontend Testing Gap:**
   - Zero tests exist for React components, D3 chart calculations, or Zustand state stores.
   - Recommended remedy: Add Vitest + `@testing-library/react` + `jsdom` to test:
     - Byte and elapsed time formatting (`src/utils/formatters.ts`)
     - Zustand store state transitions (`src/stores/cleanupStore.ts`, `src/stores/driveStore.ts`)
     - UI component interaction and modals (`CleanupConfirmationModal.tsx`).

---

*Testing analysis: 2026-11-06*
