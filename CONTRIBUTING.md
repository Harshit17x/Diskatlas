# Contributing to DiskAtlas 🧭

Thank you for your interest in contributing to **DiskAtlas**! We welcome contributions from developers, designers, technical writers, and testers of all skill levels.

---

## 🌟 How You Can Contribute

There are many ways to contribute to DiskAtlas:
1. **Reporting Bugs**: Let us know if you find incorrect calculations, scanner errors, or UI glitches.
2. **Feature Requests**: Suggest ideas to improve developer storage intelligence, visualizations, or workflows.
3. **Adding Developer Ecosystems**: Help us detect more build caches, dependencies, or IDE storage.
4. **Documentation**: Improve guides, fix typos, and add platform-specific instructions.
5. **Translations**: Help localize DiskAtlas into more languages.
6. **Code Contributions**: Submit PRs for open issues or performance enhancements.

---

## 🛠️ Development Environment Setup

### 1. Prerequisites
- **Node.js**: v20+ LTS
- **Rust & Cargo**: v1.80+ ([rustup.rs](https://rustup.rs))
- **Platform Dependencies**:
  - **macOS**: `xcode-select --install`
  - **Windows**: Visual Studio 2022 C++ Build Tools & WebView2 runtime
  - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`

### 2. Getting the Code

```bash
# 1. Fork the repository on GitHub
# 2. Clone your fork locally
git clone https://github.com/<your-username>/Diskatlas.git
cd Diskatlas

# 3. Install frontend dependencies
npm install

# 4. Verify the backend test suite passes
cargo test --manifest-path src-tauri/Cargo.toml

# 5. Launch DiskAtlas in development mode with hot-reload
npm run tauri dev
```

---

## 🧑‍💻 Code Style Guidelines

- **Rust**: Format with `cargo fmt`. Lint with `cargo clippy`. Ensure all tests pass.
- **TypeScript & React**: Format with Prettier. Adhere to strict TypeScript without `any`.
- **Safety First**: Destructive operations must NEVER delete files without user confirmation. System-protected paths must remain guarded.
- **Zero Telemetry**: We never add third-party analytics, tracking pixels, or remote telemetry SDKs.

---

## 🚀 Submitting a Pull Request (PR)

1. **Branch Naming**:
   - `feat/feature-name` for new features
   - `fix/bug-description` for bug fixes
   - `docs/topic-name` for documentation updates
2. **Commit Messages**: Write clear, descriptive commit messages:
   - `feat(developer): add support for Elixir _build directory`
   - `fix(scanner): prevent recursion into pseudo device nodes`
3. **Test Coverage**: If adding a new feature or fixing a bug, please add corresponding unit tests in `src-tauri/src/services/` or test the frontend thoroughly.
4. **Open the PR**: Submit your PR against the `main` branch with a clear description of the problem and solution.

---

## 📜 Code of Conduct

DiskAtlas follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). Please be respectful and welcoming to all contributors.

