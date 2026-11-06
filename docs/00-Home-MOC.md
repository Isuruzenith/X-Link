---
title: "X-Link Documentation Vault — Master Map of Content (MOC)"
aliases: ["Home", "Index", "MOC", "AI Context"]
tags:
  - moc
  - architecture
  - ai-context
  - xlink
created: 2026-11-03
updated: 2026-11-06
---

# X-Link — Complete AI Agent Knowledge Vault

> [!IMPORTANT] AI Agent Orientation
> This Obsidian vault (`C:\Projects\X-Link\docs`) is the **complete, authoritative architectural and UI/backend context** for **X-Link** (`com.isuru.x-link`, v0.8.0). Any AI coding agent or developer reading this vault can understand every UI screen, React component, Zustand store, Tauri IPC bridge command/event, Rust backend module, `sing-box` sidecar configuration pipeline, and ASCII wireframe without guessing.

---

## Vault Navigation (Map of Content)

| Note | Scope & Contents | Primary Tags |
| :--- | :--- | :--- |
| **[[00-Home-MOC]]** | Master index, quick-reference architecture summary, technology stack, and repository map. | `#moc` `#architecture` |
| **[[01-Architecture-Overview]]** | End-to-end 3-tier system topology, boot lifecycle, cross-platform process supervision, 5-stage self-healing engine, transactional config pipeline, and non-negotiable core invariants. | `#architecture` `#backend` `#frontend` |
| **[[02-Frontend-UI-Architecture]]** | React 19 + TypeScript + Tailwind CSS v4 + Material Web (`@material/web`) styling system, theme tokens, `App.tsx` orchestration, and all **8 Zustand stores** (`connectionStore`, `profileStore`, `routingStore`, `settingsStore`, `nodeEditorStore`, `statsStore`, `logStore`, `toastStore`). | `#frontend` `#state-management` |
| **[[03-UI-Components-and-Views]]** | Exhaustive component reference: layout shell (`NavRail` with 72px macOS traffic light inset, `ViewShell`, `ToastContainer`, `ErrorBoundary`), domain editors (`NodeEditor`, `TrafficChart`), form controls (`FormInput`, `FormSelect`, `FormSwitchRow`, `SegmentedControl`), UI primitives, and all **6 main views** (`DashboardView`, `ConfigView`, `RoutingView`, `ConnectionsView`, `LogsView`, `SettingsView`). | `#frontend` `#components` `#ui` |
| **[[04-UI-Wireframes]]** | Historical UI design sketches; some proposed flows do not match the running app. | `#wireframe` `#ui` `#frontend` |
| **[[wireframes/README\|Current UI/UX Wireframes]]** | Source-aligned wireframes for the six current views, settings tabs, editor, dialogs, and responsive states. | `#wireframe` `#ui` `#frontend` |
| **[[05-Rust-Backend-Architecture]]** | Deep dive into `src-tauri`: `ProxyState`, `SingBox` sidecar management (`v1.13.14`), cross-platform process supervision (`SidecarChild`, Windows Job Objects, Linux `PR_SET_PDEATHSIG`, macOS root helper), config generator (`generator.rs`), Kahn's DAG cycle validator (`validator.rs`), Outbound Group routing (`rules.rs`), telemetry ring buffers, and encrypted backup/restore v2. | `#backend` `#rust` `#sing-box` |
| **[[06-IPC-Bridge-and-Data-Contracts]]** | Complete contract dictionary: all **49 Tauri `invoke` commands** (arguments, return types, side effects), all **8 Tauri events** (`emit` -> `listen`), Clash REST API (`127.0.0.1:9090`) integration, and on-disk JSON schemas (`settings.json`, `profiles.json`, `routing.json`, `stats.json`, `configs/active.json`, `sidecar.session.json`). | `#ipc` `#contracts` `#backend` `#frontend` |
| **[[07-AI-Agent-Playbook]]** | Practical engineering guide for AI agents: 7 non-negotiable invariants, Dual-Store Synchronization Rule (`@tauri-apps/plugin-store` vs Rust `std::fs`), Connected-Apply Policy (`applyPolicy.ts`), and step-by-step recipes for adding settings, protocols, or IPC commands. | `#ai-context` `#playbook` `#architecture` |
| **[[08-User-Guide-and-FAQ]]** | Comprehensive end-user guide: plain-English core concepts, platform-specific quickstarts (Windows, macOS, Linux), and operational runbooks for ghost TUN interface recovery and port conflict resolution. | `#user-guide` `#runbook` `#faq` |
| **[[09-Design-System]]** | Material 3 tokens, typography, component map (Material Web vs custom), **icon strategy that survives `tauri build`**, accessibility baseline. | `#design-system` `#frontend` |
| **[[wireframes/00-window-and-grid\|Window & Grid]]** | **Canonical** window (1000×680, min 720×520), size classes, fit-to-window rules, budgets, test matrix. | `#wireframe` `#layout` |
| **[[wireframes/09-fit-layouts\|Fit Layouts]]** | Exact-scale wireframes of every view at 1000×680 and 720×520. | `#wireframe` `#layout` |
| **[[specs/README\|Specs (SDD)]]** | Spec-driven development: constitution, templates and four ready-to-run feature specs (icons, window shell, responsive fit, Material Web conformance). | `#spec` `#sdd` |
| **[[99-Vault-Changelog]]** | What changed in this revision and the documentation-drift register. | `#changelog` |

