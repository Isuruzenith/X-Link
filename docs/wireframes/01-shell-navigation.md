---
title: "Wireframe 01 — Master Application Shell & Navigation"
aliases: ["Shell Wireframe", "NavRail Wireframe", "Navigation Layout"]
tags:
  - wireframe
  - ui
  - layout
  - navigation
created: 2026-11-03
updated: 2026-11-05
---

# Wireframe 01 — Master Application Shell & Navigation

**Navigation**: [[00-Home-MOC]] | **Related Wireframes**: [[02-dashboard]] · [[03-profiles]] · [[04-node-editor]]

**Source Components**:
- Shell Host: [App.tsx](../../src/App.tsx)
- Navigation Rail: [NavRail.tsx](../../src/components/NavRail.tsx)
- View Shell Header/Body: [ViewShell.tsx](../../src/components/ViewShell.tsx)
- Notifications: [ToastContainer.tsx](../../src/components/ToastContainer.tsx)
- Resilience: [ErrorBoundary.tsx](../../src/components/ErrorBoundary.tsx)
- Structural CSS: [material-theme.css](../../src/material-theme.css)

---

## 1. Window Geometry & Responsive Breakpoints

X-Link runs inside a Tauri v2 window configured at **950 × 650 px** (minimum **380 × 550 px**). The diagrams show the webview client area; the native window caption is OS-provided. The CSS adapts across three viewport tiers:

| Breakpoint Tier | Window Width | Navigation Layout | Padding & Gaps | View Host Styling |
| :--- | :--- | :--- | :--- | :--- |
| **Desktop Wide** | `> 1050px` | Fixed Left Rail (`212px`) with labels and brand copy | `padding: 0 18px 18px 0; gap: 12px;` | `border-radius: 0 0 24px 24px; background: var(--md-panel);` |
| **Tablet / Medium** | `701px – 1050px` | Collapsed Left Rail (`64px`), icon-only, labels hidden | `padding: 0 10px 10px 0; gap: 6px;` | Same as desktop |
| **Narrow / Mobile** | `≤ 700px` | Off-canvas Drawer (`min(290px, 82vw)`), top-left floating menu button | `padding: 0; gap: 0;` | `border-radius: 0;` (full flush viewport) |

---

## 2. Desktop Shell Layout (`> 1050px`)

```text
+------------------------------------------------------------------------------------------------------------------------+
| [X] X-Link                                                                                               [-] [□] [✕]   |
+------------------------+-----------------------------------------------------------------------------------------------+
|  [Logo] X-Link         |  VIEW HEADER (.view-heading)                                                                  |
|         Network control|  Title                                                              [Header Action Items...]  |
|                        |  Subtitle copy                                                                                |
|  NAV DESTINATIONS      +-----------------------------------------------------------------------------------------------+
|  +------------------+  |                                                                                               |
|  | [*] Dashboard    |  |                                                                                               |
|  +------------------+  |                                                                                               |
|  | [ ] Profiles     |  |                                                                                               |
|  +------------------+  |                                                                                               |
|  | [ ] Routing      |  |                                      VIEW CONTENT VIEWPORT                                    |
|  +------------------+  |                                         (.view-content)                                       |
|  | [ ] Connections  |  |                                                                                               |
|  +------------------+  |                                                                                               |
|  | [ ] Logs         |  |                                                                                               |
|  +------------------+  |                                                                                               |
|  | [ ] Settings     |  |                                                                                               |
|  +------------------+  |                                                                                               |
|                        |                                                                                               |
|                        |                                                                                               |
|                        |                                                                                               |
|                        |                                                           +---------------------------------+ |
|  +------------------+  |                                                           | [✓] Connection Established  [x] | |
|  | (●) Connected    |  |                                                           | Secured via Mixed port 7892     | |
|  |     Proxy session|  |                                                           +---------------------------------+ |
|  +------------------+  |                                                                         (.toast-container)    |
+------------------------+-----------------------------------------------------------------------------------------------+
```

