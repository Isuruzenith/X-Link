---
title: "X-Link Documentation Vault — Master Map of Content (MOC)"
aliases: ["Home", "Index", "MOC", "AI Context"]
tags:
  - moc
  - architecture
  - ai-context
  - xlink
created: 2026-11-03
updated: 2026-11-03
---

# X-Link — Complete AI Agent Knowledge Vault

> [!IMPORTANT] AI Agent Orientation
> This Obsidian vault (`C:\Projects\X-Link\docs`) is the **complete, authoritative architectural and UI/backend context** for **X-Link** (`com.isuru.x-link`, v0.6.0). Any AI coding agent or developer reading this vault can understand every UI screen, React component, Zustand store, Tauri IPC bridge command/event, Rust backend module, `sing-box` sidecar configuration pipeline, and ASCII wireframe without guessing.

---

## Vault Navigation (Map of Content)

| Note | Scope & Contents | Primary Tags |
| :--- | :--- | :--- |
| **[[00-Home-MOC]]** | Master index, quick-reference architecture summary, technology stack, and repository map. | `#moc` `#architecture` |
| **[[01-Architecture-Overview]]** | End-to-end system topology, boot lifecycle, connection lifecycle, 5-stage self-healing engine, transactional config pipeline, and non-negotiable core invariants. | `#architecture` `#backend` `#frontend` |
| **[[02-Frontend-UI-Architecture]]** | React 19 + TypeScript + Tailwind CSS v4 + Material Web (`@material/web`) styling system, theme tokens, `App.tsx` orchestration, and all **8 Zustand stores** (`connectionStore`, `profileStore`, `routingStore`, `settingsStore`, `nodeEditorStore`, `statsStore`, `logStore`, `toastStore`). | `#frontend` `#state-management` |
| **[[03-UI-Components-and-Views]]** | Exhaustive component reference: layout shell (`NavRail`, `ViewShell`, `ToastContainer`, `ErrorBoundary`), domain editors (`NodeEditor`, `TrafficChart`), form controls (`FormInput`, `FormSelect`, `FormSwitchRow`, `SegmentedControl`), UI primitives, and all **6 main views** (`DashboardView`, `ConfigView`, `RoutingView`, `ConnectionsView`, `LogsView`, `SettingsView`). | `#frontend` `#components` `#ui` |
| **[[04-UI-Wireframes]]** | Detailed **ASCII wireframes** and state matrices for every screen, tab, drawer, modal, and banner in X-Link. | `#wireframe` `#ui` `#frontend` |
| **[[05-Rust-Backend-Architecture]]** | Deep dive into `src-tauri`: `ProxyState`, `SingBox` sidecar management (`v1.13.14` + `wintun.dll`), config generator (`generator.rs`), multi-format parser (`adapters/mod.rs`), validator (`validator.rs`), routing rule builder (`rules.rs`), Windows OS integration (`os/mod.rs`), system tray (`tray.rs`), diagnostics, and backup/restore. | `#backend` `#rust` `#sing-box` |
| **[[06-IPC-Bridge-and-Data-Contracts]]** | Complete contract dictionary: all **38 Tauri `invoke` commands** (arguments, return types, side effects), all **7 Tauri events** (`emit` -> `listen`), Clash REST API (`127.0.0.1:9090`) integration, and on-disk JSON schemas (`settings.json`, `profiles.json`, `routing.json`, `stats.json`, `configs/active.json`). | `#ipc` `#contracts` `#backend` `#frontend` |
| **[[07-AI-Agent-Playbook]]** | Practical engineering guide for AI agents: 7 non-negotiable invariants, Dual-Store Synchronization Rule (`@tauri-apps/plugin-store` vs Rust `std::fs`), Connected-Apply Policy (`applyPolicy.ts`), and step-by-step recipes for adding settings, protocols, or IPC commands. | `#ai-context` `#playbook` `#architecture` |

---

## Executive Summary of X-Link

**X-Link** is a privacy-first, native desktop VPN and proxy client built on **Tauri v2** (Rust backend + WebView2/WebKit frontend) wrapping the **sing-box v1.13.14** universal proxy platform.

