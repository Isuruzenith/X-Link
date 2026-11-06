---
title: "Wireframe 06 — Active Connections Monitor"
aliases: ["Connections Wireframe", "ConnectionsView Wireframe", "Socket Monitor"]
tags:
  - wireframe
  - ui
  - connections
  - sockets
created: 2026-11-03
updated: 2026-11-05
---

# Wireframe 06 — Active Connections Monitor

> [!NOTE] Target geometry
> Breakpoint tables in this note describe the **current CSS**. The target is three size classes (Compact `<600`, Medium `600–839`, Expanded `≥840`) in a **1000×680** default / **720×520** minimum window with zero document scroll — see [[wireframes/00-window-and-grid]] (rules, migration map) and [[wireframes/09-fit-layouts]] (exact-scale drawing for this view). Work is tracked in [[specs/003-responsive-fit-layout/tasks]].

**Navigation**: [[00-Home-MOC]] | **Related Wireframes**: [[01-shell-navigation]] · [[02-dashboard]] · [[05-routing]] · [[07-logs]]
**Source Components**:
- Primary View: [ConnectionsView.tsx](../../src/views/ConnectionsView.tsx)
- Container Shell: [ViewShell.tsx](../../src/components/ViewShell.tsx)
- Stores: [connectionStore.ts](../../src/stores/connectionStore.ts) · [toastStore.ts](../../src/stores/toastStore.ts)
- Native IPC: `get_active_connections` · `close_connection` · `close_all_connections`
- Dialog Plugin: `@tauri-apps/plugin-dialog` (`ask`)
- Form Controls: [FormInput.tsx](../../src/components/form/FormInput.tsx) · [SegmentedControl.tsx](../../src/components/form/SegmentedControl.tsx)
- Structural CSS: [material-theme.css](../../src/material-theme.css)

---

## 1. Grid Architecture & Responsive Breakpoints

The Connections monitor (`.utility-view`) displays real-time socket telemetry, filtering controls, sortable data tables, and an inspection drawer:

| Breakpoint Tier | Container Width | Toolbar Layout | Table Container | Detail Panel (`dl`) |
| :--- | :--- | :--- | :--- | :--- |
| **Desktop Wide** | `> 1050px` | Inline flex (`gap: 10px`), right socket counter | `.utility-table` (min 760px) | 2-column grid (`sm:grid-cols-2`) |
| **Tablet / Narrow** | `701px – 1050px` | Toolbar wraps; socket counter hides | Horizontal scroll with sticky head | 2-column grid |
| **Mobile Narrow** | `≤ 700px` | Search takes full row; segments wrap | Horizontal scroll with thumb bar | Single column stacked |
| **Extreme Narrow** | `≤ 440px` | Controls fully stacked vertically | Horizontal scroll container | Single column stacked |

---

## 2. Disconnected State (`!isConnected`)

When the proxy core is idle, interactive filter controls are disabled and a status card prompts navigation to the Dashboard:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Connections                                                                                                           |
|  See active destinations, routing decisions and transferred data                                                       |
+------------------------------------------------------------------------------------------------------------------------+
|  [🔍 Search connections (disabled)     ]  [ All Tunnels (disabled) | Proxy Tunnels | Direct ]                          |
+------------------------------------------------------------------------------------------------------------------------+
|                                                                                                                        |
|                                       +----------------------------------------+                                       |
|                                       |                  (🖧)                  |                                       |
|                                       |                                        |                                       |
|                                       |  Proxy service is disconnected.        |                                       |
|                                       |  Connect to a server from the          |                                       |
|                                       |  Dashboard to monitor live network     |                                       |
|                                       |  connections and throughput.           |                                       |
|                                       |                                        |                                       |
|                                       |  [ ⚡ Go to Dashboard ]                |                                       |
|                                       +----------------------------------------+                                       |
|                                                                                                                        |
+------------------------------------------------------------------------------------------------------------------------+
```

---

## 3. Desktop Connected State (`> 1050px`)

When running, the 1-second polling loop streams per-connection upload/download rates, accumulated volume, and route chains:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Connections                                                                                           [ ⚠ Close All ] |
|  See active destinations, routing decisions and transferred data                                                       |
+------------------------------------------------------------------------------------------------------------------------+
|  [🔍 Host, IP or rule…                 ]  [ All Tunnels ]* [ Proxy Tunnels ] [ Direct Tunnels ]   2 / 2 active sockets |
+------------------------------------------------------------------------------------------------------------------------+
|  DESTINATION ⇅           NETWORK / MATCHED RULE   LIVE SPEED ⇅     TOTAL DATA ⇅     UPTIME ⇅    KILL                   |
|------------------------------------------------------------------------------------------------------------------------|
|  api.example.com:443      [ PROXY › US-WEST ]      ↓ 1.2 MB/s       ↓ 18.4 MB        4m 12s      [ ✕ ]                  |
|  [TCP] type: tun-in      rule: final              ↑ 42.0 KB/s      ↑ 640.0 KB                                          |
|------------------------------------------------------------------------------------------------------------------------|
|  192.168.1.1:53          [ DIRECT ]               ↓ 0 B/s          ↓ 1.2 KB         18s         [ ✕ ]                  |
|  [UDP] type: tun-in      rule: geoip=private      ↑ 0 B/s          ↑ 420 B                                             |
+------------------------------------------------------------------------------------------------------------------------+
```

