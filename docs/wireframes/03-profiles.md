---
title: "Wireframe 03 — Profiles View & Subscription Diff"
aliases: ["Profiles Wireframe", "ConfigView Wireframe", "Subscription Diff Modal"]
tags:
  - wireframe
  - ui
  - profiles
  - subscriptions
created: 2026-11-03
updated: 2026-11-05
---

# Wireframe 03 — Profiles View & Subscription Diff

> [!NOTE] Target geometry
> Breakpoint tables in this note describe the **current CSS**. The target is three size classes (Compact `<600`, Medium `600–839`, Expanded `≥840`) in a **1000×680** default / **720×520** minimum window with zero document scroll — see [[wireframes/00-window-and-grid]] (rules, migration map) and [[wireframes/09-fit-layouts]] (exact-scale drawing for this view). Work is tracked in [[specs/003-responsive-fit-layout/tasks]].

**Navigation**: [[00-Home-MOC]] | **Related Wireframes**: [[01-shell-navigation]] · [[02-dashboard]] · [[04-node-editor]]

**Source Components**:
- Primary View: [ConfigView.tsx](../../src/views/ConfigView.tsx)
- Viewport Shell: [ViewShell.tsx](../../src/components/ViewShell.tsx)
- Profile Store: [profileStore.ts](../../src/stores/profileStore.ts)
- Domain Model & Types: [store.ts](../../src/utils/store.ts)
- Node Editor Store Trigger: [nodeEditorStore.ts](../../src/stores/nodeEditorStore.ts)
- Cumulative Traffic Store: [statsStore.ts](../../src/stores/statsStore.ts)
- Structural CSS: [material-theme.css](../../src/material-theme.css)

---

## 1. Grid Architecture & Layout Breakpoints

The Profiles view (`.profiles-layout`) provides a split-pane interface separating profile selection/import from server inspection, latency testing, and manual node CRUD:

| Breakpoint Tier | Container Width | Grid Template Configuration | Max Panel Height |
| :--- | :--- | :--- | :--- |
| **Desktop Wide** | `> 1050px` | `minmax(280px, .7fr) minmax(340px, 1.3fr)` (2 columns) | `max-height: min(68vh, 800px);` |
| **Desktop with Import** | `> 1050px` | Column 1: Import (row 1), List (row 2); Column 2: Servers (row 1/3) | `max-height: min(58vh, 650px);` |
| **Tablet / Narrow** | `≤ 1050px` / `≤ 700px` | Single column (`minmax(0, 1fr)`): Import $\rightarrow$ List $\rightarrow$ Servers | `max-height: 360px;` per section |

---