### Core Capabilities
1. **Dual Proxy Modes**:
   - **TUN Mode (`proxyMode: "tun"`)**: Captures all system TCP/UDP/ICMP traffic via a virtual network interface (`wintun.dll` on Windows, `gvisor` / `mixed` / `system` stack) with `FakeIP` (`198.18.0.0/15`), strict route leak protection, and locale-independent Windows IP Helper API (`GetAdaptersAddresses`) DNS loop prevention.
   - **System Proxy Mode (`proxyMode: "system"`)**: Configures OS-level HTTP/SOCKS/Mixed listeners (`7890` / `7891` / `7892`) and manages Windows `WinINet` registry settings (`HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings`) with automatic snapshot & crash-recovery restoration (`ProxyStateManager`).
2. **Universal Protocol & Subscription Support**:
   - **Outbound Protocols**: `VLESS` (with XTLS-Reality & uTLS), `VMess`, `Trojan`, `Shadowsocks`, `Hysteria2`, `TUIC v5`, `WireGuard`, `SSH`, `SOCKS5`, and `HTTP`.
   - **Import Formats**: Native `sing-box` JSON, Clash/Mihomo YAML, Base64-encoded subscription bundles, raw share links (`vless://`, `vmess://`, `trojan://`, `ss://`, `hysteria2://`, `hy2://`, `tuic://`, `wg://`), and remote HTTP/HTTPS subscriptions with **Subscription Diff Previews** and **Background Scheduled Auto-Updates**.
3. **5-Stage Self-Healing Connection Engine**:
   - Automatically recovers from TLS fingerprint blocks, DNS poisoning, or TUN driver failures during connection startup (`Chrome uTLS` $\rightarrow$ `Firefox uTLS` $\rightarrow$ `Native Go TLS` $\rightarrow$ `Public DNS Bootstrap` $\rightarrow$ `TUN Compatibility Profile` $\rightarrow$ `System Proxy Fallback`).
