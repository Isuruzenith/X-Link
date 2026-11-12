# Consolidated EGM Review Report: X-Link Obsidian Knowledge Vault

- **Target:** `docs/` (Obsidian Knowledge Vault markdown files `00` through `10`, `99`, `README.md`)
- **Review Pipeline:** Curated Preset 1 — Consolidated Report with all review rubrics (Full Flow)
- **Review Date:** 2026-11-12
- **Status:** Complete — All Findings Remediated & Verified (2026-11-12)

---

## 1. Executive Summary & Review Scope

An exhaustive EGM (Elephant-Goldfish Model) audit was executed across all 13 primary documentation files within the X-Link Obsidian Knowledge Vault (`docs/`), evaluating them against:
1. **Codebase Reality:** Exact alignment with `src/`, `src-tauri/`, `package.json`, `Cargo.toml`, and `tauri.conf.json`.
2. **Topological Coherence:** Dead wikilinks, orphan references, and circular contradictions.
3. **Architectural Invariants:** Consistency between non-negotiable playbook rules (`07-AI-Agent-Playbook.md`), troubleshooting guidelines (`10-Zero-Data-and-Proxy-Node-Troubleshooting.md`), and recent commit changes (`3a659ef`, `6c6648e`, `5e3a641`).

The review confirms that while the documentation vault provides extensive system context, recent code changes and directory cleanups have introduced severe documentation drift, 64+ broken links to deleted folders, 12 undocumented Tauri IPC commands, and direct contradictions regarding core engineering invariants.

---

## 2. Review Stages & Synthesis Findings

### Stage 5: Initial Zoom Out (Scope & High-Level Architecture)
- **Bloat vs Reality:** The documentation vault maintains extensive architectural claims for sub-systems and specifications that were deleted or never implemented. Specifically, `00-Home-MOC.md` and `99-Vault-Changelog.md` heavily cite `specs/` (001–004) and `wireframes/` (00–09), both of which were completely deleted in commit `3a659ef`.
- **System Version Drift:** The codebase has progressed to version `0.9.0` (`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`), yet the core vault entrypoint (`00-Home-MOC.md`) defines the application as `v0.8.0` under identifier `com.isuru.x-link` (the real identifier is `com.xlink.app`).

### Stage 6: Comprehension Test (Self-Containment & Broken Links)
- **64+ Broken Wikilinks:** A naive reader or AI coding agent attempting to navigate the vault through Obsidian wikilinks encounters broken link walls across 8 separate markdown files.
- **Phantom Folders:** The Obsidian Workspace table in `00-Home-MOC.md` and `README.md` points to `notes/`, `decisions/`, `meetings/`, and `_inbox/` folders that do not exist in the repository.
- **Wireframe Misdirection:** `04-UI-Wireframes.md` explicitly warns the reader that its diagrams are historical proposals and directs the reader to `[[wireframes/README|Current UI/UX Wireframes]]` for source-aligned layouts. Because `wireframes/` was deleted, the user is left with no valid wireframe reference.

### Stage 7: Critic Review (Structural, Design & Style Contradictions)
- **Contradiction on Invariant 5 (TUN Fallback):**
  - `07-AI-Agent-Playbook.md` Invariant 5 strictly forbids silent TUN fallback: *"In v0.8.0, silent modeOverride = 'system' fallback is strictly forbidden. If TUN mode initiation fails or required OS elevation is missing, the application must present a structured diagnostic error modal rather than silently downgrading the connection security posture."*
  - Conversely, commit `6c6648e` and `10-Zero-Data-and-Proxy-Node-Troubleshooting.md` implement and document automated fallback from TUN mode to System Proxy mode during zero-data probe timeouts. This is a direct architectural conflict.