---

## Executive Summary of X-Link

**X-Link** is a privacy-first, native cross-platform desktop VPN and proxy client built on **Tauri v2** (Rust backend + WebView2/WebKit frontend) wrapping the **sing-box v1.13.14** universal proxy platform for Windows, macOS, and Linux.

### Core Capabilities
1. **Cross-Platform Dual Proxy Modes**:
   - **TUN Mode (`proxyMode: "tun"`)**: Captures all system TCP/UDP/ICMP traffic via a virtual network interface without running the desktop GUI as root:
     - **Windows**: `wintun.dll` driver deployment with Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) and IP Helper DNS loop prevention.
     - **macOS**: Dynamic kernel `utun` interface allocation with root companion supervisor (`xlink-darwin-supervisor` in `/Library/Application Support/X-Link/helpers/` mode `0755` root:wheel) invoked via staged `sudo -n` and monitored via `kqueue` and `stdin` EOF heartbeats.
     - **Linux**: Unprivileged companion supervisor (`xlink-linux-supervisor`) with `libc::PR_SET_PDEATHSIG` set to `SIGTERM` and `pkexec setcap cap_net_admin,cap_net_bind_service=+ep` executed on helper binaries copied to `~/.local/share/x-link/helpers/`.
   - **System Proxy Mode (`proxyMode: "system"`)**: Configures OS-level HTTP/SOCKS/Mixed listeners (`7890` / `7891` / `7892`) and manages platform network settings (`WinINet` on Windows, `networksetup` on macOS, GNOME/GSettings on Linux) with automatic snapshot & crash-recovery restoration (`restore_system_proxy_sync`).
2. **Sing-Box v1.13 Routing Matrix & Detour DAG**:
   - **Outbound Groups**: Native GUI management of `urltest` (latency-based automated switching), `fallback` (hysteresis failover), and `selector` (manual group switching).
   - **2-Hop Detour Chaining**: Relay-to-Exit chaining with Kahn's Directed Acyclic Graph (DAG) cycle validation and protocol compatibility checks.
   - **Master Root Selector**: Root `"proxy"` selector prepends all Outbound Group tags before individual dialable node tags, enabling Clash API hot-switching (`PUT /proxies/proxy`). Fully pruned groups fall back to `["direct"]`, preventing cyclic selector loops.
3. **Hardened Live Telemetry Engine**:
   - View-gated Clash REST API polling with persistent `reqwest::Client` singleton, 800ms read timeout, top-100 throughput windowing, socket pruning, and a 10-second circuit breaker.
   - Dual 500-record native Rust ring buffers (`VecDeque`) for DNS exchange queries and routing trace matches (`[router] match ...`).
   - 60 FPS virtualized table rendering with `react-window`.
4. **Self-Describing Encrypted v2 Backups**:
   - Optional AES-256-GCM encryption with Argon2id key derivation strictly bounded to safe resource limits (1024 KiB to 65536 KiB memory, 1 to 10 iterations, parallelism = 1).
   - Backward-compatible with v1 unencrypted archives and v2 unencrypted envelopes.
   - Pre-lock proxy disconnection eliminating lock-inversion deadlocks.
