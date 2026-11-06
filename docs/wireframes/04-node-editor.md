---
title: "Wireframe 04 — Server Node Editor Drawer"
aliases: ["Node Editor Wireframe", "NodeEditor Drawer", "Server Config Sheet"]
tags:
  - wireframe
  - ui
  - node-editor
  - proxy-node
created: 2026-11-03
updated: 2026-11-05
---

# Wireframe 04 — Server Node Editor Drawer

> [!NOTE] Target geometry
> Breakpoint tables in this note describe the **current CSS**. The target is three size classes (Compact `<600`, Medium `600–839`, Expanded `≥840`) in a **1000×680** default / **720×520** minimum window with zero document scroll — see [[wireframes/00-window-and-grid]] (rules, migration map) and [[wireframes/09-fit-layouts]] (exact-scale drawing for this view). Work is tracked in [[specs/003-responsive-fit-layout/tasks]].

**Navigation**: [[00-Home-MOC]] | **Related Wireframes**: [[01-shell-navigation]] · [[02-dashboard]] · [[03-profiles]]

**Source Components**:
- Slide-Over Sheet: [NodeEditor.tsx](../../src/components/domain/NodeEditor.tsx)
- State Store: [nodeEditorStore.ts](../../src/stores/nodeEditorStore.ts)
- Form Controls: [FormInput.tsx](../../src/components/form/FormInput.tsx) · [FormSelect.tsx](../../src/components/form/FormSelect.tsx) · [switch.tsx](../../src/components/ui/switch.tsx) · [checkbox.tsx](../../src/components/ui/checkbox.tsx)
- Structural CSS: [material-theme.css](../../src/material-theme.css)

---

## 1. Geometry & Responsive Sheet Structure

The Node Editor is a global slide-over drawer mounted into `<App />` and managed by `nodeEditorStore`. It animates from the right edge over an opaque backdrop scrim:

| Layout Property | Desktop Tier (`> 700px`) | Narrow / Mobile Tier (`≤ 700px`) | Extreme Narrow (`≤ 440px`) |
| :--- | :--- | :--- | :--- |
| **Drawer Width** | Fixed `min(500px, 100vw)` | Full screen `100vw` | Full screen `100vw` |
| **Backdrop Scrim** | `position: fixed; inset: 0; background: rgb(0 0 0 / 52%);` | Same | Same |
| **Padding** | Header: `22px 24px 15px`, Body: `12px 24px 24px` | Header: `18px 16px 12px`, Body: `10px 16px 24px` | Same |
| **Grid Fields** | Multi-column (`grid-cols-2`, `grid-cols-3`) | Multi-column maintained | Single column stacked (`grid-cols-1`) |

---

## 2. Desktop Slide-Over Layout (`Basic` Tab — VLESS)

```text
                                            +----------------------------------------------------------+
                                            |  Edit server                         [VLESS]         [✕] |
                                            |----------------------------------------------------------|
                                            |  [ Basic ]*         [ Transport ]       [ TLS / Security]|
                                            |----------------------------------------------------------|
                                            |  COMMON                                                  |
                                            |  +----------------------------------------------------+  |
                                            |  | Node name (tag)                                    |  |
                                            |  | [ US-West-Reality                                ] |  |
                                            |  |                                                    |  |
                                            |  | Protocol type                                      |  |
                                            |  | [ VLESS                                         v] |  |
                                            |  |                                                    |  |
                                            |  | Server address                     | Port          |  |
                                            |  | [ 198.51.100.42                 ]  | [ 443      ]  |  |
                                            |  +----------------------------------------------------+  |
                                            |                                                          |
                                            |  AUTHENTICATION                                          |
                                            |  +----------------------------------------------------+  |
                                            |  | UUID                                               |  |
                                            |  | [ •••••••••••••••••••••••••••••••••••• ] [👁 Show] |  |
                                            |  |                                                    |  |
                                            |  | Flow (XTLS)                                        |  |
                                            |  | [ xtls-rprx-vision                              v] |  |
                                            |  +----------------------------------------------------+  |
                                            |----------------------------------------------------------|
                                            |                                  [ Cancel ]  [ Save Node]|
                                            +----------------------------------------------------------+
```

---

## 3. Desktop Slide-Over Layout (`Transport` Tab — WebSocket)

For stream-oriented protocols (`vless`, `vmess`, `trojan`, `shadowsocks`), the Transport tab configures underlying TCP/WS/gRPC layers:

