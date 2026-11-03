# X-Link v0.8.0 Cross-Platform Desktop & Sing-Box Routing Architecture

* **Author:** Isuruzenith & Claude Code (EGM Brainstorm Swarm)
* **Date:** 2026-11-03
* **Target Version:** v0.8.0 (Head: `feat/material-web-redesign`)
* **Artifact Path:** `docs/designs/concepts_brief.md`

---

## Overview

X-Link v0.8.0 elevates the application from a Windows-focused consumer client into a cross-platform desktop routing client supporting Windows, macOS, and Linux. The release consolidates the Material 3 (M3) desktop redesign, delivers reliable operating-system-level TUN mode across all three platforms, exposes sing-box routing primitives (outbound groups, detour chaining, and rule-to-node targeting) through progressive disclosure, hardens sidecar process supervision, and introduces structured diagnostic and backup tooling.

To ensure long-term stability and prevent cognitive overload, the design enforces strict architectural invariants:
1. Core dashboard simplicity remains intact: a single primary connection switch and a unified egress target.
2. Advanced routing topologies (such as failover pools and multi-hop proxy chains) require pre-flight Directed Acyclic Graph (DAG) cycle validation and referential fallback guards.
3. Telemetry streams (connection polling, DNS traces, packet inspections) are strictly bounded via native Rust ring buffers and backoff circuits to avoid UI thread freezing.
4. The Obsidian documentation vault (`docs/00`–`07`) is brought to full v0.8.0 parity and expanded with `docs/08-User-Guide-and-FAQ.md` and actionable incident triage runbooks.

---

## Analysis

Six specialized review lenses audited the proposed v0.8.0 architecture against the existing codebase (`src/` and `src-tauri/`). The consolidated findings are categorized below:

### 1. Architectural & Resource Economics (Design Critic Lens)
* **Clash API Polling Churn**: The current 1-second polling loop against `GET /connections` instantiates a new HTTP client on every call without connection pooling or read timeouts. In high-concurrency workloads (1,000–5,000 active sessions during downloads or streaming), serializing and deserializing multi-megabyte JSON payloads across the Tauri IPC boundary generates 60–120 MB/min of garbage collection churn, causing UI thread stutter in WebKit and WebView2.
  * *Required Safeguard*: Implement a backend HTTP client singleton with an 800ms read timeout, pause telemetry polling when `ConnectionsView` is unmounted or minimized, cap active connections at top 100 sorted by throughput, and employ virtualized table rendering (`@tanstack/react-virtual` or `react-window`).
* **Unbounded Telemetry Buffering**: Real-time packet tracing and DNS query inspection risk flooding the Tauri event bus and exhausting heap memory during heavy network traffic.
  * *Required Safeguard*: Isolate diagnostic inspection into a fixed-capacity native Rust ring buffer (maximum 500 DNS query records and 500 packet traces), batch IPC dispatches every 250–500ms, and enforce an automatic 5-minute inactivity session timeout.
* **Outbound Group Egress Quota & Battery Drain**: Continuous background health checks in automated failover groups (`urltest`, `fallback`) can silently consume hundreds of megabytes of metered cellular data and prevent network cards from sleeping.
  * *Required Safeguard*: Enforce a minimum health-check interval of 5 minutes (default 10 minutes), limit group membership to 10 nodes maximum per automated group, and pause active health checks when on battery power or idle.

### 2. Codebase Feasibility & Contract Parity (Codebase Details Lens)
* **Outbound Action Hardcoding**: `src-tauri/src/config/rules.rs` currently implements a closed 3-way matcher (`map_outbound_action`) that collapses all non-direct/non-block actions into `"proxy"`, and `src/utils/store.ts` restricts `OutboundAction` to `'proxy' | 'direct' | 'block' | 'dns'`. Targeting custom outbound groups requires expanding these contracts to dynamic string identifiers while preserving the default `"proxy"` fallback.
* **macOS Elevation & `utun` Naming Defect**: Currently, macOS elevation relaunches the entire GUI bundle via `osascript "with administrator privileges"`, which triggers WebKit sandbox crashes on macOS 14+ and corrupts user application data ownership. Additionally, `src-tauri/src/config/generator.rs` hardcodes `interface_name: "X-Link"`, which fails on Darwin because the kernel requires `utun[0-9]+` (or omitted interface name for auto-allocation).
  * *Required Safeguard*: Omit `interface_name` on macOS to permit kernel auto-allocation, and isolate root privileges from the WebKit GUI process.
