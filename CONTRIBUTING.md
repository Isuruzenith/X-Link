# Contributing to X-Link

Thank you for your interest in contributing to X-Link! This guide will help you get started.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How Can I Contribute?](#how-can-i-contribute)
- [Development Setup](#development-setup)
- [Code Standards](#code-standards)
- [Commit Convention](#commit-convention)
- [Pull Request Process](#pull-request-process)

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## How Can I Contribute?

### Reporting Bugs

- Search [existing issues](https://github.com/Isuruzenith/X-Link/issues) to avoid duplicates.
- Use the [Bug Report template](https://github.com/Isuruzenith/X-Link/issues/new?template=bug_report.md) to file a new issue.
- Include your OS, app version, proxy protocol, and connection mode.

### Suggesting Features

- Use the [Feature Request template](https://github.com/Isuruzenith/X-Link/issues/new?template=feature_request.md).
- Describe the problem you're trying to solve, your proposed solution, and alternatives you've considered.

### Security Vulnerabilities

- **Do NOT open a public issue.** See [SECURITY.md](SECURITY.md) for responsible disclosure instructions.

## Development Setup

### Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Node.js | v22+ | Required for the React frontend |
| npm | v10+ | Comes with Node.js |
| Rust | Stable (latest) | Install via [rustup](https://rustup.rs/) |
| C++ Build Tools | — | Windows only: install via Visual Studio Build Tools |

### Quick Start

```bash
# Clone the repository
git clone https://github.com/Isuruzenith/X-Link.git
cd X-Link

# Install frontend dependencies
npm install

# Run in development mode (auto-downloads sing-box sidecar)
npm run tauri dev
```

> **Note**: The pre-build script (`scripts/setup-sidecars.cjs`) automatically downloads the correct `sing-box` binary (and `wintun.dll` on Windows) into `src-tauri/binaries/`. These files are git-ignored.

## Code Standards

### Frontend (TypeScript / React)

```bash
# Lint the frontend code
npm run lint
```

- Follow the ESLint configuration defined in `eslint.config.js`.
- Use functional React components with hooks.
- Keep components focused and reusable.

### Backend (Rust)

```bash
# Format Rust code
cargo fmt

# Run the linter
cargo clippy -- -D warnings

# Type-check without building
cargo check
```

- Follow standard Rust idioms and the existing code style.
- All new Tauri commands go in `src-tauri/src/commands/`.
- Configuration generation logic lives in `src-tauri/src/config/`.

## Commit Convention

This project uses [Conventional Commits](https://www.conventionalcommits.org/) enforced by [Release Please](https://github.com/google-github-actions/release-please-action) for automated versioning and changelogs.

### Format

```
<type>(<optional scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Purpose |
|------|---------|
| `feat` | A new feature (triggers a **minor** version bump) |
| `fix` | A bug fix (triggers a **patch** version bump) |
| `docs` | Documentation-only changes |
| `style` | Code style changes (formatting, no logic change) |
| `refactor` | Code restructuring (no feature or fix) |
| `perf` | Performance improvements |
| `test` | Adding or updating tests |
| `chore` | Maintenance tasks (deps, CI config) |

### Examples

```bash
git commit -m "feat: add Shadowsocks protocol support"
git commit -m "fix: prevent ALPN h2 in WebSocket outbounds"
git commit -m "docs: update AI_HANDOVER with FakeIP details"
```

> **Breaking Changes**: Add `BREAKING CHANGE:` in the commit footer or use `!` after the type (e.g., `feat!: redesign config schema`). This triggers a **major** version bump.

## Pull Request Process

1. **Fork** the repository and create your branch from `main`.
2. **Make your changes** following the code standards above.
3. **Test locally** — ensure `npm run lint`, `cargo fmt --check`, `cargo clippy`, and `npm run build` all pass.
4. **Commit** using Conventional Commits.
5. **Open a PR** using the [Pull Request template](.github/pull_request_template.md).
6. **Respond to review feedback** promptly.

### What We Look For

- Clean, well-documented code.
- Passing lint and format checks.
- No regressions to existing proxy protocol support.
- Adherence to the architectural rules documented in [AI_HANDOVER.md](AI_HANDOVER.md).

---

Thank you for helping make X-Link better! 🚀
