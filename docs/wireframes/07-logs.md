---
title: "Wireframe 07 — Live Console Logs"
aliases: ["Logs Wireframe", "LogsView Wireframe", "Console Output"]
tags:
  - wireframe
  - ui
  - logs
  - console
  - diagnostics
created: 2026-11-03
updated: 2026-11-05
---

# Wireframe 07 — Live Console Logs

> [!NOTE] Target geometry
> Breakpoint tables in this note describe the **current CSS**. The target is three size classes (Compact `<600`, Medium `600–839`, Expanded `≥840`) in a **1000×680** default / **720×520** minimum window with zero document scroll — see [[wireframes/00-window-and-grid]] (rules, migration map) and [[wireframes/09-fit-layouts]] (exact-scale drawing for this view). Work is tracked in [[specs/003-responsive-fit-layout/tasks]].

**Navigation**: [[00-Home-MOC]] | **Related Wireframes**: [[01-shell-navigation]] · [[02-dashboard]] · [[05-routing]] · [[06-connections]]
**Source Components**:
- Primary View: [LogsView.tsx](../../src/views/LogsView.tsx)
- Container Shell: [ViewShell.tsx](../../src/components/ViewShell.tsx)
- Stores: [logStore.ts](../../src/stores/logStore.ts) · [toastStore.ts](../../src/stores/toastStore.ts)
- Virtualization: `react-window` (`List`, row height 30px)
- Form Controls: [FormInput.tsx](../../src/components/form/FormInput.tsx) · [SegmentedControl.tsx](../../src/components/form/SegmentedControl.tsx) · [switch.tsx](../../src/components/ui/switch.tsx)
- Structural CSS: [material-theme.css](../../src/material-theme.css)

---

## 1. Grid Architecture & Responsive Breakpoints

The Logs terminal view (`.utility-view.utility-log`) combines virtualized console output with real-time severity filtering and clipboard tools:

| Breakpoint Tier | Container Width | Action Toolbar | Filter Toolbar | Terminal Row Geometry |
| :--- | :--- | :--- | :--- | :--- |
| **Desktop Wide** | `> 1050px` | Inline right actions (`gap: 10px`) | Search (`max 390px`) + Segmented + Counter | 30px row: `w-14` time, `w-9` level, flex body |
| **Tablet / Narrow** | `701px – 1050px` | Inline right actions | Counter hidden (`sm:inline`); filters wrap | Text truncated with trailing ellipsis |
| **Mobile Narrow** | `≤ 700px` | Header actions wrap | Full-width search; segments horizontal scroll | Compact typography (`text-[11px]`) |
| **Extreme Narrow** | `≤ 440px` | Stacked action buttons | Controls stacked vertically | Single-line mono with strict overflow clip |

---

## 2. Desktop Live Log Stream Layout (`> 1050px`)

Log streams are rendered through a virtualized `react-window` List maintaining fixed `30px` row heights for memory efficiency:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Logs                                                                       [ 📋 Copy All ] [ 🗑 Clear ] | [●] Autoscroll|
|  Follow proxy activity and system events as they happen                                                                |
+------------------------------------------------------------------------------------------------------------------------+
|  [🔍 Find a message…                   ]  [ All ]* [ Info ] [ Warn ] [ Errors ] [ System ]       Showing 6 of 6 entries|
+------------------------------------------------------------------------------------------------------------------------+
|  +------------------------------------------------------------------------------------------------------------------+  |
|  | 10:14:02  SYS   sing-box v1.13.14 starting with profile "Premium Sub"                                            |  |
|  | 10:14:03  INFO  [TUN] interface wintun created (172.19.0.1/30, mtu=1500)                                         |  |
|  | 10:14:03  INFO  [DNS] fakeip router ready on 198.18.0.0/15                                                       |  |
|  | 10:14:04  SYS   sing-box established on port Mixed:7892.                                                         |  |
|  | 10:15:20  WARN  [WARN] proxy handshake 198.51.100.42:443 timed out, retrying                                     |  |
|  | 10:16:11  ERR   [ERROR] connection reset by peer (cloudflare.com:443)                                            |  |
|  |                                                                                                                  |  |
|  |                                      (react-window virtualized viewport)                                          |  |
|  +------------------------------------------------------------------------------------------------------------------+  |
+------------------------------------------------------------------------------------------------------------------------+
```

---

## 3. Empty & Filtered States

### 3.1 Initial Idle State (No Core Logs Yet)
```text
+------------------------------------------------------------------------------------------------------------------------+
|                                                          ( >_ )                                                        |
|                                        Waiting for core process console output…                                        |
+------------------------------------------------------------------------------------------------------------------------+
```

### 3.2 Filter Yields Zero Matches
When search query or level filters exclude all current lines in the buffer:
```text
+------------------------------------------------------------------------------------------------------------------------+
|                                                          ( >_ )                                                        |
|                                           No logs match the current filters.                                           |
+------------------------------------------------------------------------------------------------------------------------+
```

---

## 4. Clipboard Feedback & Toast Alerts

Clicking `[ Copy All ]` extracts formatted plaintext logs into the system clipboard, raising reactive toasts:

```text
+----------------------------------------------------+      +----------------------------------------------------+
| [✓] Logs Copied                                [✕] |      | [!] Copy Failed                                [✕] |
| All logs copied to clipboard.                      |      | Clipboard access failed. Try again.                |
+----------------------------------------------------+      +----------------------------------------------------+
              (Success Notification)                                      (Clipboard Error / Denied)