4. **Hot Node Switching & Validated Cold Restart**:
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
│   ├── 05-Rust-Backend-Architecture.md    # Rust Modules, Config Engine, Sidecar & OS Layer
│   ├── 06-IPC-Bridge-and-Data-Contracts.md# 38 Tauri Commands, 7 Events & Disk JSON Schemas
│   └── 07-AI-Agent-Playbook.md            # Invariants, Dual-Store Rules & Modification Recipes
├── src\                                   # Frontend React + TypeScript Application
│   ├── App.tsx                            # Root lifecycle, event listeners, polling & tab switcher
│   ├── App.css / index.css                # Tailwind v4 + Material 3 CSS variables & utility classes
│   ├── components\
│   │   ├── NavRail.tsx                    # Left 6-tab navigation rail + status footer
│   │   ├── ViewShell.tsx                  # Standardized header + scrollable content wrapper
│   │   ├── TrafficChart.tsx               # Real-time SVG cubic-bezier upload/download area chart
│   │   ├── ToastContainer.tsx             # Floating toast notification stack (max 5)
│   │   ├── ErrorBoundary.tsx              # React crash fallback screen
│   │   ├── domain\NodeEditor.tsx          # Slide-over sheet for adding/editing 10 proxy protocols
│   │   ├── form\                          # FormInput, FormSelect, FormSwitch, SegmentedControl
│   │   └── ui\                            # Button, Card, Badge, Sheet, Table, Switch, Select, Icon
│   ├── stores\                            # 8 Zustand Stores
│   │   ├── connectionStore.ts             # Tunnel status, ports, speeds, PID/CPU/RAM, toggleConnect
│   │   ├── profileStore.ts                # Profiles, nodes, import, diff preview, latency, GeoIP
│   │   ├── routingStore.ts                # Custom routing rules & remote/local rule sets
│   │   ├── settingsStore.ts               # App settings, theme, ports status, elevation, versions
│   │   ├── nodeEditorStore.ts             # Form state & serializer for NodeEditor slide-over
│   │   ├── statsStore.ts                  # Per-node cumulative upload/download/connection stats
│   │   ├── logStore.ts                    # Ring buffer of 2,000 console/system logs
│   │   └── toastStore.ts                  # Ephemeral toast notifications
│   ├── utils\
│   │   ├── store.ts                       # Tauri plugin-store wrapper, types & schema migrations
│   │   ├── applyPolicy.ts                 # Live vs Defer vs Restart setting classification & applier
│   │   ├── connectedApply.ts              # Debounced (800ms) validated cold-restart wiring
│   │   ├── errors.ts                      # Raw Rust/sing-box error -> user-friendly message mapper
│   │   └── tauri.ts / uuid.ts             # Safe Tauri environment detection & UUID generator
│   └── views\                             # 6 Primary Application Screens
│       ├── DashboardView.tsx              # Connect button, telemetry bar, server table, traffic chart
│       ├── ConfigView.tsx                 # Profiles grid, subscription diff modal, import panel
│       ├── RoutingView.tsx                # Routing rules table, rule sets, process picker
│       ├── ConnectionsView.tsx            # Real-time active sockets table with search/filter/kill
│       ├── LogsView.tsx                   # Virtualized log viewer with tag highlighting & filters
│       └── SettingsView.tsx               # 6-tab settings (General, TUN, Sniff, DNS, API, Diagnostics)
└── src-tauri\                             # Rust Native Backend (Tauri v2)
    ├── Cargo.toml                         # Rust dependencies & Windows feature flags
    ├── tauri.conf.json                    # Window config (1080x700), bundle resources, sidecar
    ├── binaries\                          # sing-box-x86_64-pc-windows-msvc.exe & wintun.dll
    └── src\
        ├── main.rs / lib.rs               # App builder, setup hook, command registration, exit cleanup
        ├── state.rs                       # ProxyState, ProxyStatus, PreviewedSubscription, LogBuffer
        ├── constants.rs                   # File names, ports, DNS defaults, sidecar version constants
        ├── tray.rs                        # Dynamic system tray menu (status, top 10 nodes, toggle)
        ├── os\mod.rs                      # WinINet system proxy, UAC elevation, IpHlpApi DNS, port owner
        ├── config\
        │   ├── mod.rs                     # Config pipeline exports
        │   ├── generator.rs               # Builds final sing-box JSON (Inbounds, Outbounds, DNS, Route)
        │   ├── adapters\mod.rs            # Parses sing-box JSON, Clash YAML, Base64 & share links
        │   ├── rules.rs                   # Compiles user routing rules, rule-sets & LAN bypass
        │   ├── subscription.rs            # HTTP fetch with UA fallback & 10MB stream limit
        │   └── validator.rs               # Runs `sing-box check -c <temp.json>` before commit
        └── commands\
            ├── mod.rs                     # Command module exports
            ├── proxy.rs                   # toggle_proxy, 5-stage self-healing, hot switch, Clash API
            ├── config.rs                  # Import, profiles, diff preview, transactional commit/rollback
            ├── system.rs                  # Version gate, port conflict/status, UAC, autostart, processes
            ├── latency.rs                 # Direct TCP ping & Clash API `/proxies/{tag}/delay` URL test
            ├── geo.rs                     # Consent-gated public IP geolocation (`freeipapi.com`)
            ├── logs.rs                    # Ring-buffer log retrieval (`get_buffered_logs`)
            ├── diagnostics.rs             # Sanitized system report & offline routing/DNS simulator
            ├── backup.rs                  # Full JSON backup export, validation & atomic restore
            └── subscription_schedule.rs   # Background 60s ticker for auto-updating subscriptions
```

---

## Quick Links for Common AI Tasks

- **Want to understand how the app connects and heals itself?** $\rightarrow$ See [[01-Architecture-Overview#3. Connection Lifecycle & 5-Stage Self-Healing Engine]]
- **Want to inspect or modify a UI screen or component?** $\rightarrow$ See [[03-UI-Components-and-Views]] and [[04-UI-Wireframes]]
- **Want to see how React state is stored and synced with Rust?** $\rightarrow$ See [[02-Frontend-UI-Architecture#3. Zustand State Management Stores]] and [[06-IPC-Bridge-and-Data-Contracts]]
- **Want to modify `sing-box` config generation, TUN, or DNS?** $\rightarrow$ See [[05-Rust-Backend-Architecture#3. Configuration Engine (`src-tauri/src/config/`)]]
- **Want to make code changes without breaking critical invariants?** $\rightarrow$ Read [[07-AI-Agent-Playbook]] first!
