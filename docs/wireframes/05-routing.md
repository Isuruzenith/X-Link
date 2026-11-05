---
title: "Wireframe 05 — Routing Rules & Rule Sets"
aliases: ["Routing Wireframe", "RoutingView Wireframe", "Rule Sets Wireframe"]
tags:
  - wireframe
  - ui
  - routing
  - rules
created: 2026-11-03
updated: 2026-11-05
---

# Wireframe 05 — Routing Rules & Rule Sets

**Navigation**: [[00-Home-MOC]] | **Related Wireframes**: [[01-shell-navigation]] · [[02-dashboard]] · [[03-profiles]] · [[06-connections]] · [[07-logs]]
**Source Components**:
- Primary View: [RoutingView.tsx](../../src/views/RoutingView.tsx)
- Container Shell: [ViewShell.tsx](../../src/components/ViewShell.tsx)
- Stores: [routingStore.ts](../../src/stores/routingStore.ts) · [settingsStore.ts](../../src/stores/settingsStore.ts) · [connectionStore.ts](../../src/stores/connectionStore.ts)
- Types & Models: [store.ts](../../src/utils/store.ts)
- Form Controls: [FormInput.tsx](../../src/components/form/FormInput.tsx) · [FormSelect.tsx](../../src/components/form/FormSelect.tsx) · [FormSwitch.tsx](../../src/components/form/FormSwitch.tsx)
- Structural CSS: [material-theme.css](../../src/material-theme.css)

---

## 1. Grid Architecture & Responsive Breakpoints

The Routing layout (`.routing-view`) divides global fallback and rule set management into an overview grid above a full-width rule evaluation table:

| Breakpoint Tier | Container Width | Overview Grid Configuration | Rule Grid Fields | Table Overflow |
| :--- | :--- | :--- | :--- | :--- |
| **Desktop Wide** | `> 1050px` | `minmax(250px, 1fr) minmax(0, 2fr)` (2 cols) | `repeat(3, minmax(0, 1fr))` | `.routing-table-scroll` (min 680px) |
| **Tablet / Narrow** | `701px – 1050px` | `minmax(0, 1fr)` (single column stacked) | `repeat(2, minmax(0, 1fr))` | Horizontal scroll with sticky head |
| **Mobile Narrow** | `≤ 700px` | Single column stacked | `repeat(2, minmax(0, 1fr))` | Horizontal scroll with thumb bar |
| **Extreme Narrow** | `≤ 440px` | Single column stacked | `minmax(0, 1fr)` (single col) | Horizontal scroll container |

---

## 2. Desktop Connected State (`> 1050px`)

When the proxy is running (`isConnected === true`), a warning banner signals live restart protection above the overview cards:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Routing                                                                                                [ + Add Rule ] |
|  Set the final outbound, manage rule sets and choose how traffic is routed                                             |
+------------------------------------------------------------------------------------------------------------------------+
|  [!] Routing changes are saved and applied with a validated restart of the running proxy; if the restart fails...     |
+------------------------------------------------------------------------------------------------------------------------+
|                                                                                                                        |
|  +----------------------------------------------+  +----------------------------------------------------------------+  |
|  |  (⤹) Global Routing                          |  |  (🗄) Rule Sets  [ 1 ]                         [ + Add Rule Set ]|  |
|  |----------------------------------------------|  |----------------------------------------------------------------|  |
|  |  Final outbound (fallback)                   |  |  (🗄) geoip-cn  [remote] [binary] [↻ 1d] [✓ Cached]    [⤓] [🗑]  |  |
|  |  [ 🌐 Proxy (default)                      v]|  |                                                                |  |
|  |                                              |  |                                                                |  |
|  |  Bypass LAN / Private IPs               (●)  |  |                                                                |  |
|  |  Route RFC-1918 addresses directly           |  |                                                                |  |
|  +----------------------------------------------+  +----------------------------------------------------------------+  |
|                                                                                                                        |
|  +------------------------------------------------------------------------------------------------------------------+  |
|  |  (☷) Routing Rules  [ 2 / 2 active ]                                                                             |  |
|  |------------------------------------------------------------------------------------------------------------------|  |
|  |  #   ACTIVE   TYPE            VALUE                OUTBOUND      NOTES                 ACTIONS                   |  |
|  |  1    [●]     [domain]        example.com          [ DIRECT ]    Direct test           [ ✎ ]  [ 🗑 ]              |  |
|  |  2    [●]     [ip_cidr]       1.1.1.1/32           [ PROXY  ]    —                     [ ✎ ]  [ 🗑 ]              |  |
|  |                                                                                                                  |  |
|  |  (i) Rules evaluate top-to-bottom. First match wins. Hardcoded: dns → dns-out, private IPs → direct.             |  |
|  +------------------------------------------------------------------------------------------------------------------+  |
+------------------------------------------------------------------------------------------------------------------------+
```

---

## 3. Inline Forms & Validation States

### 3.1 Inline Add / Edit Rule Form (`showAddRule === true`)
Expanding the rule editor mounts a grid form above the rule table. Selecting `process_name` exposes process pickers:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Rule type             | Rule value                                  | Outbound         | Invert | Notes               |
|  [ Process Name      v]| [ chrome.exe                              ] | [ ⚡ Direct    v] |  [ ]   | [ Bypass browser  ] |
|                        | [↻ Refresh processes] [chrome.exe (PID 912)v]|                  |        |                     |
|                        | [+chrome.exe] [+telegram.exe] [+discord.exe]|                  |        | [ Add ]  [ ✕ ]      |
+------------------------------------------------------------------------------------------------------------------------+
```

