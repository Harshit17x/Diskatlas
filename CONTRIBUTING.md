# Contributing to DiskAtlas

Thank you for your interest in contributing to DiskAtlas. We welcome contributions from developers, designers, technical writers, and testers.

---

## Areas of Contribution

You can contribute to DiskAtlas in several ways:
1. **Bug Reports**: Document unexpected behavior, incorrect storage measurements, or user interface errors.
2. **Feature Proposals**: Suggest capabilities to improve developer storage discovery, visualizations, or workflows.
3. **Developer Ecosystem Support**: Add or improve detection rules for compilers, package managers, and development runtimes.
4. **Documentation**: Clarify guides, improve setup instructions, and update platform-specific build notes.
5. **Localization**: Assist in translating user-facing strings into additional languages.
6. **Code Contributions**: Address open issues, optimize database queries, or enhance scanner throughput.

---

## Development Environment Setup

### Prerequisites
- **Node.js**: v20+ LTS
- **Rust & Cargo**: v1.80+ ([rustup.rs](https://rustup.rs))
- **Platform Dependencies**:
  - **macOS**: `xcode-select --install`
  - **Windows**: Visual Studio 2022 C++ Build Tools and WebView2 runtime
  - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`

### Getting the Code

```bash
# 1. Fork the repository on GitHub
# 2. Clone your fork locally
git clone https://github.com/<your-username>/Diskatlas.git
cd Diskatlas

# 3. Install frontend dependencies
npm install

# 4. Verify that the backend test suite passes
cargo test --manifest-path src-tauri/Cargo.toml

# 5. Launch DiskAtlas in development mode with hot reload
npm run tauri dev
```

---

## Code Style and Quality Standards

- **Rust**:
  - Format code using `cargo fmt`.
  - Check for linter warnings with `cargo clippy`.
  - Ensure all 25 unit and integration tests pass: `cargo test --manifest-path src-tauri/Cargo.toml`.
- **TypeScript & React**:
  - Adhere to strict TypeScript typing; avoid using `any`.
  - Format code with Prettier and verify linting with `npm run lint`.
- **Safety**:
  - Destructive filesystem operations must never execute automatically.
  - Deletions must support reversible trash or Recycle Bin routing by default.
  - Critical operating system paths must remain protected.
- **Privacy**:
  - Do not add remote tracking, analytics SDKs, or background telemetry.

---

## Submitting Pull Requests

1. **Branch Naming**:
   - `feat/feature-name` for new features
   - `fix/bug-description` for bug fixes
   - `docs/topic-name` for documentation updates
2. **Commit Messages**: Use concise, conventional commit prefixes:
   - `feat(developer): add detection for Elixir _build directory`
   - `fix(scanner): prevent traversal into pseudo device nodes`
   - `docs: update Windows release guide`
3. **Testing**: Add or update unit tests to verify changes whenever modifying scanner logic, storage classification, or cleanup routines.
4. **Pull Request Description**: Clearly describe the problem, the solution, and any testing performed across target operating systems.

---

## Code of Conduct

DiskAtlas adopts the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). All contributors and maintainers are expected to uphold a respectful and collaborative environment.
