# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |
| < latest | :x:               |

## Reporting a Vulnerability

We take the security of X-Link seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

> **⚠️ Please do NOT open a public GitHub issue for security vulnerabilities.**

Instead, use one of the following methods:

1. **GitHub Private Vulnerability Reporting** (preferred): Navigate to the [Security tab](https://github.com/Isuruzenith/X-Link/security/advisories/new) of this repository and submit a private advisory.
2. **Email**: Send details to `contact@isuruzenith.com` with the subject line `[X-Link Security]`.

### What to Include

- A description of the vulnerability and its potential impact.
- Steps to reproduce the issue.
- Any relevant logs, screenshots, or proof-of-concept code.
- Your suggested fix (if any).

### Response Timeline

- **Acknowledgment**: Within 48 hours of your report.
- **Initial Assessment**: Within 5 business days.
- **Resolution**: We aim to release a patch within 30 days for confirmed vulnerabilities.

### Scope

The following are in scope for security reports:

- The X-Link desktop application (Tauri/Rust backend and React frontend).
- The `sing-box` configuration generation logic.
- Credential handling and storage via `tauri-plugin-store`.
- System proxy and TUN interface management.

Third-party dependencies (e.g., `sing-box` core) should be reported to their respective maintainers.

### Recognition

We appreciate responsible disclosure and will credit reporters (with permission) in our release notes.
