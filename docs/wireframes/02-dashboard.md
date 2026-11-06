---
title: "Wireframe 02 — Dashboard View"
aliases: ["Dashboard Wireframe", "Dashboard Screen", "Connection Dashboard"]
tags:
  - wireframe
  - ui
  - dashboard
  - connection
created: 2026-11-03
updated: 2026-11-05
---

# Wireframe 02 — Dashboard View

> [!NOTE] Target geometry
> Breakpoint tables in this note describe the **current CSS**. The target is three size classes (Compact `<600`, Medium `600–839`, Expanded `≥840`) in a **1000×680** default / **720×520** minimum window with zero document scroll — see [[wireframes/00-window-and-grid]] (rules, migration map) and [[wireframes/09-fit-layouts]] (exact-scale drawing for this view). Work is tracked in [[specs/003-responsive-fit-layout/tasks]].

**Navigation**: [[00-Home-MOC]] | **Related Wireframes**: [[01-shell-navigation]] · [[03-profiles]] · [[04-node-editor]]

**Source Components**:
- Primary View: [DashboardView.tsx](../../src/views/DashboardView.tsx)
- Header/Viewport Container: [ViewShell.tsx](../../src/components/ViewShell.tsx)
- Real-time SVG Traffic Chart: [TrafficChart.tsx](../../src/components/TrafficChart.tsx)
- State Stores: [connectionStore.ts](../../src/stores/connectionStore.ts) · [profileStore.ts](../../src/stores/profileStore.ts) · [settingsStore.ts](../../src/stores/settingsStore.ts)
- Structural CSS: [material-theme.css](../../src/material-theme.css)

---

## 1. Grid Architecture & Responsive Breakpoints

The Dashboard layout (`.dashboard-layout`) organizes connection controls, real-time throughput metrics, server selection chips, and diagnostic data in a 2-column CSS Grid:

| Breakpoint Tier | Container Width | Layout Columns | Details Placement | Card Padding |
| :--- | :--- | :--- | :--- | :--- |
| **Desktop Wide** | `> 1140px` | `minmax(280px, .85fr) minmax(340px, 1.35fr)` | 4-column metric grid | `padding: 24px - 25px;` |
| **Desktop Standard** | `851px – 1140px` | `minmax(260px, 1fr) minmax(340px, 1.25fr)` | `.dashboard-details { grid-column: 1 / -1; }` | `padding: 24px;` |
| **Tablet / Narrow** | `≤ 850px` | `minmax(0, 1fr)` (single vertical column) | `.dashboard-details { grid-column: auto; }` | `padding: 24px;` |
| **Mobile Narrow** | `≤ 700px` | Single column | `.dashboard-details { grid-template-columns: repeat(2, 1fr); }` | `padding: 19px;` |

---