```

---

## 5. Narrow Viewport Layout (`≤ 700px`)

```text
+--------------------------------------------------------------------------------+
|  [ ☰ ]  Logs                                                                   |
|         Follow proxy activity and system events as they happen                 |
|         [ 📋 Copy All ]  [ 🗑 Clear ]  |  [●] Autoscroll                       |
+--------------------------------------------------------------------------------+
|  [🔍 Find a message…                                                         ] |
|  [ All ]*  [ Info ]  [ Warn ]  [ Errors ]  [ System ]                          |
+--------------------------------------------------------------------------------+
|  +--------------------------------------------------------------------------+  |
|  | 10:14:02  SYS   sing-box v1.13.14 starting with profile "Premium Sub"    |  |
|  | 10:14:03  INFO  [TUN] interface wintun created (172.19.0.1/30, mtu=1500) |  |
|  | 10:14:03  INFO  [DNS] fakeip router ready on 198.18.0.0/15               |  |
|  | 10:14:04  SYS   sing-box established on port Mixed:7892.                 |  |
|  | 10:15:20  WARN  [WARN] proxy handshake 198.51.100.42:443 timed out...   |  |
|  | 10:16:11  ERR   [ERROR] connection reset by peer (cloudflare.com:443)    |  |
|  +--------------------------------------------------------------------------+  |
+--------------------------------------------------------------------------------+
```

---

## 6. UI Elements & Accurate Labels

| Element Selector / Event | UI Label / Display Text | Description & State Binding |
| :--- | :--- | :--- |
| `Button[handleCopy]` | `📋 Copy All` | Joins all in-memory logs and invokes clipboard API; displays success or error toast. |
| `Button[clearLogs]` | `🗑 Clear` | Clears all buffer entries in `useLogStore`. |
| `Switch[autoscroll-toggle]` | `Autoscroll` | Locks list to bottom on log push; turns off when scrolled >40px away, re-enables at bottom. |
| `FormInput[filterText]` | `Search logs` (label) / `Find a message…` | Substrings matched against log line text with deferred value optimization. |
| `SegmentedControl[levelFilter]` | `All` / `Info` / `Warn` / `Errors` / `System` | Filters log entries matching type (`info`, `warn`, `error`, `system`). |
| `.utility-log-filters > span` | `Showing {filtered.length} of {logs.length} entries` | Live counter summarizing active filter results against total log buffer. |
| `LogRow .text-muted-foreground`| `{log.timestamp}` | Formatted timestamp string (`HH:MM:SS`) pinned at start of row. |
| `LogRow [getLevelLabel]` | `INFO` / `WARN` / `ERR` / `SYS` | Color-coded severity badge (`ERR` = destructive, `WARN` = amber, `SYS` = blue). |
| `renderHighlightedText` | `[{tagText}]` | Highlighting helper wrapping bracket tags (e.g. `[DNS]`, `[TUN]`) in styled pill borders. |