5. **Universal Protocol & Subscription Support**:
   - **Outbound Protocols**: `VLESS` (with XTLS-Reality & uTLS), `VMess`, `Trojan`, `Shadowsocks`, `Hysteria2`, `TUIC v5`, `WireGuard`, `SSH`, `SOCKS5`, and `HTTP`.
   - **Import Formats**: Native `sing-box` JSON, Clash/Mihomo YAML, Base64-encoded subscription bundles, raw share links (`vless://`, `vmess://`, `trojan://`, `ss://`, `hysteria2://`, `hy2://`, `tuic://`, `wg://`), and remote HTTP/HTTPS subscriptions with **Subscription Diff Previews** and **Background Scheduled Auto-Updates**.
6. **5-Stage Self-Healing Connection Engine**:
   - Automatically recovers from TLS fingerprint blocks, DNS poisoning, or TUN driver failures during connection startup (`Chrome uTLS` $\rightarrow$ `Firefox uTLS` $\rightarrow$ `Native Go TLS` $\rightarrow$ `Public DNS Bootstrap` $\rightarrow$ `TUN Compatibility Profile` $\rightarrow$ `System Proxy Fallback`).
7. **Hot Node Switching & Validated Cold Restart**:
   - Switches servers with zero downtime via the authenticated Clash-compatible REST API (`PUT /proxies/proxy`) when `apiEnabled` is true, or falls back to a transactional cold restart.
   - Edits to settings or routing while connected are debounced (`800ms`), validated via `sing-box check`, and applied transactionally with automatic rollback if the new configuration fails to boot.

---

## Technology Stack Matrix

| Layer | Technology | Version / Details |
| :--- | :--- | :--- |
| **Desktop Shell** | Tauri | `v2` (`@tauri-apps/api ^2.10.1`, `tauri = "2"`) |
| **Frontend Framework** | React + TypeScript | `React ^19.2.0`, `TypeScript ~5.9.3`, `Vite ^8.0.0` |
| **State Management** | Zustand | `^5.0.12` (8 modular stores in `src/stores/`) |
| **Styling & Design System** | Tailwind CSS v4 + Material Web | `@tailwindcss/vite ^4.2.1`, `@material/web ^2.4.1`, `shadcn`/Radix primitives |
| **Data Visualization & Virtualization** | `react-window` + Custom SVG | `react-window ^2.2.7` (virtualized 2,000-line logs), SVG Bézier `TrafficChart` |
| **Tauri Plugins** | Official Plugins | `shell`, `store`, `clipboard-manager`, `dialog`, `process` |
| **Rust Runtime** | Tokio + Reqwest + Serde | `tokio 1` (async), `reqwest 0.12` (rustls-tls), `serde_json 1` |
| **Windows Native APIs** | `windows-sys` + `winreg` | `windows-sys 0.59` (`IpHlpApi`, `WinInet`, `Security`), `winreg 0.52` |
| **Proxy Core Sidecar** | `sing-box` + `wintun.dll` | Bundled `v1.13.14` (`src-tauri/binaries/sing-box-*`, `wintun.dll`) |

---

## Repository Directory Map