---

## 4. Connection Details Drawer & Filter Zero States

### 4.1 Selected Connection Details Panel
Clicking any destination button mounts the detail inspector card (`aria-label="Selected connection details"`) below the table:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Connection details                                                                                              [ ✕ ] |
|------------------------------------------------------------------------------------------------------------------------|
|  Destination                                            Source                                                         |
|  api.example.com:443                                     127.0.0.1:54321                                                |
|                                                                                                                        |
|  Route                                                  Matched rule                                                   |
|  proxy › US-West-Reality                                domain_suffix                                                  |
+------------------------------------------------------------------------------------------------------------------------+
```

### 4.2 Empty Query Results State
When query filters match zero sockets, an empty prompt directs filter adjustment:
```text
+------------------------------------------------------------------------------------------------------------------------+
|                                                          (🖧)                                                          |
|                                           No active network connections found.                                         |
|                                            Try modifying your filter query.                                            |
+------------------------------------------------------------------------------------------------------------------------+
```

---

## 5. Narrow Viewport Layout (`≤ 700px`)

```text
+--------------------------------------------------------------------------------+
|  [ ☰ ]  Connections                                            [ ⚠ Close All ] |
|         See active destinations, routing decisions and transferred data        |
+--------------------------------------------------------------------------------+
|  [🔍 Host, IP or rule…                                                       ] |
|  [ All Tunnels ]*  [ Proxy Tunnels ]  [ Direct Tunnels ]                       |
+--------------------------------------------------------------------------------+
|  [ <------------------ HORIZONTAL SCROLL AREA ------------------> ]            |
|  DESTINATION ⇅       NETWORK / ROUTE   LIVE SPEED ⇅   TOTAL DATA ⇅   TIME ⇅  KILL|
|  api.example.com:443  [PROXY›US-WEST]   ↓ 1.2 MB/s     ↓ 18.4 MB      4m 12s  [✕] |
|  [TCP] type: tun-in  rule: final       ↑ 42.0 KB/s    ↑ 640.0 KB                 |
+--------------------------------------------------------------------------------+
|  Connection details                                                      [ ✕ ] |
|  Destination: api.example.com:443                                               |
|  Source:      127.0.0.1:54321                                                  |
|  Route:       proxy › US-West-Reality                                          |
|  Rule:        domain_suffix                                                    |
+--------------------------------------------------------------------------------+
```

---

## 6. UI Elements & Accurate Labels

| Element Selector / Event | UI Label / Display Text | Description & State Binding |
| :--- | :--- | :--- |
| `Button[handleKillAll]` | `⚠ Close All` | Prompts native modal `Close all active network connections?`; calls `close_all_connections`. |
| `FormInput[searchQuery]` | `Search connections` (label) / `Host, IP or rule…` | Filters live list against destination host, destination IP, or routing rule string. |
| `SegmentedControl[filterType]` | `All Tunnels` / `Proxy Tunnels` / `Direct Tunnels` | Filters list by outbound chain matching `isDirect`. |
| `TableCell[Destination]` | Button `{host}:{port}` + Badges (`network`, `type`) | Clicking row toggles selection state `selectedId`. |
| `TableCell[Live speed]` | `↓ {downSpeed}` / `↑ {upSpeed}` | Computed delta speed formatted via `formatSpeed(bps)`. |
| `TableCell[Total data]` | `↓ {download}` / `↑ {upload}` | Cumulative transferred payload formatted via `formatBytes(bytes)`. |
| `TableCell[Kill]` | `Close connection to {host}:{port}` | Calls `close_connection(id)`; displays spinning icon `[ ⟳ ]` during in-flight teardown. |
| `Card[role="region"]` | `Connection details` | Inspection card displaying destination, source socket, route path, and rule name. |