```text
                                            +----------------------------------------------------------+
                                            |  Edit server                         [VLESS]         [✕] |
                                            |----------------------------------------------------------|
                                            |  [ Basic ]          [ Transport ]*      [ TLS / Security]|
                                            |----------------------------------------------------------|
                                            |  TRANSPORT LAYER                                         |
                                            |  +----------------------------------------------------+  |
                                            |  | Network transport                                  |  |
                                            |  | [ WebSocket                                     v] |  |
                                            |  |                                                    |  |
                                            |  | Path                         | Host header         |  |
                                            |  | [ /graphql                ]  | [ cdn.example.com]  |  |
                                            |  +----------------------------------------------------+  |
                                            |                                                          |
                                            |  (If Network is set to gRPC:)                            |
                                            |  +----------------------------------------------------+  |
                                            |  | Service name                                       |  |
                                            |  | [ GunService                                     ] |  |
                                            |  +----------------------------------------------------+  |
                                            |----------------------------------------------------------|
                                            |                                  [ Cancel ]  [ Save Node]|
                                            +----------------------------------------------------------+
```

---

## 4. Desktop Slide-Over Layout (`TLS / Security` Tab — XTLS Reality)

```text
                                            +----------------------------------------------------------+
                                            |  Edit server                         [VLESS]         [✕] |
                                            |----------------------------------------------------------|
                                            |  [ Basic ]          [ Transport ]       [ TLS / Security]*|
                                            |----------------------------------------------------------|
                                            |  TLS SECURITY                                            |
                                            |  +----------------------------------------------------+  |
                                            |  | Security mode                                      |  |
                                            |  | [ REALITY (XTLS)                                v] |  |
                                            |  |                                                    |  |
                                            |  | uTLS fingerprint                                   |  |
                                            |  | [ Chrome                                        v] |  |
                                            |  +----------------------------------------------------+  |
                                            |                                                          |
                                            |  TLS PARAMETERS                                          |
                                            |  +----------------------------------------------------+  |
                                            |  | SNI                          | ALPN                |  |
                                            |  | [ aka.ms                  ]  | [ h2, http/1.1   v] |  |
                                            |  |                                                    |  |
                                            |  | [ ] Allow Insecure (skip cert verification)        |  |
                                            |  +----------------------------------------------------+  |
                                            |                                                          |
                                            |  REALITY CONFIGURATION                                   |
                                            |  +----------------------------------------------------+  |
                                            |  | (i) REALITY impersonates a real TLS server.        |  |
                                            |  | Public key (pbk)                                   |  |
                                            |  | [ 1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f ] |  |
                                            |  | Short ID (sid)               | Destination SNI     |  |
                                            |  | [ 97c4e5fcb1e8            ]  | [ aka.ms          ] |  |
                                            |  +----------------------------------------------------+  |
                                            |----------------------------------------------------------|
                                            |                                  [ Cancel ]  [ Save Node]|
                                            +----------------------------------------------------------+
```

---

## 5. Non-Tabbed Layout: Specialized Protocols (WireGuard, SSH, SOCKS)

Protocols that do not use multi-tier transport/TLS abstractions (`isSpecial`: `wireguard`, `ssh`, `socks`, `http`) omit the tab bar and render all settings in a clean single-pane card:

```text
                                            +----------------------------------------------------------+
                                            |  Edit server                     [WireGuard]         [✕] |
                                            |----------------------------------------------------------|
                                            |  COMMON                                                  |
                                            |  +----------------------------------------------------+  |
                                            |  | Node name (tag)                                    |  |
                                            |  | [ Home-WireGuard                                 ] |  |
                                            |  | Protocol type: [ WireGuard                      v] |  |
                                            |  | Endpoint address             | Port                |  |
                                            |  | [ 198.51.100.99           ]  | [ 51820          ]  |  |
                                            |  +----------------------------------------------------+  |
                                            |                                                          |
                                            |  WIREGUARD INTERFACE & PEER                              |
                                            |  +----------------------------------------------------+  |
                                            |  | Private key (Interface)                            |  |
                                            |  | [ •••••••••••••••••••••••••••••••••••• ] [👁 Show] |  |
                                            |  | Local interface address                            |  |
                                            |  | [ 10.0.0.2/32, fd00::2/128                       ] |  |
                                            |  | Peer public key                                    |  |
                                            |  | [ a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v2=   ] |  |
                                            |  | Pre-shared key (Optional)                          |  |
                                            |  | [ [Optional PSK]                       ] [👁 Show] |  |
                                            |  | MTU                          | Reserved bytes      |  |
                                            |  | [ 1420                    ]  | [ [0,0,0]         ] |  |
                                            |  +----------------------------------------------------+  |
                                            |----------------------------------------------------------|
                                            |                                  [ Cancel ]  [ Save Node]|
                                            +----------------------------------------------------------+
```