When submitted with an empty rule value, inline error text alerts the user:
```text
|  Rule value: [                                            ]  |
|  [!] Enter a rule value.                                     | <- (aria-invalid role="alert")
```

### 3.2 Inline Add Rule Set Form (`showAddRuleSet === true`)
```text
+------------------------------------------------------------------------------------------------------------------------+
|  Add Rule Set                                                                                                    [ ✕ ] |
|  Tag: [ geoip-cn        ]  Type: [ Remote v]  Format: [ Binary (.srs) v]  Update interval: [ 1d v]  [ Add ]              |
|  Remote URL: [ https://cdn.jsdelivr.net/.../geoip/cn.srs                                                             ] |
|  [!] Enter a remote URL for the rule set.                                                                              |
+------------------------------------------------------------------------------------------------------------------------+
```

---

## 4. Empty States (No Rules / No Rule Sets)

```text
+------------------------------------------------+  +----------------------------------------------------------------+
|  (⤹) Global Routing                           |  |  (🗄) Rule Sets  [ 0 ]                         [ + Add Rule Set ]|
|  Final outbound: [ 🌐 Proxy (default)        v]|  |----------------------------------------------------------------|
|  Bypass LAN / Private IPs: [●]                 |  |  No rule sets. Add a remote GeoIP/GeoSite rule set above.      |
+------------------------------------------------+  +----------------------------------------------------------------+

+--------------------------------------------------------------------------------------------------------------------+
|  (☷) Routing Rules  [ 0 / 0 active ]                                                                               |
|--------------------------------------------------------------------------------------------------------------------|
|                       No routing rules. All traffic falls to final outbound (proxy).                               |
+--------------------------------------------------------------------------------------------------------------------+
```

---

## 5. Narrow Viewport Layout (`≤ 700px`)

In viewports `<= 700px`, the overview grid stacks vertically and the rules table enables smooth horizontal scroll:

```text
+--------------------------------------------------------------------------------+
|  [ ☰ ]  Routing                                                 [ + Add Rule ] |
|         Set the final outbound, manage rule sets and choose...                 |
+--------------------------------------------------------------------------------+
|  +--------------------------------------------------------------------------+  |
|  |  (⤹) Global Routing                                                      |  |
|  |  Final outbound (fallback): [ 🌐 Proxy (default)                       v]|  |
|  |  Bypass LAN / Private IPs                                           (●)  |  |
|  +--------------------------------------------------------------------------+  |
|  +--------------------------------------------------------------------------+  |
|  |  (🗄) Rule Sets  [ 1 ]                                  [ + Add Rule Set ]|  |
|  |  geoip-cn  [remote] [binary] [↻ 1d]                              [⤓] [🗑] |  |
|  +--------------------------------------------------------------------------+  |
|  +--------------------------------------------------------------------------+  |
|  |  (☷) Routing Rules  [ 2 / 2 active ]                                     |  |
|  |  [ <------------------ HORIZONTAL SCROLL AREA ------------------> ]      |  |
|  |  #  ACTIVE  TYPE       VALUE        OUTBOUND    NOTES       ACTIONS      |  |
|  |  1   [●]    [domain]   example.com  [ DIRECT ]  Direct test [✎] [🗑]     |  |
|  |  2   [●]    [ip_cidr]  1.1.1.1/32   [ PROXY  ]  —           [✎] [🗑]     |  |
|  +--------------------------------------------------------------------------+  |
+--------------------------------------------------------------------------------+
```

---

## 6. UI Elements & Accurate Labels

| Element Selector / Event | UI Label / Display Text | Description & State Binding |
| :--- | :--- | :--- |
| `ViewShell.actions` | `+ Add Rule` | Opens inline rule form; resets `editingRule` to `null`. |
| `.routing-card-header` | `Global Routing` / `Rule Sets` / `Routing Rules` | Section headings with icons `alt_route`, `database`, `filter_list`. |
| `FormSelect[finalOutbound]` | `🌐 Proxy (default)` / `⚡ Direct` / `🚫 Block` | Fallback routing policy bound to `settings.finalOutbound`. |
| `FormSwitchRow[bypassLan]` | `Bypass LAN / Private IPs` | Routes RFC-1918 traffic directly without hitting proxy tunnels. |
| `BadgeOutbound` | `PROXY` / `DIRECT` / `BLOCK` / `DNS` | Styled badges reflecting `rule.outbound` action type. |
| `Button[submitRule]` | `Add` / `Update` | Commits rule to store; triggers cold restart rollback protection if connected. |
| `Button[updateRuleSet]` | Download icon `[ ⤓ ]` (spins during update) | Triggers `updateRuleSet(rs)` for remote ruleset bundles. |
| `Switch[rule.enabled]` | Toggle switch (`aria-label="Enable rule {value}"`) | Soft-disables rule (`opacity-40` row state) without deletion. |
| `.info-box` | `Rules evaluate top-to-bottom...` | Persistent routing priority guidance callout at base of view. |