```text
C:\Projects\X-Link\
├── docs\                                  # [THIS VAULT] Obsidian Vault + GitHub Pages landing site
│   ├── .obsidian\                         # Obsidian vault settings & graph color groups
│   ├── 00-Home-MOC.md                     # Master Map of Content (Start Here)
│   ├── 01-Architecture-Overview.md        # System Architecture, Lifecycle & Self-Healing
│   ├── 02-Frontend-UI-Architecture.md     # React 19, Theme System & Zustand Stores
│   ├── 03-UI-Components-and-Views.md      # All 6 Views, Domain Components & Form Primitives
│   ├── 04-UI-Wireframes.md                # Full ASCII Wireframes for all Views, Tabs & Modals
│   ├── 05-Rust-Backend-Architecture.md    # Rust Modules, Config Engine, Process Supervisors & OS Layer
│   ├── 06-IPC-Bridge-and-Data-Contracts.md# 49 Tauri Commands, 8 Events & Disk JSON Schemas
│   ├── 07-AI-Agent-Playbook.md            # Invariants, Dual-Store Rules & Modification Recipes
│   ├── 08-User-Guide-and-FAQ.md           # End-User Guide, Platform Quickstarts & Emergency Triage
│   ├── 09-Design-System.md                # Material 3 tokens, component map, icon strategy
│   ├── 99-Vault-Changelog.md              # Revision notes + documentation drift register
│   ├── specs\                             # Spec-driven development: constitution, templates, 001-004
│   ├── wireframes\                        # 00 window/grid, 01-08 per-view, 09 exact-scale fit layouts
│   └── tools\gen_wireframes.py            # Regenerates the exact-scale ASCII wireframes
├── src\                                   # Frontend React + TypeScript Application
│   ├── App.tsx                            # Root lifecycle, event listeners, polling & tab switcher
│   ├── App.css / index.css                # Tailwind v4 + Material 3 CSS variables & utility classes
│   ├── components\
│   │   ├── NavRail.tsx                    # Left 6-tab navigation rail with 72px macOS traffic light inset
│   │   ├── ViewShell.tsx                  # Standardized header + scrollable content wrapper
│   │   ├── TrafficChart.tsx               # Real-time SVG cubic-bezier upload/download area chart
│   │   ├── ToastContainer.tsx             # Floating toast notification stack (max 5)
│   │   ├── ErrorBoundary.tsx              # React crash fallback screen
│   │   ├── domain\NodeEditor.tsx          # Slide-over sheet for adding/editing 10 proxy protocols + detours
│   │   ├── form\                          # FormInput, FormSelect, FormSwitch, SegmentedControl
│   │   └── ui\                            # Button, Card, Badge, Sheet, Table, Switch, Select, Icon
│   ├── stores\                            # 8 Zustand Stores
│   │   ├── connectionStore.ts             # Tunnel status, ports, speeds, PID/CPU/RAM, toggleConnect
│   │   ├── profileStore.ts                # Profiles, nodes, import, diff preview, latency, GeoIP
│   │   ├── routingStore.ts                # Custom routing rules, rule sets & Outbound Groups
│   │   ├── settingsStore.ts               # App settings, theme, ports status, elevation, versions
│   │   ├── nodeEditorStore.ts             # Form state & serializer for NodeEditor slide-over
│   │   ├── statsStore.ts                  # Per-node cumulative upload/download/connection stats
│   │   ├── logStore.ts                    # Ring buffer of 2,000 console/system logs
│   │   └── toastStore.ts                  # Ephemeral toast notifications
│   ├── utils\
│   │   ├── store.ts                       # Tauri plugin-store wrapper, contracts & schema migrations
│   │   ├── applyPolicy.ts                 # Live vs Defer vs Restart setting classification & applier
│   │   ├── connectedApply.ts              # Debounced (800ms) validated cold-restart wiring
│   │   ├── errors.ts                      # Raw Rust/sing-box error -> user-friendly message mapper
│   │   └── tauri.ts / uuid.ts             # Safe Tauri environment detection & UUID generator
│   └── views\                             # 6 Primary Application Screens
│       ├── DashboardView.tsx              # Connect button, telemetry bar, server table, traffic chart
│       ├── ConfigView.tsx                 # Profiles grid, subscription diff modal, import panel
│       ├── RoutingView.tsx                # Routing rules table, Outbound Groups cards, rule sets
│       ├── ConnectionsView.tsx            # Virtualized react-window active sockets table with search/kill
│       ├── LogsView.tsx                   # Virtualized log viewer with tag highlighting & filters
│       └── SettingsView.tsx               # 6-tab settings + encrypted v2 backup export/import modal
└── src-tauri\                             # Rust Native Backend (Tauri v2)
    ├── Cargo.toml                         # Rust dependencies, supervisor targets & Windows features
    ├── tauri.conf.json                    # Window config (1000x680 default, min 720x520, Overlay titleBar on macOS), bundle resources
    ├── binaries\                          # sing-box-x86_64-pc-windows-msvc.exe & wintun.dll
    └── src\
        ├── main.rs / lib.rs               # App builder, setup hook, command registration, exit cleanup
        ├── state.rs                       # ProxyState, SidecarChild, ring buffers, persistent clash_client
        ├── constants.rs                   # File names, ports, DNS defaults, sidecar version constants
        ├── tray.rs                        # Dynamic system tray menu (status, top 10 nodes, toggle)
        ├── bin\
        │   ├── xlink-darwin-supervisor.rs # macOS root companion supervisor binary
        │   └── xlink-linux-supervisor.rs  # Linux unprivileged companion supervisor binary
        ├── os\
        │   ├── mod.rs                     # PlatformNetworkManager trait & TunSupportStatus enum
        │   ├── windows.rs                 # WinINet proxy, wintun.dll, Job Objects, DNS loop prevention
        │   ├── darwin.rs                  # macOS networksetup, scutil DNS, sudoers helper staging
        │   └── linux.rs                   # Linux GSettings/resolv.conf, ~/.local helper, pkexec setcap
        ├── config\
        │   ├── mod.rs                     # Config pipeline & route rules compiler (7 params)
        │   ├── generator.rs               # Builds sing-box JSON (12 params, Outbound Groups, debug log)
        │   ├── adapters\mod.rs            # Parses sing-box JSON, Clash YAML, Base64 & share links
        │   ├── rules.rs                   # OutboundGroup structs, build_singbox_rule (4 params)
        │   ├── subscription.rs            # HTTP fetch with UA fallback & 10MB stream limit
        │   └── validator.rs               # Kahn's DAG cycle validator & sing-box check
        └── commands\
            ├── mod.rs                     # Command module exports
            ├── proxy.rs                   # toggle_proxy, detour validation, ring buffer readers, Clash API
            ├── config.rs                  # Import, profiles, diff preview, node extraction filtering
            ├── system.rs                  # Version gate, port status, check_tun_support_status, autostart
            ├── latency.rs                 # Direct TCP ping & Clash API `/proxies/{tag}/delay` URL test
            ├── geo.rs                     # Consent-gated public IP geolocation (`freeipapi.com`)
            ├── logs.rs                    # Ring-buffer log retrieval (`get_buffered_logs`)
            ├── diagnostics.rs             # Sanitized report with ring buffers & offline routing simulator
            ├── backup.rs                  # Encrypted v2 backup (AES-256-GCM + Argon2id) export, validation & restore
            └── subscription_schedule.rs   # Background 60s ticker for auto-updating subscriptions
```