- **Contradiction on Invariant 1 (ALPN Handling):**
  - `07-AI-Agent-Playbook.md` Invariant 1 commands that for VLESS/VMess over WebSocket/HTTPUpgrade, ALPN *must be forced* to `["http/1.1"]`.
  - `docs/designs/sing-box-configuration-recovery.md` Goal 1 mandates that profile-specified ALPN must be preserved and not overridden.
  - The actual implementation in `src-tauri/src/config/generator.rs` selectively filters out `"h2"` while preserving other values, rather than blindly overwriting the array.

### Stage 8: Intermediate Zoom Out (Codebase Alignment Audit)
- **Commit `5e3a641` Contradiction in Doc 10:**
  - `10-Zero-Data-and-Proxy-Node-Troubleshooting.md` was added on 2026-11-12. In Section 2.4, it claims that `raw_uri.rs` auto-detects spoofed SNIs on import and injects `insecure: true`.
  - In Section 4.3, it claims that `generator.rs` replaces the outbound `server` field with a pre-resolved IP address.
  - In reality, commit `5e3a641` deliberately removed both of these mechanisms from `raw_uri.rs` and `generator.rs` to preserve link intent and prevent TLS handshake breakages. The document directly describes code that was deleted in the exact same commit.

### Stage 9: Implementation Readiness Test (Technical Accuracy & Contracts)
- **IPC Command Discrepancy:**
  - `06-IPC-Bridge-and-Data-Contracts.md` and `05-Rust-Backend-Architecture.md` document 49 Tauri commands.
  - `src-tauri/src/lib.rs` registers 61 commands. 12 production commands are missing from documentation.
- **Window Geometry Discrepancy:**
  - `00-Home-MOC.md`, `04-UI-Wireframes.md`, and `99-Vault-Changelog.md` define canonical window dimensions as `1000 × 680` (min `720 × 520`).
  - `src-tauri/tauri.conf.json` defines window dimensions as `950 × 650` (min `380 × 550`).
- **Phantom Tokens File:**
  - `09-Design-System.md` specifies that UI components must consume `--xl-*` tokens defined in `src/styles/tokens.css`.
  - `src/styles/tokens.css` does not exist, and `--xl-*` tokens are absent from all frontend components.

---

## 3. Itemized Findings by Severity

### 🔴 Blocker Findings

#### [FINDING-01] Stale Code Claims in `10-Zero-Data-and-Proxy-Node-Troubleshooting.md`
- **File:** `docs/10-Zero-Data-and-Proxy-Node-Troubleshooting.md:56-80, 167-170`
- **Defect:** Sections 2.4 and 4.3 document code blocks in `src-tauri/src/config/adapters/raw_uri.rs` and `generator.rs` that were removed in commit `5e3a641`. The document states that the importer auto-injects `is_insecure = true` on import and that the generator replaces domain names in `server` with resolved IP addresses. Neither statement reflects the current codebase.
- **Impact:** Misleads developers and agents regarding where and how Bug Host SNI spoofing and DNS resolution are handled.

#### [FINDING-02] 64+ Dangling Wikilinks from Deleted `specs/` and `wireframes/` Trees
- **Files:** `docs/00-Home-MOC.md`, `docs/02-Frontend-UI-Architecture.md`, `docs/03-UI-Components-and-Views.md`, `docs/04-UI-Wireframes.md`, `docs/07-AI-Agent-Playbook.md`, `docs/09-Design-System.md`, `docs/99-Vault-Changelog.md`, `docs/README.md`
- **Defect:** Commit `3a659ef` purged `docs/specs/`, `docs/wireframes/`, and several design documents. The markdown files still link to these deleted resources.
- **Impact:** Broken navigation within Obsidian, missing specifications, and failure of agents attempting to load referenced specs.