### Component Structure:
- **Root Element** (`.app-root`): Flex container with full viewport height (`h-screen overflow-hidden relative`), canvas background (`var(--md-canvas)`).
- **Navigation Aside** (`.app-navigation`):
  - Brand block (`.nav-brand`): `39x39px` app logo, `strong` ("X-Link"), `span` ("Network control").
  - Navigation links (`.nav-destinations`): Vertical list of 6 button destinations.
  - Active tab indicator (`.nav-destination.is-active`): Highlighted container with `var(--md-sys-color-secondary-container)` background and bold typography.
  - Session footer (`.nav-session`): Status pill indicator (`.session-indicator`) with status text and label.
- **View Host** (`.view-host.view-fade`): Houses the lazy-loaded view matching `activeTab`, wrapped in `<Suspense>` and `<ErrorBoundary>`.
- **Slide-Over Sheet** (`<NodeEditor />`): Global overlay drawer, conditionally rendered when active.
- **Toast Stack** (`<ToastContainer />`): Floating alerts pinned to bottom-right.

---

## 3. Tablet Collapsed Layout (`701px – 1050px`)

At widths between 701px and 1050px, the rail collapses into a compact icon bar (`64px`), hiding textual labels to preserve content width:

```text
+----------------------------------------------------------------------------------------------------------------+
| [X] X-Link                                                                                       [-] [□] [✕]   |
+------+---------------------------------------------------------------------------------------------------------+
| [Logo|  VIEW HEADER (.view-heading)                                                                            |
|  ]   |  Title                                                                        [Header Action Items...]  |
+------+  Subtitle copy                                                                                          |
|      +---------------------------------------------------------------------------------------------------------+
| [*]  |                                                                                                         |
|      |                                                                                                         |
| [ ]  |                                                                                                         |
|      |                                                                                                         |
| [ ]  |                                         VIEW CONTENT VIEWPORT                                           |
|      |                                            (.view-content)                                              |
| [ ]  |                                                                                                         |
|      |                                                                                                         |
| [ ]  |                                                                                                         |
|      |                                                                                                         |
| [ ]  |                                                                                                         |
|      |                                                                                                         |
|      |                                                                                                         |
| (●)  |                                                                                                         |
+------+---------------------------------------------------------------------------------------------------------+
```

### Key Differences:
- `.app-navigation`: Width shrinks from `212px` to `64px`, `padding-inline: 8px`.
- `.nav-brand-text`, `.nav-session-text`, and `.nav-label`: Set to `display: none;`.
- Icons centered horizontally inside `nav-destination` buttons.
- `.nav-session`: Padding reduced to `15px 0`, icon/dot centered.

---

## 4. Narrow Viewport Layout (`≤ 700px`)

### 4.1 Normal State (Drawer Closed)
When width is ≤ 700px, the sidebar is hidden off-screen (`transform: translateX(-105%)`). A floating hamburger button appears in the top-left corner, and the header copy shifts right:

```text
+--------------------------------------------------------------------------------+
|  [ ☰ ]   Dashboard                                          (●) Connected      |
|          Your connection at a glance                                           |
+--------------------------------------------------------------------------------+
|                                                                                |
|  VIEW CONTENT AREA                                                             |
|  (Padded 16px horizontal, stacks vertically in single column)                  |
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  HERO CONNECTION CARD                                                    |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  TRAFFIC CARD                                                            |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
+--------------------------------------------------------------------------------+
```

### 4.2 Drawer Open State (with Modal Scrim)
Tapping `[ ☰ ]` slides in the mobile navigation drawer over an opaque backdrop scrim (`rgb(0 0 0 / 50%)`):