* **Documentation Version Drift**: Core documentation in `docs/00-Home-MOC.md` currently references `v0.6.0` and documents 38 IPC commands, whereas the live codebase contains 46 commands (including newly added `simulate_routing`, `export_system_diagnostics`, and backup operations). Full documentation alignment is required.

### 3. Simplicity, Scope & Anti-Creep (Zoom-Out / YAGNI Lens)
* **Reinventing Wireshark**: Attempting to implement low-level packet capture and deep packet inspection inside an application-layer proxy client introduces bloated native dependencies, driver instability, and maintenance debt.
  * *Required Boundary*: Constrain "Packet Tracer" to sing-box application-layer routing traces and Clash API connection inspection. Do not integrate raw libpcap or driver-level packet capture.
* **Avoid Monolithic Background Daemons**: Building a custom multi-process background daemon with XPC/socket IPC solely for macOS TUN mode adds Apple Team ID signing hurdles and installer complexity.
  * *Required Boundary*: Retain standard child process supervision across all platforms. Use minimal elevation helpers (such as `pkexec setcap` on Linux and a narrowly scoped Darwin helper script) rather than converting X-Link into a persistent root system service.

### 4. Usability & Cognitive Ergonomics (Comprehension Lens)
* **Dashboard Egress Ambiguity**: Introducing multiple outbound groups creates confusion regarding what the primary Connect switch targets.
  * *Required Design*: The Dashboard retains a single primary "Egress Selector" that can point to either an individual node or an outbound group. When a group is active, the status card displays both the group name and the current active resolved node (e.g., "Auto-Fastest -> Singapore 02 (42ms)").
* **Dangling Outbound References on Subscription Refresh**: When subscription updates rename or prune nodes, rules mapped directly to specific node tags become invalid, causing `sing-box check` to reject the configuration.
  * *Required Design*: Rules must map to persistent Outbound Groups or virtual targets rather than transient subscription node tags. If a referenced target disappears, the compiler must automatically fall back to the master `"proxy"` selector and flag a non-blocking warning in the UI.
* **Progressive Disclosure of Advanced Dialing**: Advanced multiplexing (smux/yamux) and transport parameters (TCP Fast Open, MPTCP) must reside in a collapsed "Advanced Networking" accordion with protocol validation guards (e.g., disallowing multiplexing controls on QUIC-native protocols like Hysteria2 and TUIC).

### 5. Migration & Backward Compatibility (Implementation Readiness Lens)
* **Schema Evolution to Version 3**: Bumping persistence schemas from v2 to v3 must happen cleanly across `settings.json`, `profiles.json`, and `routing.json`.
  * *Storage Model*: Outbound groups are stored in `routing.json` under an optional `"outboundGroups"` array.
  * *Safe Deserialization*: All Rust configuration structs must use `#[serde(default)]` so unmigrated JSON files load without error during headless launches.
  * *Memory Synchronization*: Frontend initialization in `App.tsx` must run `migrateStores()` and reload store caches before mounting views, preventing Invariant 3 dual-store overwrites.
* **Encrypted Backup Envelope Compatibility**: Upgrading backup archives from unencrypted JSON (v1) to AES-256-GCM (v2) must not lock out legacy backups.
  * *Envelope Contract*: Define a self-describing JSON envelope specifying encryption algorithm, KDF parameters (Argon2id salt, iterations, memory cost), nonce, authentication tag, and payload. `validate_backup_archive` accepts both v1 and v2 formats.

### 6. Observability & SRE Debuggability (Structural Lens)
* **Unsupervised Zombie Sidecars**: Invariant 4 (`Ownership-Scoped Process Control`) relies on clean application exits. Abrupt crashes leave sing-box running in the background holding ports `2080`, `7892`, `9090`, and the TUN interface.
  * *SRE Safeguard*: Bind the child process to a Windows Job Object (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) on Windows and set `PR_SET_PDEATHSIG` (`SIGTERM`) on Linux. On startup, probe previous session ports and terminate any verified orphaned sing-box process before launching a new instance.
* **Traceable DAG Validation Errors**: If an operator configures circular detour dependencies (e.g., Node A detours to Node B, and Node B detours to Node A), the validation engine must not collapse the error into a generic string.
  * *SRE Safeguard*: Implement Kahn's topological sort in both TypeScript and Rust. Return structured validation payloads specifying error code `CYCLIC_DETOUR`, the offending tags, and the exact cycle path.