## 2. Desktop Layout: Selected Profile & Server List (`> 1050px`)

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Profiles                                                                                              [ + Add profile ]|
|  Choose a configuration and select the server that carries your traffic                                                |
+------------------------------------------------------------------------------------------------------------------------+
|                                                                                                                        |
|  SAVED PROFILES (2)                                  |  PREMIUM SUBSCRIPTION                                           |
|  Select a profile to view its servers.               |  12 nodes · subscription · Active profile                       |
|  +------------------------------------------------+  |  [Auto-update: 6h v] [ ⟳ Refresh ] [+ Add node] [⚡ Test all]    |
|  | [Link]  Premium Subscription                   |  |-----------------------------------------------------------------|
|  |         subscription · 12 nodes · 11/05/2026   |  |  +-----------------------------------------------------------+  |
|  |         Auto: 6h                   [Connected] |  |  | [US]  US-West-Reality                [Connected]  TCP: 42 ms  |  |
|  |         [ ⟳ ] [ 🗑 ]                           |  |  |       vless · 1.4 GB used                             |  |
|  +------------------------------------------------+  |  |       [ 🖧 Ping ] [ ⚡ Speed ] [ ✎ Edit ] [ 🗑 Delete ]       |  |
|                                                      |  |  +-----------------------------------------------------------+  |
|  +------------------------------------------------+  |  |  +-----------------------------------------------------------+  |
|  | [Globe] Self-Hosted Backup Nodes               |  |  | [SG]  SG-Direct-Hy2                               TCP: 88 ms  |  |
|  |         manual · 3 nodes · 10/28/2026          |  |  |       hysteria2 · 420.0 MB used                       |  |
|  |         [ 🗑 ]                                 |  |  |       [ 🖧 Ping ] [ ⚡ Speed ] [ ✎ Edit ] [ 🗑 Delete ]       |  |
|  +------------------------------------------------+  |  |  +-----------------------------------------------------------+  |
|                                                      |  |  |  +-----------------------------------------------------------+  |
|                                                      |  |  | [JP]  Tokyo-Trojan-WS                             Failed   |  |
|                                                      |  |  |       trojan                                               |  |
|                                                      |  |  |       [ 🖧 Ping ] [ ⚡ Speed ] [ ✎ Edit ] [ 🗑 Delete ]       |  |
|                                                      |  |  +-----------------------------------------------------------+  |
+------------------------------------------------------------------------------------------------------------------------+
```

---

## 3. Import Profile Panel & Pre-Import Node Preview

Clicking `[ + Add profile ]` expands the `.profiles-import` panel above the saved profiles list:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Profiles                                                                                            [ ✕ Close import ]|
|  Choose a configuration and select the server that carries your traffic                                                |
+------------------------------------------------------------------------------------------------------------------------+
|  ADD A PROFILE                                                                                                         |
|  Paste a share link, subscription URL or config, or choose a file.        [ Empty profile ]  [ 📂 Choose file ]        |
|  --------------------------------------------------------------------------------------------------------------------  |
|  Profile name (optional)                                                                                               |
|  [ My Premium Subscription                                                                                          ]  |
|                                                                                                                        |
|  Link or configuration                                                                                                 |
|  +------------------------------------------------------------------------------------------------------------------+  |
|  | https://example.com/api/v1/client/subscribe?token=test-subscription-token-dummy                                  |  |
|  +------------------------------------------------------------------------------------------------------------------+  |
|                                                                                                                        |
|  INLINE PREVIEW BOX (After clicking "Preview nodes"):                                                                  |
|  +------------------------------------------------------------------------------------------------------------------+  |
|  | [✓] Found 14 proxy nodes                                                                                     [✕] |  |
|  | [VLESS: 8]  [Trojan: 4]  [Hysteria2: 2]                                                                            |  |
|  | Sample servers: US-West-01, US-West-02, SG-Hy2-01 (and 11 more)                                                   |  |
|  +------------------------------------------------------------------------------------------------------------------+  |
|                                                                                                                        |
|  [ 📋 Paste clipboard ]       [ 👁 Preview nodes ]                                              [ ⬇ Import profile ] |  |
+------------------------------------------------------------------------------------------------------------------------+
```

### Import Panel Validation & Error Displays:
- **URL Syntax Warning**: Displays `.material-error` alert: `"Enter a valid subscription URL, such as https://example.com/config."`
- **Import Error**: Displays `.material-error` alert: `[!] Failed to parse subscription: Invalid JSON schema`.
- **Import Success**: Displays `.material-success` alert: `[✓] Profile imported.`

---

## 4. Empty States

### 4.1 Empty Profiles List (First Boot)
When no profiles have been created or imported:

```text
+----------------------------------------------------+  +----------------------------------------------------+
|  SAVED PROFILES (0)                                |  |  Servers                                           |
|  Select a profile to view its servers.             |  |  Select a saved profile to see its servers.        |
|  +----------------------------------------------+  |  |  +----------------------------------------------+  |
|  |                     [📂]                     |  |  |                     [dns]                      |  |
|  |               No profiles yet                |  |  |              No profile selected                |  |
|  |  Add a subscription, paste a share link or   |  |  |  Choose a profile to see its servers, or        |  |
|  |  choose a config file to get started.        |  |  |  add a new one.                                 |  |
|  |                                              |  |  |                                                  |  |
|  |         [ + Add your first profile ]         |  |  |                                                  |  |
|  +----------------------------------------------+  |  +----------------------------------------------------+
+----------------------------------------------------+
```