```text
+--------------------------------------------------------------------------------+
| +----------------------------+                                                 |
| | [Logo] X-Link          [✕] |                                                 |
| |        Network control     |                                                 |
| |----------------------------|                                                 |
| |  NAVIGATION                |                                                 |
| |  +----------------------+  |                                                 |
| |  | [*] Speed  Dashboard |  |                                                 |
| |  +----------------------+  |                                                 |
| |  | [ ] Folder Profiles  |  |                 SCRIM BACKDROP                  |
| |  | [ ] Route  Routing   |  |              (.nav-scrim: 50% black)            |
| |  | [ ] Hub    Connection|  |                                                 |
| |  | [ ] Term   Logs      |  |         (Tap outside to close drawer)           |
| |  | [ ] Cog    Settings  |  |                                                 |
| |  +----------------------+  |                                                 |
| |                            |                                                 |
| |  +----------------------+  |                                                 |
| |  | (●) Connected        |  |                                                 |
| |  |     Proxy session    |  |                                                 |
| |  +----------------------+  |                                                 |
| +----------------------------+                                                 |
|   (.app-navigation.is-open)                                                    |
+--------------------------------------------------------------------------------+
```

---

## 5. Navigation Items Specification

Accurate mapping of all 6 navigation targets defined in `NAV_ITEMS`:

| ID | Label | Material Symbol | Routing View Component | Description |
| :--- | :--- | :--- | :--- | :--- |
| `dashboard` | **Dashboard** | `speed` | [DashboardView.tsx](../../src/views/DashboardView.tsx) | Connection toggle, real-time throughput chart, active node selector, session stats. |
| `profiles` | **Profiles** | `folder_open` | [ConfigView.tsx](../../src/views/ConfigView.tsx) | Subscription management, manual node configurations, diff modal, latency testing. |
| `routing` | **Routing** | `alt_route` | [RoutingView.tsx](../../src/views/RoutingView.tsx) | Final outbound, rule set management, custom routing rules, bypass policy. |
| `connections` | **Connections** | `hub` | [ConnectionsView.tsx](../../src/views/ConnectionsView.tsx) | Live connection tracker, active sockets, per-destination traffic, kill connection. |
| `logs` | **Logs** | `terminal` | [LogsView.tsx](../../src/views/LogsView.tsx) | Virtualized core log stream, log level filters (Info, Warn, Error, System), autoscroll. |
| `settings` | **Settings** | `settings` | [SettingsView.tsx](../../src/views/SettingsView.tsx) | TUN/System mode, port allocation, DNS, Clash API, app updates, core diagnostics. |

---

## 6. Session Indicator States

The `.nav-session` element in the sidebar reflects real-time status from `useConnectionStore.connectionStatus`:

```text
[ Disconnected ]      [ Connecting ]        [ Connected ]
(○) Disconnected      (◐) Connecting        (●) Connected
    Proxy session         Proxy session         Proxy session
Dot: #7e7576 (Gray)   Dot: #b47a29 (Amber)  Dot: #368360 (Green)
```

- **Disconnected**: Status indicator gray (`var(--md-sys-color-outline)`). Text: "Disconnected".
- **Connecting**: Status indicator amber pulse (`#b47a29`). Text: "Connecting".
- **Connected**: Status indicator solid green (`#368360`). Text: "Connected".

---

## 7. View Error State

If an active view throws while rendering, the view-level error boundary replaces that view's content without removing the navigation rail:

```text
+-------------------------------------------------------------+
|                       [error icon]                          |
|                  Something went wrong                       |
|           <error message for this component>                |
|              [Try Again]  [Reload App]                       |
+-------------------------------------------------------------+
```

`Try Again` retries the view; `Reload App` reloads the webview. A supplied fallback title may replace the default heading.

---

## 8. Accessibility & Keyboard Navigation

1. **Floating Menu Button (`.mobile-menu-button`)**:
   - `aria-label="Open navigation"`, `aria-expanded={open}`, `aria-controls="primary-navigation"`.
   - Focus is restored to this button upon closing the mobile drawer.
2. **Drawer Trapping (`≤ 700px`)**:
   - Drawer container (`<aside id="primary-navigation">`) receives `inert={compact && !open}` when hidden, preventing off-screen keyboard tab navigation.
   - When opened, focus shifts automatically to the active destination (`[aria-current="page"]`).
   - A keyboard listener traps `Tab` and `Shift+Tab` within the drawer buttons.
   - Pressing `Escape` closes the drawer and refocuses the trigger button.
3. **Active Page Indication**:
   - The selected tab element has `aria-current="page"` and `.is-active`.
