---
title: "99 — Vault Changelog & Documentation Drift Register"
aliases: ["Changelog", "Drift Register"]
tags:
  - changelog
  - architecture
created: 2026-11-06
updated: 2026-11-06
---

# 99 — Vault Changelog & Documentation Drift Register

**Navigation**: [[00-Home-MOC]]

## 1. Revision 2026-11-12 (EGM Knowledge Vault Audit & Reconciliation)

**Resolved & Synchronized**
- **Codebase Truth Alignment**: Reconciled the vault against codebase reality following commits `3a659ef`, `00beacc`, `39d6566`, `6c6648e`, and `5e3a641`.
- **Zero-Data Troubleshooting Correction**: Updated [[10-Zero-Data-and-Proxy-Node-Troubleshooting]] Sections 2.4 and 4.3 to accurately reflect that `raw_uri.rs` preserves pristine share link parameters verbatim on import, while runtime `generator.rs` handles dynamic `insecure: true` injection and injects static predefined DNS rules while retaining domain hostnames in outbound `server`.
- **Tauri IPC Command Dictionary**: Added all 12 missing production Tauri commands (`get_tunnel_status`, `start_tunnel`, `stop_tunnel`, `reload_tunnel`, `select_active_node`, `get_logs`, `clear_logs`, `get_persisted_state`, `save_generator_settings`, `save_routing_rules`, `preview_singbox_config`, `bulk_apply_bug_host`) to [[06-IPC-Bridge-and-Data-Contracts]] and updated total registered command count from 49 to 61 across [[00-Home-MOC]], [[05-Rust-Backend-Architecture]], and [[06-IPC-Bridge-and-Data-Contracts]].
- **Engineering Invariants Reconciled**: Updated [[07-AI-Agent-Playbook]] Invariant 1 (stripping `"h2"` from WebSocket/HTTPUpgrade ALPN) and Invariant 5 (authorizing automated graceful TUN fallback to System Proxy during Zero-Data network probe timeouts).
- **Broken Wikilink Purge**: Removed 64+ broken links referencing deleted `specs/`, `wireframes/`, and phantom folders (`notes/`, `decisions/`, `meetings/`, `_inbox/`) across [[00-Home-MOC]], [[02-Frontend-UI-Architecture]], [[03-UI-Components-and-Views]], [[04-UI-Wireframes]], [[07-AI-Agent-Playbook]], [[09-Design-System]], and [[README]].
- **Version, Bundle Identifier & Window Geometry**: Aligned documented version to `v0.9.0`, bundle identifier to `com.xlink.app`, and window geometry to `950 × 650` (min `380 × 550`).
- **Design System Realignment**: Removed phantom `src/styles/tokens.css` references in [[09-Design-System]] and documented the active token layers in `src/material-theme.css` and `src/index.css`.

## 2. Revision 2026-11-06

**Added**
- [[09-Design-System]] — Material 3 tokens, component map (Material Web vs custom), icon strategy, accessibility baseline.
- `wireframes/00-window-and-grid` — window size, size classes, fit rules and budgets.
- `wireframes/09-fit-layouts` — exact-scale wireframes (1 col = 10 px) for all six views, the node-editor sheet, the minimum window and Compact.
- `specs/` — `specs/README`, `specs/constitution`, templates, and four feature specs (`001-self-hosted-icons`, `002-desktop-window-shell`, `003-responsive-fit-layout`, `004-material-web-ui-conformance`). *(Note: `specs/` and `wireframes/` trees were later removed in commit 3a659ef).*

**Fixed**
- [[00-Home-MOC]]: removed a duplicated, outdated `src-tauri` tree; fixed capability numbering (`1,2,3,4,2,3,4` → `1…7`); corrected window values; added navigation rows, active-work table, repo-map entries.
- [[04-UI-Wireframes]]: superseded-geometry banner (window, rail, macOS inset).
- `wireframes/README` and `wireframes/01-shell-navigation`: window `950×650 / 380×550` replaced by `1000×680 / 720×520`; breakpoints replaced by the three size classes.
- `wireframes/02-dashboard` … `wireframes/08-settings`: "Target geometry" callout linking the new rules.
- [[02-Frontend-UI-Architecture]], [[03-UI-Components-and-Views]]: rail widths, icon font self-hosting, stale `X-Link v0.6.0` label, drift warning.
- [[07-AI-Agent-Playbook]]: new §4 *Spec-Driven Workflow*, Recipes D and E.
- `.obsidian/graph.json`: color groups for `#spec`, `#design-system`, `#sdd`.

**Untouched**: all Rust/IPC/backend notes (01, 05, 06, 08), `designs/*` architecture documents, the GitHub Pages landing files (`index.html`, `style.css`, `app.js`).

## 3. Drift register — verify against source before relying on it

| # | Notes disagree about | Source A | Source B | Resolution |
| :--- | :--- | :--- | :--- | :--- |
| 1 | Window size | `1080×700`, min `900×620` ([[04-UI-Wireframes]], old MOC) | `950×650`, min `380×550` (tauri.conf.json) | Resolved: `950×650` / `380×550` canonical config |
| 2 | Rail width | `220px` ([[02-Frontend-UI-Architecture]], [[03-UI-Components-and-Views]]) | `212px` / `64px` / `290px` drawer | `220 / 80` |
| 3 | Breakpoints | five sets across wireframes (`1140/1050/850/700/440`) | – | Respect desktop window bounds |
| 4 | App version label | `X-Link v0.6.0` / `v0.8.0` | `v0.9.0` | Resolved: `0.9.0` (`package.json`, `Cargo.toml`, `tauri.conf.json`) |
| 5 | Outbound Groups, encrypted v2 backup | Implemented ([[00-Home-MOC]], 03, 05, 06) | Historical proposals | Implemented in Rust backend |
| 6 | Dashboard server UI | Server **table** with TCP/URL ping buttons ([[03-UI-Components-and-Views]]) | Server chips | Table with ping actions |
| 7 | Styling stack | Tailwind + shadcn/Radix + Material Web ([[02-Frontend-UI-Architecture]]) | `material-theme.css` + `index.css` | Resolved: Material Web + `index.css` / `material-theme.css` |
| 8 | macOS top inset | `72px` | 40px proposal | Implemented: `72px` (`pt-[72px]`) |
| 9 | Material Web status | Treated as the long-term UI base | Repository is in maintenance mode | Wrapper layer in `src/components/ui/` |
| 10 | Runtime network assets | Country flags from `flagcdn.com`; landing page uses Google Fonts | Offline-first intent | Flags bundled; app never loads CDN fonts |