* **Operational Failure Runbooks**: The documentation vault lacks emergency procedures for host internet loss (stale proxy registry keys), ghost TUN interfaces, and credential recovery. Structured runbooks are mandatory.

---

## Goals vs. Non-Goals

### In-Scope Goals
1. **Cross-Platform Material 3 Desktop UI**: Responsive Material 3 shell across Windows (native controls), macOS (overlay title bar with 72px traffic light padding), and Linux (CSD/SSD compatibility), with dark mode as default.
2. **Cross-Platform TUN Mode**:
   - Windows: Bundled `wintun.dll` with UAC elevation.
   - Linux: Single-click Polkit (`pkexec`) GUI prompt to grant `setcap cap_net_admin,cap_net_bind_service=+ep` directly on the sing-box binary without running the desktop app as root.
   - macOS: Dynamic `utun` auto-allocation (removing the hardcoded `"X-Link"` tag) and a one-time scoped helper installation to eliminate repeated password prompts on connect while keeping the WebKit GUI unprivileged.
   - Failure Policy: Strict error dialog explaining driver/permission failure with retry options, preventing silent connection security downgrades.
3. **Outbound Groups & Routing Matrix**:
   - Support `urltest` (latency auto-failover), `fallback`, `loadbalance`, and custom `selector` groups.
   - Allow routing rules to target outbound groups or master egress with compile-time fallback to `"proxy"`.
   - Single-level or 2-hop detour chaining (entry relay -> exit node) with pre-flight DAG cycle validation.
4. **Hardened Telemetry & Telemetry Safeguards**:
   - Persistent HTTP client singleton with 800ms read timeout and circuit breaker for Clash API polling.
   - Fixed-capacity native ring buffers (500 entries) for DNS queries and connection events.
   - Windowed/virtualized rendering for connection tables and log streams.
5. **Full Subscription Management**:
   - Parsing for Clash YAML, Sing-box JSON, Base64 URI lists, and Shadowsocks SIP008 JSON formats.
   - Background update scheduling with randomized jitter, SSRF loopback filtering, and node identity deduplication.
6. **Encrypted Backup Subsystem**:
   - Version 2 backup archive format supporting optional AES-256-GCM encryption with Argon2id key derivation, maintaining full backward compatibility with v1 plaintext archives.
7. **Complete Documentation Vault Alignment**:
   - Update `docs/00` through `07` to v0.8.0 specification and 46 Tauri IPC commands.
   - Author `docs/08-User-Guide-and-FAQ.md` with end-user onboarding, core concepts explained in plain English, and an emergency troubleshooting matrix.
   - Create operational triage runbooks for host internet recovery and ghost TUN interface deallocation.

### Out-of-Scope Non-Goals
1. **Raw Packet Capture & Kernel Dissection**: No integration of `libpcap`, WinPcap, or raw socket sniffers. Inspection is strictly limited to sing-box routing events and Clash API connection telemetry.
2. **Persistent Root System Daemons**: No Windows Services or persistent macOS `launchd` system daemons running 24/7 in Session 0. Sidecar process lifecycle remains tied to the active user session.
3. **Future Pure-Rust Rewrite**: The `eframe`/`egui` rewrite remains strictly out-of-scope for v0.8.0. All work targets the production Tauri v2 + React 19 architecture.
4. **Arbitrary Unbounded Detour Graphs**: No arbitrary N-tier proxy mesh routing. Detour chains are capped at a maximum of 2 hops (Relay -> Exit) to prevent routing loops and socket exhaustion.
5. **Sensitive Secret Redaction from Disaster Recovery Backups**: Backups intended for restoration must preserve complete cryptographic fidelity (UUIDs, keys, passwords). Token redaction applies exclusively to diagnostic support bundles (`export_system_diagnostics`).

---

## Alternatives Weighed

1. **Persistent Privileged System Daemon vs. Scoped Child Process Elevation**:
   - *Alternative Considered*: Installing a persistent background system daemon (`com.xlink.daemon`) on macOS and a Windows Service to manage sing-box independently of the UI.
   - *Reason Rejected*: Introducing system-level daemons breaks Invariant 4 (owned child process supervision via pipes), complicates uninstallation, creates cross-user file ownership conflicts in user configuration directories, and requires Apple Developer Team ID notarization for privileged helper tools. Scoped elevation (wintun on Windows, `setcap` on Linux, and auto-allocated `utun` on macOS) preserves process ownership within the user session.