---

## Active Work & Known Issues

| ID | Issue | Spec | Status |
| :--- | :--- | :--- | :--- |
| ICON-001 | Material icons not shown in the packaged (built) app | [[specs/001-self-hosted-icons/spec]] | Ready — do first |
| WIN-001 | Window size inconsistent across notes; first paint/clamp not guaranteed | [[specs/002-desktop-window-shell/spec]] | Ready |
| FIT-001 | UI does not consistently fit the window; five breakpoint sets | [[specs/003-responsive-fit-layout/spec]] | Ready |
| MUI-001 | Mixed shadcn / Tailwind / Material Web look | [[specs/004-material-web-ui-conformance/spec]] | Ready |

**Start any UI task here:** [[specs/README]] → [[specs/constitution]] → the feature's `spec.md`, `plan.md`, `tasks.md`.

---

## Quick Links for Common AI Tasks

- **Want to understand how the app connects and heals itself?** $\rightarrow$ See [[01-Architecture-Overview#3. Connection Lifecycle & 5-Stage Self-Healing Engine]]
- **Want to inspect or modify a UI screen or component?** $\rightarrow$ See [[03-UI-Components-and-Views]] and [[04-UI-Wireframes]]
- **Want to see how React state is stored and synced with Rust?** $\rightarrow$ See [[02-Frontend-UI-Architecture#3. Zustand State Management Stores]] and [[06-IPC-Bridge-and-Data-Contracts]]
- **Want to modify `sing-box` config generation, TUN, or DNS?** $\rightarrow$ See [[05-Rust-Backend-Architecture#3. Configuration Engine (`src-tauri/src/config/`)]]
- **Want to make code changes without breaking critical invariants?** $\rightarrow$ Read [[07-AI-Agent-Playbook]] first!
- **Want to change how anything looks or fits the window?** $\rightarrow$ [[09-Design-System]], [[wireframes/00-window-and-grid]], then [[specs/README]]