### 4.2 Selected Profile with Zero Servers
When an empty manual profile is created via `[ Empty profile ]`:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Servers: Manual Staging                                                              [ + Add node ] [ ⚡ Test all ]   |
|  0 nodes · manual · Active profile                                                                                     |
|  +------------------------------------------------------------------------------------------------------------------+  |
|  |                                                      [⚡]                                                         |  |
|  |                                              No servers available                                                |  |
|  |                                  This profile has no selectable proxy nodes.                                     |  |
|  +------------------------------------------------------------------------------------------------------------------+  |
+------------------------------------------------------------------------------------------------------------------------+
```

---

## 5. Subscription Update Diff Preview Modal (`diffPreview` Active)

Triggered by clicking `[ ⟳ ]` on a subscription. A full accessibility dialog modal mounts with a `backdrop-blur-xs` scrim:

```text
+------------------------------------------------------------------------------------------------------------------------+
|                                           MODAL BACKDROP SCRIM (bg-black/60)                                           |
|                                                                                                                        |
|                      +--------------------------------------------------------------------------+                      |
|                      |  [⇄] Subscription Update Preview                                    [✕]  |                      |
|                      |      Profile: Premium Subscription                                       |                      |
|                      |--------------------------------------------------------------------------|                      |
|                      |  SUMMARY STATS                                                           |                      |
|                      |  +-------------------+  +-------------------+  +----------------------+  |                      |
|                      |  | NODES             |  | ADDED             |  | REMOVED              |  |                      |
|                      |  | 12 → 14           |  | +3                |  | -1                   |  |                      |
|                      |  +-------------------+  +-------------------+  +----------------------+  |                      |
|                      |                                                                          |                      |
|                      |  ACTIVE NODE WARNING (Conditional if selected server was deleted):       |                      |
|                      |  +--------------------------------------------------------------------+  |                      |
|                      |  | [!] Active server removed by provider!                             |  |                      |
|                      |  | Your currently selected server "US-Old-1" is no longer available.  |  |                      |
|                      |  | Active server will automatically fallback to "US-West-Reality".    |  |                      |
|                      |  +--------------------------------------------------------------------+  |                      |
|                      |                                                                          |                      |
|                      |  ADDED SERVERS (+3)                                                      |                      |
|                      |  +--------------------------------------------------------------------+  |                      |
|                      |  | [+ US-West-Reality-02]  [+ JP-Tokyo-Hy2-03]  [+ UK-London-VLESS]   |  |                      |
|                      |  +--------------------------------------------------------------------+  |                      |
|                      |                                                                          |                      |
|                      |  REMOVED SERVERS (-1)                                                    |                      |
|                      |  +--------------------------------------------------------------------+  |                      |
|                      |  | [- US-Old-1]                                                       |  |                      |
|                      |  +--------------------------------------------------------------------+  |                      |
|                      |                                                                          |                      |
|                      |  This preview expires after five minutes. Applying uses previewed nodes. |                      |
|                      |--------------------------------------------------------------------------|                      |
|                      |                                            [ Cancel ]  [ ⟳ Apply Update ]|                      |
|                      +--------------------------------------------------------------------------+                      |
|                                                                                                                        |
+------------------------------------------------------------------------------------------------------------------------+
```

---

## 6. Narrow Viewport Layout (`≤ 700px`)

On narrow displays, the grid collapses into a single column. The import panel stacks above saved profiles, which stack above server nodes:

```text
+--------------------------------------------------------------------------------+
|  [ ☰ ]   Profiles                                             [ + Add profile ]|
|          Choose a configuration and select your server                         |
+--------------------------------------------------------------------------------+
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  Saved profiles (2)                                                      |  |
|  |  +--------------------------------------------------------------------+  |  |
|  |  | [Link]  Premium Subscription                   [Connected]         |  |  |
|  |  |         subscription · 12 nodes · 11/05/2026                       |  |  |
|  |  |         Auto: 6h                                        [ ⟳ ] [ 🗑 ] |  |  |
|  |  +--------------------------------------------------------------------+  |  |
|  |  +--------------------------------------------------------------------+  |  |
|  |  | [Globe] Self-Hosted Backup Nodes                                   |  |  |
|  |  |         manual · 3 nodes · 10/28/2026                         [ 🗑 ]|  |  |
|  |  +--------------------------------------------------------------------+  |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  Premium Subscription                                                    |  |
|  |  12 nodes · subscription · Active profile                                |  |
|  |  [Auto-update: 6h v]  [ ⟳ Refresh ]  [+ Add node]  [⚡ Test all]          |  |
|  |                                                                          |  |
|  |  +--------------------------------------------------------------------+  |  |
|  |  | [US] US-West-Reality                         [Connected] TCP: 42ms |  |  |
|  |  |      vless · 1.4 GB used                                           |  |  |
|  |  |      [ 🖧 Ping ] [ ⚡ Speed ] [ ✎ Edit ] [ 🗑 Delete ]                |  |  |
|  |  +--------------------------------------------------------------------+  |  |
|  |  +--------------------------------------------------------------------+  |  |
|  |  | [SG] SG-Direct-Hy2                                       TCP: 88ms |  |  |
|  |  |      hysteria2 · 420.0 MB used                                     |  |  |
|  |  |      [ 🖧 Ping ] [ ⚡ Speed ] [ ✎ Edit ] [ 🗑 Delete ]                |  |  |
|  |  +--------------------------------------------------------------------+  |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
+--------------------------------------------------------------------------------+
```

---

## 7. Interactive Controls & Accurate Labels

| Element Selector | Label / Display Text | Exact Behavior & State |
| :--- | :--- | :--- |
| `[+ Add profile]` | `Add profile` / `Close import` | Toggles `showImport` state. Switches icon from `add` to `close`. |
| `[Empty profile]` | `Empty profile` | Calls `createEmptyProfile(importName \|\| 'Manual Profile')`. Creates empty profile and selects it. |
| `[Choose file]` | `Choose file` | Opens native file dialog for `.json`, `.yaml`, `.yml`, `.txt`. Imports parsed configuration. |
| `[Preview nodes]` | `Preview nodes` / `Previewing…` | Calls `previewImportConfig(content)`. Shows node count, protocol badges, and sample names. |
| `[Auto-update select]`| `Auto-update: Off / 1h / 6h / 12h / 24h` | Updates background interval in `profileStore` and schedules periodic background fetch. |
| `[Test all nodes]` | `Test all nodes` / `Testing…` | When connected + API enabled: runs `proxy-url` round-trip latency. Otherwise runs direct TCP handshake. |
| `[🖧 Ping]` | `Test direct TCP for {tag}` | Probes TCP connection directly to `node.server:node.port`. Updates `latencyResults[tag].latencyMs`. |
| `[⚡ Speed]` | `Test proxy URL for {tag}` | Available when connected + Clash API enabled. Tests real end-to-end proxy round-trip latency. |
| `[✎ Edit]` | `Edit {tag}` | Available only for the active profile. Calls `openEditor(node)`, opening the NodeEditor slide-over. |
| `[🗑 Delete]` | `Delete {tag}` | Triggers `ask()` confirmation dialog: `"Delete node \"{tag}\"? This cannot be undone."` |

---

## 8. Diff Modal Accessibility & Focus Management

- **Mount Focus**: When `diffPreview` becomes active, focus moves automatically to the close button `[✕]`.
- **Focus Trapping**: Keyboard `Tab` / `Shift+Tab` cycles exclusively through interactive elements inside `dialogRef`.
- **Dismissal**: `Escape` key closes the preview and restores keyboard focus to the originating refresh button (`refreshTriggerRef`).
- **Expiration Safety**: Preview token enforces a strict 5-minute lifespan. Stale tokens are rejected by the backend.