#### [FINDING-03] 12 Production Tauri IPC Commands Missing from `06-IPC-Bridge-and-Data-Contracts.md`
- **File:** `docs/06-IPC-Bridge-and-Data-Contracts.md:19`
- **Defect:** Documents 49 commands while 61 commands are registered in `src-tauri/src/lib.rs`. Missing commands:
  1. `get_tunnel_status` (`commands/proxy.rs`)
  2. `start_tunnel` (`commands/proxy.rs`)
  3. `stop_tunnel` (`commands/proxy.rs`)
  4. `reload_tunnel` (`commands/proxy.rs`)
  5. `select_active_node` (`commands/proxy.rs`)
  6. `get_logs` (`commands/logs.rs`)
  7. `clear_logs` (`commands/logs.rs`)
  8. `get_persisted_state` (`commands/config.rs`)
  9. `save_generator_settings` (`commands/config.rs`)
  10. `save_routing_rules` (`commands/config.rs`)
  11. `preview_singbox_config` (`commands/config.rs`)
  12. `bulk_apply_bug_host` (`commands/config.rs`)
- **Impact:** Incomplete API contract dictionary; frontend engineers and agents lack reference for tunnel lifecycle, log clearing, config preview, and bug host bulk application.

#### [FINDING-04] Direct Contradiction on Invariant 5 (TUN Fallback vs Zero-Data Routing)
- **Files:** `docs/07-AI-Agent-Playbook.md:52-58` vs `docs/10-Zero-Data-and-Proxy-Node-Troubleshooting.md:171-174` & `src-tauri/src/commands/proxy.rs`
- **Defect:** Invariant 5 forbids automated or silent fallback to System Proxy mode when TUN mode fails. Commit `6c6648e` implemented automated fallback from TUN mode to System Proxy mode for Zero-Data routes upon probe timeout, which Doc 10 actively highlights as a core capability.
- **Impact:** Causes AI agents and contributors to reject or accidentally break zero-data recovery logic when attempting to comply with Invariant 5.

---

### 🟡 Major Findings

#### [FINDING-05] Application Version and Bundle Identifier Mismatches
- **Files:** `docs/00-Home-MOC.md:16`, `docs/04-UI-Wireframes.md:54`, `src-tauri/tauri.conf.json:3-5`
- **Defect:** The vault designates X-Link as `v0.8.0` with bundle identifier `com.isuru.x-link`. The active project configuration designates `v0.9.0` with identifier `com.xlink.app`.
- **Impact:** Stale bundle metadata across documentation and incorrect build assumptions.

#### [FINDING-06] Fictitious Token File Reference in `09-Design-System.md`
- **File:** `docs/09-Design-System.md:39-50`
- **Defect:** Cites `src/styles/tokens.css` as the source of truth for `--xl-*` tokens. Directory `src/styles/` does not exist, and the codebase defines theme variables in `src/material-theme.css` and `src/index.css` without `--xl-*` prefixes.
- **Impact:** Frontend styling guidance instructs developers to import and reference non-existent CSS tokens.

#### [FINDING-07] Canonical Window Geometry Conflict
- **Files:** `docs/00-Home-MOC.md:36`, `docs/04-UI-Wireframes.md:20`, `src-tauri/tauri.conf.json:16-19`
- **Defect:** Documents assert canonical window dimensions of `1000 × 680` with minimum `720 × 520`. The live configuration in `tauri.conf.json` remains `950 × 650` with minimum `380 × 550`.
- **Impact:** Wireframe dimensions and responsive layout expectations do not match the compiled desktop window.

#### [FINDING-08] Phantom Workspace Directories in Vault Navigation
- **Files:** `docs/00-Home-MOC.md:210-216`, `docs/README.md:14-19`
- **Defect:** References non-existent directories: `docs/notes/`, `docs/decisions/`, `docs/meetings/`, and `docs/_inbox/`.
- **Impact:** Clutters navigation and creates dead links in Obsidian.

#### [FINDING-09] Ambiguity in Invariant 1 vs Selective ALPN Filtering
- **Files:** `docs/07-AI-Agent-Playbook.md:24-28` vs `src-tauri/src/config/generator.rs`
- **Defect:** Invariant 1 states ALPN must be forced to `["http/1.1"]`. Actual logic in `generator.rs` only strips `"h2"` from ALPN arrays on WebSocket and HTTPUpgrade outbounds, retaining explicit user ALPN entries unless empty.
- **Impact:** Oversimplification in the Playbook obscures actual protocol negotiation behavior.

