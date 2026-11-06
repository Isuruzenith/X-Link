---
title: "X-Link — Current UI/UX Wireframes"
aliases: ["Current Wireframes", "Material Web Wireframes"]
tags:
  - wireframe
  - ui
  - frontend
created: 2026-11-05
updated: 2026-11-06
---

# X-Link — Current UI/UX Wireframes

Low-fidelity layouts for the **current Tauri + React + Material Web desktop UI**. These are documentation of the implemented interface, not a proposal or a pixel-perfect rendering. Each note links to its source components; controls, responsive arrangements, and important alternative states are shown with text diagrams. The canonical window is **1000 × 680 px**, minimum **720 × 520 px** (see [[00-window-and-grid]]); per-view notes below were captured from the source when the window was documented as `950 × 650` / min `380 × 550`, so their breakpoint tables show *today's CSS*, while [[00-window-and-grid]] and [[09-fit-layouts]] define the **target** that [[specs/003-responsive-fit-layout/spec]] implements.

| Note | Coverage |
| --- | --- |
| **[[00-window-and-grid]]** | **Canonical** window size, size classes (Compact / Medium / Expanded), fit-to-window rules, vertical budgets, test matrix, legacy-breakpoint migration map |
| **[[09-fit-layouts]]** | **Exact-scale** drawings (1 col = 10 px) of every view at 1000×680 and 720×520, per-view fit contract |
| [[01-shell-navigation]] | Window shell, six destinations, session indicator, compact navigation drawer, toast and error states |
| [[02-dashboard]] | Connection states, live traffic, server switching, telemetry and inbound listeners |
| [[03-profiles]] | Profile list, import/preview, nodes, empty states, subscription diff dialog |
| [[04-node-editor]] | Server editor drawer, protocol-specific fields, transport, TLS and validation |
| [[05-routing]] | Final outbound, rule sets, rules table, inline editors and validation |
| [[06-connections]] | Disconnected/empty states, filters, sortable table, details and close actions |
| [[07-logs]] | Stream, level/search filters, autoscroll and empty states |
| [[08-settings]] | General, TUN Mode, Sniffing, DNS, API, Diagnostics and backup/restore flows |

**Reading key:** `[Button]` is an action; `[value v]` is a selector; `(●)` / `( )` denote on/off or status as labeled; `[*]` denotes the selected destination. Example addresses, domains, node names, and counts are illustrative. The existing [[04-UI-Wireframes]] document is a historical design reference and includes proposals not present in the app (for example, outbound-group editing and encrypted backups); use these notes and the source for current behavior.

**Accessibility and visual system:** The app uses Material Web controls where registered, semantic HTML for structure, existing light/dark theme tokens, descriptive status text alongside color, keyboard-accessible navigation, and a narrow-screen drawer. Wireframes abstract away exact color, spacing, fonts and OS window chrome.