---

## 6. Error & Validation States

When validation fails on `[ Save Node ]`, an error callout mounts immediately above the footer and scrolls into view:

```text
                                            |  +----------------------------------------------------+  |
                                            |  | [!] Reality Public Key required.                   |  |
                                            |  +----------------------------------------------------+  |
                                            |----------------------------------------------------------|
                                            |                                  [ Cancel ]  [ Save Node]|
                                            +----------------------------------------------------------+
```

### Comprehensive Validation Error Reference:

| Condition / Field | Validation Rule | Exact Error Message Displayed |
| :--- | :--- | :--- |
| **Node Tag** | `!state.editTag.trim()` | `"Tag name is required."` |
| **VLESS / VMess UUID** | `!state.editUuid.trim()` | `"UUID required."` |
| **REALITY Mode** | `!state.editPublicKey.trim()` | `"Reality Public Key required."` |
| **WireGuard Private Key** | `!state.editWgPrivateKey.trim()` | `"WireGuard private key is required."` |
| **WireGuard Peer Public Key**| `!state.editWgPeerPublicKey.trim()`| `"Peer public key is required."` |
| **WireGuard Reserved Bytes** | Invalid JSON parse | `"Reserved bytes must be a valid JSON array, e.g. [0,0,0]"` |
| **SSH Username** | `!state.editSshUser.trim()` | `"SSH username is required."` |
| **Backend Core Validation** | `sing-box check` failure | `"Failed to save server: {backend_error_string}"` |

---

## 7. Narrow Viewport Layout (`≤ 700px`)

On mobile and narrow desktop windows, the drawer expands to `100vw`, turning into a full-height dialog:

```text
+--------------------------------------------------------------------------------+
|  Edit server                                         [VLESS]               [✕] |
+--------------------------------------------------------------------------------+
|  [ Basic ]*                   [ Transport ]                  [ TLS / Security] |
+--------------------------------------------------------------------------------+
|                                                                                |
|  COMMON                                                                        |
|  +--------------------------------------------------------------------------+  |
|  | Node name (tag)                                                          |  |
|  | [ US-West-Reality                                                      ] |  |
|  |                                                                          |  |
|  | Protocol type                                                            |  |
|  | [ VLESS                                                               v] |  |
|  |                                                                          |  |
|  | Server address                                                           |  |
|  | [ 198.51.100.42                                                        ] |  |
|  |                                                                          |  |
|  | Port                                                                     |  |
|  | [ 443                                                                  ] |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
|  AUTHENTICATION                                                                |
|  +--------------------------------------------------------------------------+  |
|  | UUID                                                                     |  |
|  | [ •••••••••••••••••••••••••••••••••••• ] [👁 Show]                       |  |
|  |                                                                          |  |
|  | Flow (XTLS)                                                              |  |
|  | [ xtls-rprx-vision                                                    v] |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
+--------------------------------------------------------------------------------+
|                                                    [ Cancel ]      [ Save Node]|
+--------------------------------------------------------------------------------+
```

---

## 8. Keyboard Navigation, Accessibility & Focus Management

1. **Focus Initialization**: Opening the editor records `document.activeElement` into `returnFocusRef` and focuses the close button (`[aria-label="Close node editor"]`).
2. **Focus Restoration**: When the drawer closes (via Cancel, Save, or Escape), focus returns directly to the originating edit button.
3. **Focus Trapping**: A global keyboard listener traps `Tab` and `Shift+Tab` cycles strictly inside `drawerRef`.
4. **Escape Key Handling**: Pressing `Escape` invokes `closeEditor()` without submitting unverified edits.
5. **Secret Visibility Toggles**: All credential fields (UUID, Trojan/Shadowsocks/SSH passwords, WireGuard private keys) use `ToggleInput` with an eye button (`aria-pressed`, `aria-label="Show/Hide ..."`).