2. **Direct Rule-to-Node Binding vs. Indirection via Outbound Groups**:
   - *Alternative Considered*: Allowing routing rules to select individual subscription node tags directly in the rule editor.
   - *Reason Rejected*: Subscription node tags change frequently upon remote updates, creating dangling references that crash sing-box config validation. Routing rules must target persistent Outbound Groups or virtual tags with automated fallback to the master `"proxy"` selector.
3. **Unchecked Arbitrary Detour Chaining vs. DAG-Validated 2-Hop Chains**:
   - *Alternative Considered*: Allowing users to set the `detour` field on any outbound without graph restrictions.
   - *Reason Rejected*: Users easily configure circular dependencies (A -> B -> A), causing the Go runtime to deadlock or exit early with opaque startup errors. Limiting chains to 2 hops and enforcing compile-time DAG cycle checks eliminates loop hazards.
4. **Unencrypted Backup Format vs. Encrypted Envelope**:
   - *Alternative Considered*: Keeping backups strictly unencrypted plain JSON indefinitely.
   - *Reason Rejected*: Users store sensitive subscription URLs and private proxy credentials in backups. An encrypted envelope using AES-256-GCM and Argon2id protects credentials at rest while self-describing metadata allows seamless backward compatibility with legacy v1 backups.

---

## Key Decisions & Staging for Next Steps

### Key Technical Decisions
* **Decision 1 (Routing Engine & Outbound Targets)**: Maintain the master `"proxy"` selector as the root egress for all default traffic. Routing rules target persistent Outbound Groups only (never ephemeral subscription nodes directly). If an outbound group tag is missing, the compiler safely falls back to `"proxy"`.
* **Decision 2 (Detour Topology)**: Enforce a strict 2-hop limit (Entry Relay -> Exit Node) and execute pre-flight Kahn's DAG cycle detection in both TypeScript and Rust. Reject cyclic links with structured error payloads before writing `active.json`.
* **Decision 3 (Process Integrity & Zombie Prevention)**: Protect against zombie sidecars by attaching Windows Job Objects on Windows, applying `PR_SET_PDEATHSIG` on Linux, and executing pre-launch socket collision sweeps on startup.
* **Decision 4 (Telemetry Bounding)**: Replace unthrottled Clash API polling with a persistent `reqwest` client singleton (800ms read timeout), pause polling when `ConnectionsView` is unmounted, cap the table to the top 100 connections, and virtualize row rendering.
* **Decision 5 (macOS TUN Ergonomics)**: Remove the hardcoded `"X-Link"` interface name to enable Darwin dynamic `utun` allocation. Use standard `osascript` administrator prompts on connection initiation, avoiding complex multi-process background daemons.
* **Decision 6 (Schema Migration & Backup Envelope)**: Bump configuration schema to `_schemaVersion: 3`. Store outbound groups inside `routing.json`. Decorate Rust config structs with `#[serde(default)]`. Introduce self-describing v2 backup envelope with optional AES-256-GCM encryption.
* **Decision 7 (Documentation Vault Structure)**: Retain the existing Obsidian documentation vault (`docs/00` through `07`), update all IPC contracts and architecture diagrams to v0.8.0 parity, and introduce `docs/08-User-Guide-and-FAQ.md` containing plain-English onboarding guides and SRE failure recovery runbooks.

### Staging for Next Steps (`/egm-draft`)
* **Stage 1: Core Contracts & Schemas**: Define TypeScript interfaces in `src/utils/store.ts` and Rust structs in `src-tauri/src/config/` for Outbound Groups, DAG cycle validation, and the v3 schema migration.
* **Stage 2: Process & Telemetry Hardening**: Implement Windows Job Objects, Linux parent-death signals, Clash API client timeouts, and ring-buffered diagnostic streams.
* **Stage 3: Cross-Platform TUN & OS Layer**: Implement dynamic Darwin `utun` allocation, Linux capability validation, and unified platform error reporting.
* **Stage 4: Frontend UI Redesign & Outbound Management**: Refine Material 3 window styling, integrate the Outbound Group manager and progressive Node Editor into secondary views, and preserve the simple single-switch Dashboard.
* **Stage 5: Documentation Vault & Runbooks**: Update `docs/00` through `07` to reflect 46 IPC commands and author `docs/08-User-Guide-and-FAQ.md`.