---

### 🟢 Nit Findings

#### [FINDING-10] Documentation Drift Register Untracked Since 2026-11-06
- **File:** `docs/99-Vault-Changelog.md`
- **Defect:** `99-Vault-Changelog.md` has not been updated to record the removal of specs/wireframes (`3a659ef`), the version bump to 0.9.0 (`00beacc`), sing-box recovery hardening (`39d6566`), or zero-data features (`6c6648e`, `5e3a641`).

#### [FINDING-11] Stale macOS Inset Targets
- **Files:** `docs/02-Frontend-UI-Architecture.md:38`, `docs/04-UI-Wireframes.md:22`
- **Defect:** References an aspirational target of 40px for the macOS traffic light inset while noting that the code currently uses 72px (`pt-[72px]`).

---

## 4. Traceability Matrix & Action Plan

| Finding ID | Severity | Target File(s) | Action Item / Concrete Remediation |
| :--- | :--- | :--- | :--- |
| **FINDING-01** | 🔴 Blocker | `docs/10-Zero-Data-and-Proxy-Node-Troubleshooting.md` | Update Sections 2.4 and 4.3 to accurately document that `raw_uri.rs` preserves pristine link parameters and `generator.rs` retains domain hostnames while relying on predefined DNS rules. |
| **FINDING-02** | 🔴 Blocker | `docs/00-Home-MOC.md`, `02`, `03`, `04`, `07`, `09`, `99`, `README.md` | Purge or update all dangling wikilinks pointing to deleted `specs/` and `wireframes/` files; restore canonical inline wireframe summaries where appropriate. |
| **FINDING-03** | 🔴 Blocker | `docs/06-IPC-Bridge-and-Data-Contracts.md`, `05-Rust-Backend-Architecture.md` | Add complete contract documentation for the 12 missing Tauri commands and update the total registered command count from 49 to 61. |
| **FINDING-04** | 🔴 Blocker | `docs/07-AI-Agent-Playbook.md` | Refine Invariant 5 to explicitly reconcile standard proxy mode transitions with the automated TUN-to-System fallback enabled for SniZeroData / zero-data routing. |
| **FINDING-05** | 🟡 Major | `docs/00-Home-MOC.md`, `04-UI-Wireframes.md` | Bump documented version to `v0.9.0` and update identifier to `com.xlink.app`. |
| **FINDING-06** | 🟡 Major | `docs/09-Design-System.md` | Remove reference to `src/styles/tokens.css` and accurately document the active token hierarchy in `src/material-theme.css` and `src/index.css`. |
| **FINDING-07** | 🟡 Major | `src-tauri/tauri.conf.json` or `docs/00-Home-MOC.md` | Align documented canonical window dimensions (`1000 × 680` / `720 × 520`) with `tauri.conf.json` (`950 × 650` / `380 × 550`). |
| **FINDING-08** | 🟡 Major | `docs/00-Home-MOC.md`, `docs/README.md` | Remove or create the referenced Obsidian workspace folders (`notes/`, `decisions/`, etc.). |
| **FINDING-09** | 🟡 Major | `docs/07-AI-Agent-Playbook.md` | Clarify Invariant 1 to state that `"h2"` is stripped from WebSocket/HTTPUpgrade ALPN rather than blindly overwriting custom ALPN arrays. |
| **FINDING-10** | 🟢 Nit | `docs/99-Vault-Changelog.md` | Add changelog entries covering commits `3a659ef` through `5e3a641`. |
| **FINDING-11** | 🟢 Nit | `docs/02-Frontend-UI-Architecture.md`, `04` | Standardize documentation on the actual implemented 72px macOS inset. |