## 2. Desktop Connected State (`> 1050px`)

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Dashboard                                                                                           (●) Connected     |
|  Your connection at a glance                                                                                           |
+------------------------------------------------------------------------------------------------------------------------+
|                                                                                                                        |
|  +----------------------------------------------+  +----------------------------------------------------------------+  |
|  |  (⏻)                            [TUN routing]|  |  Live traffic                                            [Live]|  |
|  |                                              |  |  Last 30 seconds · bytes per second                            |  |
|  |  Connected                                   |  |                                                                |  |
|  |  Traffic is routed through US-West-Reality.  |  |  (↓) Download                  (↑) Upload                      |  |
|  |                                              |  |  14.2 MB/s                     1.8 MB/s                        |  |
|  |  +----------------------------------------+  |  |                                                                |  |
|  |  | [   Disconnect                       ] |  |  |  20 MB/s |             ╭───────╮                                |  |
|  |  +----------------------------------------+  |  |           |     ╭───────╯       ╰───╮                            |  |
|  |                                              |  |   0 B/s   +─────────────────────────● (TrafficChart.tsx)       |  |
|  |  [✓] Session active · 01:24:10               |  |  > View traffic data as a table                                |  |
|  +----------------------------------------------+  +----------------------------------------------------------------+  |
|                                                                                                                        |
|  +------------------------------------------------------------------------------------------------------------------+  |
|  |  Active server                                                                                  [dns]            |  |
|  |  Premium Subscription                                                                                            |  |
|  |                                                                                                                  |  |
|  |  [Router]  US-West-Reality                                                                                       |  |
|  |            198.51.100.42:443                                                                                     |  |
|  |                                                                                                                  |  |
|  |  Switch server:                                                                                                  |  |
|  |  ( US-West-Reality )*  ( SG-Direct-Hy2 )  ( JP-Tokyo-Trojan )  ( Frankfurt-WG )  [View all 12 servers]           |  |
|  +------------------------------------------------------------------------------------------------------------------+  |
|                                                                                                                        |
|  +------------------------------------------------------------------------------------------------------------------+  |
|  |  (↓) Downloaded        | (↑) Uploaded          | (🕒) Uptime         | (🖧) Connections      | (🧠) Core          |  |
|  |  1.42 GB               | 184.5 MB              | 01:24:10            | [ 42 active        ]  | 38.2 MB · 1.4%     |  |
|  +------------------------------------------------------------------------------------------------------------------+  |
|                                                                                                                        |
|  +------------------------------------------------------------------------------------------------------------------+  |
|  |  Inbound listeners   [Mixed 7892]  [API 9090]  [LAN sharing on]                                     [ Settings ] |  |
|  +------------------------------------------------------------------------------------------------------------------+  |
|                                                                                                                        |
+------------------------------------------------------------------------------------------------------------------------+
```

---

## 3. Desktop Disconnected & Connecting Variations

### 3.1 Disconnected (No Profile Saved)
When the application starts with no imported profiles:

```text
+----------------------------------------------+  +----------------------------------------------------------------+
|  (⏻)                   [TUN preferred]       |  |  Live traffic                                            [Idle]|
|                                              |  |  Last 30 seconds · bytes per second                            |
|  Disconnected                                |  |                                                                |
|  Add a profile to start connecting.          |  |  (↓) Download                  (↑) Upload                      |
|                                              |  |  0 B/s                         0 B/s                           |
|  +----------------------+ +----------------+ |  |                                                                |
|  | [ Connect (disabled)]| | [Add a profile]| |  |   0 B/s   +---------------------------------------------------- |
|  +----------------------+ +----------------+ |  |  > View traffic data as a table                                |
|                                              |  +----------------------------------------------------------------+
|  [✓] Ready to connect                        |  +----------------------------------------------------------------+
+----------------------------------------------+  |  Active server                                           [dns] |
                                                  |  No profile selected                                           |
                                                  |                                                                |
                                                  |  Import a profile to see available servers.                    |
                                                  +----------------------------------------------------------------+
```

### 3.2 Disconnected (Profile Available, Admin Elevation Required)
When TUN mode is preferred on Windows/macOS/Linux without root/elevated privileges:

```text
+----------------------------------------------+
|  (⏻)                   [TUN preferred]       |
|                                              |
|  Disconnected                                |
|  Connect to route traffic through your       |
|  selected server.                            |
|                                              |
|  +----------------------------------------+  |
|  | [ Connect                            ] |  |
|  +----------------------------------------+  |
|                                              |
|  [!] Run as administrator for TUN            |  <- (dashboard-admin-action: requests UAC / root elevation)
+----------------------------------------------+
```

### 3.3 Connecting State (Busy Handshake)
When `connectionStatus === 'connecting'`, UI controls disable and an animated spinner is displayed:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Dashboard                                                                                          (◐) Connecting     |
|  Your connection at a glance                                                                                           |
+------------------------------------------------------------------------------------------------------------------------+
|  +----------------------------------------------+
|  |  (⏻)                   [TUN preferred]       |
|  |                                              |
|  |  Connecting                                  |
|  |  Starting the proxy and checking your        |
|  |  connection…                                 |
|  |                                              |
|  |  +----------------------------------------+  |
|  |  | [ ⟳ Connecting… (disabled)            ] |  |  <- (.button-spinner inline animation)
|  |  +----------------------------------------+  |
|  |                                              |
|  |  [✓] Ready to connect                        |
|  +----------------------------------------------+
```

### 3.4 Connected with Fallback to System Proxy
When the user preferred TUN mode, but driver initialization failed, the engine gracefully falls back to system proxy without aborting:

```text
+----------------------------------------------+
|  (⏻)                    [System proxy]       |
|                                              |
|  Connected                                   |
|  Traffic is routed through US-West-Reality.  |
|  TUN was unavailable. This session is        |  <- (.mode-fallback warning banner)
|  using System Proxy instead.                 |
|                                              |
|  +----------------------------------------+  |
|  | [ Disconnect                         ] |  |
|  +----------------------------------------+  |
|                                              |
|  [✓] Session active · 00:04:12               |
+----------------------------------------------+
```

---

## 4. Narrow Viewport Layout (`≤ 700px`)

In narrow windows, cards stack vertically into a single scrollable container (`.content-grid-scroll`), and the details card switches to a 2×2 / 2×3 grid:

```text
+--------------------------------------------------------------------------------+
|  [ ☰ ]   Dashboard                                          (●) Connected      |
|          Your connection at a glance                                           |
+--------------------------------------------------------------------------------+
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  (⏻)                                                        [TUN routing]|  |
|  |  Connected                                                               |  |
|  |  Traffic is routed through US-West-Reality.                              |  |
|  |                                                                          |  |
|  |  +--------------------------------------------------------------------+  |  |
|  |  | [ Disconnect                                                     ] |  |  |
|  |  +--------------------------------------------------------------------+  |  |
|  |  [✓] Session active · 01:24:10                                           |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  Live traffic                                                      [Live]|  |
|  |  Last 30 seconds · bytes per second                                      |  |
|  |                                                                          |  |
|  |  (↓) Download: 14.2 MB/s               (↑) Upload: 1.8 MB/s              |  |
|  |                                                                          |  |
|  |  +────────────────────────────────────────────────────────────────────+  |  |
|  |  |                   SVG TRAFFIC AREA CHART                           |  |  |
|  |  +────────────────────────────────────────────────────────────────────+  |  |
|  |  > View traffic data as a table                                          |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  Active server                                                     [dns] |  |
|  |  Premium Subscription                                                    |  |
|  |  [Router] US-West-Reality (198.51.100.42:443)                            |  |
|  |  [US-West-Reality]* [SG-Direct-Hy2] [JP-Tokyo-Trojan] [Frankfurt-WG]...  |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  (↓) Downloaded        | (↑) Uploaded                                    |  |
|  |  1.42 GB               | 184.5 MB                                        |  |
|  |------------------------+-------------------------------------------------|  |
|  |  (🕒) Uptime           | (🖧) Connections                                |  |
|  |  01:24:10              | [ 42 active ]                                   |  |
|  |------------------------+-------------------------------------------------|  |
|  |  (🧠) Core                                                               |  |
|  |  38.2 MB · 1.4%                                                          |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  Inbound listeners                                                       |  |
|  |  [Mixed 7892]  [API 9090]                                   [ Settings ] |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
+--------------------------------------------------------------------------------+
```

---

## 5. UI Elements & Accurate Labels

| Element Selector | Label / Display Text | Dynamic Behavior & Sources |
| :--- | :--- | :--- |
| `.dashboard-status` | `Connected` / `Connecting` / `Disconnected` | Live connection status pill in `ViewShell` action slot. |
| `.dashboard-connect-mode` | `TUN routing` / `System proxy` / `TUN preferred` / `System proxy preferred` | Reflects `effectiveProxyMode` when connected, or `settings.proxyMode` when disconnected. |
| `.dashboard-primary-action` | `Connect` / `Connecting…` / `Disconnect` | Triggers `toggleConnect()`. Disabled when busy or when no profile is present. |
| `.dashboard-admin-action` | `Run as administrator for TUN` | Shown only when `proxyMode === 'tun' && !isElevated && !connected`. Calls `requestElevation()`. |
| `.live-tag` | `Live` (connected) / `Idle` (disconnected) | Signals whether telemetry loop is polling live stats. |
| `.speed-key.down` | `Download` + tabular speed (e.g. `14.2 MB/s`) | Current download rate formatted via `formatSpeed(downloadSpeed)`. |
| `.speed-key.up` | `Upload` + tabular speed (e.g. `1.8 MB/s`) | Current upload rate formatted via `formatSpeed(uploadSpeed)`. |
| `.server-choice` | Node tag (e.g. `US-West-Reality`) | Displays up to 8 servers; clicking calls `selectNode(node)` for instant hot-switching. |
| `.dashboard-detail.detail-link`| `Connections` + `{activeConnections} active` | Interactive button navigating directly to the Connections monitor tab. |
| `.dashboard-inbounds` | `Inbound listeners` + port chips | Displays active ports: `Mixed`, `HTTP`, `SOCKS5`, optional `API`, and `LAN sharing on`. |

---

## 6. Interaction & State Transitions

1. **Connection Lifecycle (`toggleConnect`)**:
   - Disconnected $\rightarrow$ user clicks "Connect" $\rightarrow$ state becomes `connecting`.
   - Sidecar process `sing-box` spawns; backend validates ports and configuration.
   - On success: status becomes `connected`, timer increments `uptime`, traffic chart starts streaming.
   - On error: status resets to `disconnected`, error toast is raised.
2. **Instant Hot-Node Switching**:
   - While connected, selecting any server chip (`.server-choice`) immediately calls Clash API (`PUT /proxies/proxy`).
   - If REST API is enabled, traffic routes through the new server with **zero process restart**.
3. **Cross-Tab Deep Linking**:
   - Clicking `"Add a profile"` navigates to `profiles` tab.
   - Clicking `"Connections"` metric button navigates to `connections` tab.
   - Clicking `"Settings"` inbound button navigates to `settings` tab.
