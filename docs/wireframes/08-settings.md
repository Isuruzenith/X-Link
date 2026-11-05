---
title: "Wireframe 08 — Settings & Diagnostics"
aliases: ["Settings Wireframe", "SettingsView Wireframe", "Diagnostics Wireframe", "Configuration Backup"]
tags:
  - wireframe
  - ui
  - settings
  - configuration
  - diagnostics
  - tun
  - dns
created: 2026-11-05
updated: 2026-11-05
---

# Wireframe 08 — Settings & Diagnostics

**Navigation**: [[00-Home-MOC]] | **Related Wireframes**: [[01-shell-navigation]] · [[02-dashboard]] · [[03-profiles]] · [[04-node-editor]] · [[05-routing]] · [[06-connections]] · [[07-logs]]
**Source Components**:
- Primary View: [SettingsView.tsx](../../src/views/SettingsView.tsx)
- Container Shell: [ViewShell.tsx](../../src/components/ViewShell.tsx)
- State Stores: [settingsStore.ts](../../src/stores/settingsStore.ts) · [connectionStore.ts](../../src/stores/connectionStore.ts) · [profileStore.ts](../../src/stores/profileStore.ts) · [routingStore.ts](../../src/stores/routingStore.ts) · [toastStore.ts](../../src/stores/toastStore.ts)
- Settings & Restart Policies: [applyPolicy.ts](../../src/utils/applyPolicy.ts) · [connectedApply.ts](../../src/utils/connectedApply.ts) · [store.ts](../../src/utils/store.ts)
- Native IPC Commands: `simulate_routing` · `export_system_diagnostics` · `export_config_backup` · `validate_backup_archive` · `import_config_backup`
- Dialog Plugin: `@tauri-apps/plugin-dialog` (`ask`)
- Form & UI Controls: [FormInput.tsx](../../src/components/form/FormInput.tsx) · [FormSelect.tsx](../../src/components/form/FormSelect.tsx) · [FormSwitch.tsx](../../src/components/form/FormSwitch.tsx) · [SegmentedControl.tsx](../../src/components/form/SegmentedControl.tsx) · [Card.tsx](../../src/components/ui/card.tsx) · [Button.tsx](../../src/components/ui/button.tsx) · [Badge.tsx](../../src/components/ui/badge.tsx) · [Icon.tsx](../../src/components/ui/icon.tsx)
- Structural CSS: [material-theme.css](../../src/material-theme.css) · [index.css](../../src/index.css)

---

## 1. Grid Architecture & Responsive Breakpoints

The Settings interface (`.settings-layout`) organizes proxy operational parameters, core tuners, and diagnostic utilities into a split-pane layout consisting of a fixed secondary navigation rail on the left and a scrollable configuration viewport on the right:

| Breakpoint Tier | Container Width | Secondary Navigation Rail (`.settings-navigation`) | Content Viewport (`.settings-content`) | Card Internal Padding |
| :--- | :--- | :--- | :--- | :--- |
| **Desktop Wide** | `> 1050px` | Sticky vertical column (`170px`), full text labels and icons | 2-column CSS Grid (`grid-cols-1 xl:grid-cols-2`, gap: 16px) | `padding: 22px;` |
| **Tablet / Narrow** | `701px – 1050px` | Sticky vertical column (`170px`), full text labels and icons | Single column stacked (`grid-cols-1`), cards full width | `padding: 22px;` |
| **Mobile Narrow** | `≤ 700px` | Static horizontal scroll bar (`flex-row`, `overflow-x: auto`) | Full-width single column stack (`minmax(0, 1fr)`) | `padding: 18px;` |
| **Extreme Narrow** | `≤ 440px` | Horizontal chips, compact typography | Stacked inputs, wrapped buttons | `padding: 14px;` |

---

## 2. Master Settings Shell Layout (`> 1050px`)

The outer application shell wraps the view in `ViewShell` (`title="Settings"`, `subtitle="Proxy mode, TUN, sniffing, DNS and API options"`). Inside, `.settings-layout` divides the horizontal space into a 170px secondary navigation rail and a flexible content region:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Settings                                                                                                              |
|  Proxy mode, TUN, sniffing, DNS and API options                                                                        |
|------------------------------------------------------------------------------------------------------------------------|
|                                                                                                                        |
|  SECONDARY NAV (170px)  |  CONTENT VIEWPORT (.settings-content, id="settings-{tab}")                                   |
|  +-------------------+  |  +----------------------------------------------------------------------------------------+  |
|  | [*] General       |  |  | [ Active Tab Content Container ]                                                       |  |
|  +-------------------+  |  |                                                                                        |  |
|  | [ ] TUN Mode      |  |  |                                                                                        |  |
|  +-------------------+  |  |                                                                                        |  |
|  | [ ] Sniffing      |  |  |                                                                                        |  |
|  +-------------------+  |  |                                                                                        |  |
|  | [ ] DNS           |  |  |                                                                                        |  |
|  +-------------------+  |  |                                                                                        |  |
|  | [ ] API           |  |  |                                                                                        |  |
|  +-------------------+  |  |                                                                                        |  |
|  | [ ] Diagnostics   |  |  |                                                                                        |  |
|  +-------------------+  |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
+------------------------------------------------------------------------------------------------------------------------+
```

### Secondary Navigation Rail Items
Each navigation item is an accessible button (`.settings-nav-item`) with an icon, text label, and reactive `.is-active` state (`aria-current="page"` when active):
1. **General** (`icon="tune"`): Core proxy mode, system behaviors, connection multiplexing, appearance theme, inbound listener ports.
2. **TUN Mode** (`icon="router"`): Wintun driver configuration, packet stack implementations, route table injection, NAT settings.
3. **Sniffing** (`icon="visibility"`): Deep packet inspection, protocol extractors (HTTP/TLS/QUIC), destination domain overrides.
4. **DNS** (`icon="dns"`): Proxy, fallback, and direct resolver endpoints, query strategies, FakeIP pool allocation, LAN bypass.
5. **API** (`icon="code"`): Clash-compatible HTTP REST controller, port bindings, authentication secret token, CORS headers.
6. **Diagnostics** (`icon="troubleshoot"`): System health telemetry, sanitized diagnostics exporter, backup/restore, routing simulator.

---

## 3. Tab 1 — General Settings

The General tab contains proxy mode selection, system lifecycle options, connection multiplexing, theme controls, and inbound port configuration:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Settings                                                                                                              |
|  Proxy mode, TUN, sniffing, DNS and API options                                                                        |
|------------------------------------------------------------------------------------------------------------------------|
|                         |                                                                                              |
|  [*] General            |  +--------------------------------------------+  +----------------------------------------+  |
|  [ ] TUN Mode           |  |  Proxy Mode                                |  |  Inbound Ports                         |  |
|  [ ] Sniffing           |  |--------------------------------------------|  |----------------------------------------|  |
|  [ ] DNS                |  |  Proxy Mode: TUN vs System Proxy      (o)  |  |  Use Separate HTTP & SOCKS5 Ports  ( )  | |
|  [ ] API                |  |  Toggle between native routing (TUN) and   |  |  Bind separate HTTP and SOCKS5         |  |
|  [ ] Diagnostics        |  |  OS settings (System Proxy).               |  |  protocol listeners                    |  |
|                         |  |  [!] Mode change applies on next connect.  |  |                                        |  |
|                         |  +--------------------------------------------+  |  HTTP Proxy Port           * Free      |  |
|                         |                                                  |  [ 7890                              ] |  |
|                         |  +--------------------------------------------+  |  (disabled/dimmed when separate off)   |  |
|                         |  |  App Preferences                           |  |                                        |  |
|                         |  |--------------------------------------------|  |  SOCKS5 Port               * Free      |  |
|                         |  |  Close to Tray                        (o)  |  |  [ 7891                              ] |  |
|                         |  |  Minimize to system tray on close          |  |  (disabled/dimmed when separate off)   |  |
|                         |  |                                            |  |                                        |  |
|                         |  |  Launch on System Login               ( )  |  |  Mixed inbound port (rec.) * X-Link    |  |
|                         |  |  Launch minimized on user login            |  |  [ 7892                              ] |  |
|                         |  |                                            |  |                                        |  |
|                         |  |  LAN Hotspot Sharing                  ( )  |  |  Port changes restart connection when  |  |
|                         |  |  Bind to 0.0.0.0 for LAN device sharing    |  |  connected (validated cold restart).   |  |
|                         |  |                                            |  +----------------------------------------+  |
|                         |  |  Node Geolocation Lookup              ( )  |                                              |
|                         |  |  Resolve country flags via freeipapi.com   |  +----------------------------------------+  |
|                         |  +--------------------------------------------+  |  [Logo] X-Link                 [v0.8.0]  ||
|                         |                                                  |         Empowered by sing-box          |  |
|                         |  +--------------------------------------------+  +----------------------------------------+  |
|                         |  |  Connection Optimization                   |                                              |
|                         |  |--------------------------------------------|                                              |
|                         |  |  Enable Multiplexing (Mux)            ( )  |                                              |
|                         |  |  Multiplex streams over single connection  |                                              |
|                         |  +--------------------------------------------+                                              |
|                         |                                                                                              |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |  |  Appearance                                                                            |  |
|                         |  |  [ Light ]                         [ Dark ]*                       [ System ]          |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
+------------------------------------------------------------------------------------------------------------------------+
```

### UI Elements & Behavior:
- **Proxy Mode**: Master switch toggling between `tun` (native virtual adapter) and `system` (OS proxy settings). Toggling to TUN displays a warning toast reminding the user that Administrator privileges and Wintun driver are required. When connected, changes defer to next connect (`classifySetting: 'defer'`).
- **Inbound Ports**: Dual mode controlled by `useSeparatePorts`:
  - When disabled: HTTP and SOCKS5 inputs are dimmed (`opacity-50`) and disabled; Mixed inbound port (`mixedPort`, default `7892`) handles both protocols.
  - When enabled: Mixed port input is dimmed; HTTP Port (`httpPort`, default `7890`) and SOCKS5 Port (`socksPort`, default `7891`) are active.
  - Port status indicator badges update live: `* Free`, `* Conflict`, `* Invalid`, `* X-Link`, or `* Unchecked`.

---

## 4. Tab 2 — TUN Mode Configuration & Elevation Status

The TUN Mode tab manages virtual network adapter parameters, IP routing table hooks, packet encapsulation stacks, and NAT behavior:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Settings                                                                                                              |
|  Proxy mode, TUN, sniffing, DNS and API options                                                                        |
|------------------------------------------------------------------------------------------------------------------------|
|                         |                                                                                              |
|  [ ] General            |  +----------------------------------------------------------------------------------------+  |
|  [*] TUN Mode           |  |  [!] TUN Mode requires Administrator elevation. Click "Run as Admin" on Dashboard.     |  |
|  [ ] Sniffing           |  +----------------------------------------------------------------------------------------+  |
|  [ ] DNS                |                                                                                              |
|  [ ] API                |  +----------------------------------------------------------------------------------------+  |
|  [ ] Diagnostics        |  |  TUN Interface Configuration                                                           |  |
|                         |  |----------------------------------------------------------------------------------------|  |
|                         |  |  TUN stack implementation                     |  TUN MTU                               |  |
|                         |  |  [ Mixed (gVisor TCP + system UDP)         v] |  [ 1500                              ] |  |
|                         |  |  Mixed: best compat. gVisor: best isolation.    |  1400 recommended. 1500 for ethernet.  ||
|                         |  |----------------------------------------------------------------------------------------|  |
|                         |  |  Auto Route                                                                       (o)  |  |
|                         |  |  Add routes to direct traffic into TUN interface                                      |   |
|                         |  |                                                                                        |  |
|                         |  |  Auto Redirect (nftables)                                                         ( )  |  |
|                         |  |  Use nftables-based redirect on Linux                                                  |  |
|                         |  |                                                                                        |  |
|                         |  |  Strict Route                                                                      (o)  | |
|                         |  |  Block unmatched traffic to prevent leaks                                              |  |
|                         |  |                                                                                        |  |
|                         |  |  Endpoint Independent NAT                                                         ( )  |  |
|                         |  |  Improves UDP NAT traversal (P2P, gaming)                                              |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |  |  (i) TUN mode captures ALL system traffic. Combined with Auto Route + Strict Route,    |  |
|                         |  |      it prevents DNS leaks. Interface changes apply via validated cold restart.        |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
+------------------------------------------------------------------------------------------------------------------------+
```

### Elevation & Safety Invariants:
- **Elevation Banner**: Rendered in `bg-destructive/10 text-destructive` when `!isElevated`. Prompts user to elevate to Administrator via Dashboard.
- **Elevation Gating**: When `!isElevated`, all TUN toggle switches (`Auto Route`, `Auto Redirect`, `Strict Route`, `Endpoint Independent NAT`) are disabled to prevent invalid driver states.
- **TUN Stacks**:
  - `mixed`: Uses gVisor for TCP streams and kernel networking for UDP datagrams. Recommended default for compatibility.
  - `gvisor`: Full userspace network stack. Highest security and namespace isolation.
  - `system`: Kernel-level routing stack for lowest CPU overhead.

---

## 5. Tab 3 — Traffic Sniffing & Protocol Detection

The Sniffing tab controls deep packet inspection of outbound streams to extract destination hostnames and Server Name Indication (SNI) for rule-based routing:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Settings                                                                                                              |
|  Proxy mode, TUN, sniffing, DNS and API options                                                                        |
|------------------------------------------------------------------------------------------------------------------------|
|                         |                                                                                              |
|  [ ] General            |  +----------------------------------------------------------------------------------------+  |
|  [ ] TUN Mode           |  |  Traffic Sniffing & Protocol Detection                                                 |  |
|  [*] Sniffing           |  |----------------------------------------------------------------------------------------|  |
|  [ ] DNS                |  |  Enable Traffic Sniffing                                                          (o)  |  |
|  [ ] API                |  |  Inspect connection metadata to extract domain names for routing                       |  |
|  [ ] Diagnostics        |  |----------------------------------------------------------------------------------------|  |
|                         |  |  HTTP Sniffing                                 TLS/HTTPS Sniffing                     |   |
|                         |  |  Extract hostname from HTTP Host header  (o)  Extract SNI from TLS ClientHello    (o)  |  |
|                         |  |                                                                                        |  |
|                         |  |  QUIC/HTTP3 Sniffing                           Override Destination                   |   |
|                         |  |  Extract SNI from QUIC Initial packets   (o)  Use sniffed domain to override IP   ( )  |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |  |  [!] Traffic sniffing changes are applied with a validated restart while connected;    |  |
|                         |  |      on failure the previous values are restored.                                      |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
+------------------------------------------------------------------------------------------------------------------------+
```

### Configuration Switches:
- **Master Toggle (`sniffEnabled`)**: When disabled, all subordinate protocol toggles are disabled and visually dimmed (`opacity-50`).
- **HTTP Sniffing (`sniffHttp`)**: Reads plaintext HTTP `Host:` headers on port 80.
- **TLS Sniffing (`sniffTls`)**: Extracts SNI hostname from initial TLS `ClientHello` handshake on port 443.
- **QUIC Sniffing (`sniffQuic`)**: Parses initial UDP QUIC packets for HTTP/3 destinations.
- **Override Destination (`sniffOverrideDestination`)**: Overwrites the original target IP with the sniffed hostname for outbound routing evaluation.

---

## 6. Tab 4 — DNS Configuration, Strategy & Resolution Mode

The DNS tab configures remote upstream resolvers, local direct resolvers, query address family strategies, and FakeIP pools:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Settings                                                                                                              |
|  Proxy mode, TUN, sniffing, DNS and API options                                                                        |
|------------------------------------------------------------------------------------------------------------------------|
|                         |                                                                                              |
|  [ ] General            |  +----------------------------------------------------------------------------------------+  |
|  [ ] TUN Mode           |  |  DNS Servers                                                                           |  |
|  [ ] Sniffing           |  |----------------------------------------------------------------------------------------|  |
|  [*] DNS                |  |  Primary DNS (via proxy)        Fallback DNS (via proxy)       Direct DNS (local/bypass)| |
|  [ ] API                |  |  [ https://1.1.1.1/dns-query  ] [ https://8.8.8.8/dns-query  ] [ Auto (System DNS)     ] ||
|  [ ] Diagnostics        |  |  [Cloudflare DoH] [Google DoH]  [Google DoH] [Quad9 DoH]       [System (Auto)]          | |
|                         |  |  [Cloudflare DoT] [AdGuard DoQ] [Google DoT] [Cloudflare UDP]  [Cloudflare] [Google]    | |
|                         |  |  Supports DoH (https://), DoT (tls://), DoQ (quic://), TCP (tcp://), Traditional UDP. |   |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |  |  DNS Options & Strategy                                                                |  |
|                         |  |----------------------------------------------------------------------------------------|  |
|                         |  |  DNS query strategy                 |  Enable DNS Caching                         (o)  |  |
|                         |  |  [ Prefer IPv4 (Default)         v] |  Store resolved domain IPs locally               |  |
|                         |  |                                     |                                                  |  |
|                         |  |                                     |  DNS Leak Protection                        (o)  |  |
|                         |  |                                     |  Block DNS requests outside the secure tunnel    |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |  |  Resolution Mode                                                                       |  |
|                         |  |----------------------------------------------------------------------------------------|  |
|                         |  |  DNS mode: [ FakeIP (recommended for TUN)                                           v] |  |
|                         |  |  FakeIP avoids leaking real destination IPs and speeds up routing decisions.           |  |
|                         |  |                                                                                        |  |
|                         |  |  FakeIP range                         |  FakeIP exclude filter                         |  |
|                         |  |  [ 198.18.0.0/15                    ] |  [ geosite:private                           ] |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |  |  Routing                                                                               |  |
|                         |  |  Bypass LAN / Private IPs                                                         (o)  |  |
|                         |  |  Route RFC-1918 addresses directly instead of through the proxy                        |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |  |  (i) While connected, DNS changes are applied with a validated restart; on failure     |  |
|                         |  |      the previous values are restored.                                                 |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
+------------------------------------------------------------------------------------------------------------------------+
```

### Quick Preset Buttons:
- **Primary DNS**: Quick pills for `Cloudflare DoH` (`https://1.1.1.1/dns-query`), `Google DoH` (`https://dns.google/dns-query`), `Cloudflare DoT` (`tls://1.1.1.1:853`), and `AdGuard DoQ` (`quic://dns.adguard-dns.com:853`).
- **Fallback DNS**: Quick pills for `Google DoH`, `Quad9 DoH` (`https://dns.quad9.net/dns-query`), `Google DoT` (`tls://8.8.8.8:853`), and `Cloudflare UDP` (`1.1.1.1:53`).
- **Direct DNS**: Quick pills for `System (Auto)` (empty string), `Cloudflare` (`1.1.1.1:53`), and `Google` (`8.8.8.8:53`).

---

## 7. Tab 5 — External Clash REST API

The API tab configures an embedded Clash-compatible HTTP control server for third-party web dashboards (such as Yacd or MetaCubeX):

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Settings                                                                                                              |
|  Proxy mode, TUN, sniffing, DNS and API options                                                                        |
|------------------------------------------------------------------------------------------------------------------------|
|                         |                                                                                              |
|  [ ] General            |  +----------------------------------------------------------------------------------------+  |
|  [ ] TUN Mode           |  |  Clash-Compatible REST API                                                             |  |
|  [ ] Sniffing           |  |----------------------------------------------------------------------------------------|  |
|  [ ] DNS                |  |  Enable External API                                                              (o)  |  |
|  [*] API                |  |  Expose HTTP API for external controllers (Yacd, MetaCubeX)                            |  |
|  [ ] Diagnostics        |  |----------------------------------------------------------------------------------------|  |
|                         |  |  API Port (140px)  |  API secret token                                                 |  |
|                         |  |  [ 9090         ]  |  [ 4f9b8c2e1d7a6f5e0b3c8d1a...                   ]  [ Refresh ]   |  |
|                         |  |                    |  (Secret token required for authentication)                       |  |
|                         |  |----------------------------------------------------------------------------------------|  |
|                         |  |  Allow CORS (Cross-Origin)                                                        (o)  |  |
|                         |  |  Allow web-based controllers (Yacd) to connect from browser                           |   |
|                         |  |----------------------------------------------------------------------------------------|  |
|                         |  |  [v] API active at http://127.0.0.1:9090 -- open in Yacd UI [>]       [ Copy Token ]    | |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |  |  (i) The Clash-compatible API lets Yacd or MetaCubeX dashboard control routing and     |  |
|                         |  |      monitor connections in real-time.                                                |   |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
+------------------------------------------------------------------------------------------------------------------------+
```

### Authentication & Token Validation States:
1. **Token Generation**: Clicking `[ Refresh ]` (`.settings-api-generate`) generates a secure pseudorandom secret via `generateApiSecret()`.
2. **Missing Token Warning**: If API is enabled with an empty secret, an alert appears:
```text
+----------------------------------------------------------------------------------------+
|[!] API secret token is required. Controller will not start without authentication.     |
+----------------------------------------------------------------------------------------+
```
3. **Active Controller Banner**: When connected and token is valid:
   - Displays `http://127.0.0.1:{apiPort}` with direct link to `https://yacd.haishan.me/`.
   - Includes notice `(Requires "Allow CORS" for web dashboard)` if `apiCors` is false.
   - Provides `[ Copy Token ]` button for clipboard extraction.

---

## 8. Tab 6 — Diagnostics, Backup & Simulation

The Diagnostics tab bundles three specialized tools: real-time system health inspection, privacy-safe diagnostics export, full configuration backup/restore, and a local routing rule simulator:

```text
+------------------------------------------------------------------------------------------------------------------------+
|  Settings                                                                                                              |
|  Proxy mode, TUN, sniffing, DNS and API options                                                                        |
|------------------------------------------------------------------------------------------------------------------------|
|                         |                                                                                              |
|  [ ] General            |  +----------------------------------------------------------------------------------------+  |
|  [ ] TUN Mode           |  |  System Health & Bundle                                            [ Privacy Safe ]    |  |
|  [ ] Sniffing           |  |----------------------------------------------------------------------------------------|  |
|  [ ] DNS                |  |  Export a sanitized diagnostic snapshot of core system services, port bindings,        |  |
|  [ ] API                |  |  sing-box sidecar health, and redacted log excerpts.                                   |  |
|  [*] Diagnostics        |  |                                                                                        |  |
|                         |  |  +------------------+  +------------------+  +------------------+  +------------------+  ||
|                         |  |  | ELEVATION        |  | PROXY CORE       |  | CURRENT MODE     |  | PORT CONFLICTS   |  ||
|                         |  |  | [#] Administrator|  | [*] Running      |  | TUN              |  | [v] None detected|  ||
|                         |  |  +------------------+  +------------------+  +------------------+  +------------------+  ||
|                         |  |                                                                                        |  |
|                         |  |  [ Generate Diagnostics ]  [ Copy JSON ]  [ Download Report ]  [ Hide Preview ]        |  |
|                         |  |                                                                                        |  |
|                         |  |  Diagnostics Preview (10:14:02 AM)                                         v Sanitized |  |
|                         |  |  +----------------------------------------------------------------------------------+  |  |
|                         |  |  | {                                                                                |  |  |
|                         |  |  |   "timestamp": 1730000000,                                                       |  |  |
|                         |  |  |   "appVersion": "0.8.0",                                                         |  |  |
|                         |  |  |   "os": { "platform": "windows", "arch": "x86_64", "isElevated": true },         |  |  |
|                         |  |  |   "sidecar": { "bundledTestedVersion": "1.13.14", "isCompatible": true },        |  |  |
|                         |  |  |   "proxy": { "status": "running", "effectiveMode": "tun", "tunSupported": true },|  |  |
|                         |  |  |   "ports": { "mixedPort": 7892, "portStatuses": { "7892": "bound" } },           |  |  |
|                         |  |  |   "profiles": { "profileCount": 2, "activeProfileType": "remote" },              |  |  |
|                         |  |  |   "recentLogs": [ "[INFO] Core started", "[INFO] Secret: [REDACTED]" ]           |  |  |
|                         |  |  | }                                                                                |  |  |
|                         |  |  +----------------------------------------------------------------------------------+  |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |  |  Configuration Backup & Restore                                 [ Contains secrets ]   |  |
|                         |  |----------------------------------------------------------------------------------------|  |
|                         |  |  Backups include subscription URLs, proxy passwords and API tokens. Keep private.      |  |
|                         |  |  Disconnect before restoring; restart the app before reconnecting.                      | |
|                         |  |                                                                                        |  |
|                         |  |  [ Export backup ]   [ Inspect backup file ]                                           |  |
|                         |  |                                                                                        |  |
|                         |  |  +----------------------------------------------------------------------------------+  |  |
|                         |  |  |  Backup from X-Link 0.8.0                                             [ Dismiss ]|  |  |
|                         |  |  |  2 profiles . 6 routing rules . 2 rule sets                                      |  |  |
|                         |  |  |  Profiles: Premium Sub, Self-Hosted                                              |  |  |
|                         |  |  |                                                                                  |  |  |
|                         |  |  |  Sections to restore:                                                            |  |  |
|                         |  |  |  [x] Profiles and node configs    [x] App settings    [x] Routing rules          |  |  |
|                         |  |  |                                                                                  |  |  |
|                         |  |  |  [!] Restoring overwrites local sections. Missing profiles removed from list.   |  |   |
|                         |  |  |  [ Restore selected sections ]                                                   |  |  |
|                         |  |  +----------------------------------------------------------------------------------+  |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |  |  Routing Rule & DNS Simulation                                             [ Simulate ]|  |
|                         |  |----------------------------------------------------------------------------------------|  |
|                         |  |  Test how destinations resolve through X-Link routing logic and active outbounds.      |  |
|                         |  |                                                                                        |  |
|                         |  |  Destination (Domain or IP)                                                            |  |
|                         |  |  [ 192.168.1.1                                              ]  [ Simulate ]            |  |
|                         |  |                                                                                        |  |
|                         |  |  +----------------------------------------------------------------------------------+  |  |
|                         |  |  | 192.168.1.1  [ IP ]                                  Target Outbound: [ DIRECT ] |  |  |
|                         |  |  |----------------------------------------------------------------------------------|  |  |
|                         |  |  | Match Reason                     Rule Matched            Network Scope / DNS     |  |  |
|                         |  |  | Private IPv4 address matches     Default Fallback        LAN / Direct Bypass     |  |  |
|                         |  |  | LAN bypass rule                                          (prefer_ipv4)           |  |  |
|                         |  |  +----------------------------------------------------------------------------------+  |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |  |  (i) Diagnostics help troubleshoot routing loops, port collisions, and DNS issues.     |  |
|                         |  +----------------------------------------------------------------------------------------+  |
|                         |                                                                                              |
+------------------------------------------------------------------------------------------------------------------------+
```

### Sub-Features in Diagnostics:

#### 8.1 System Health & Sanitized Report Preview
- **4-Metric Health Grid**:
  1. `Elevation`: `[#] Administrator` or `[ ] Standard User`
  2. `Proxy Core`: `[*] Running` (green) or `[ ] Stopped` (muted)
  3. `Current Mode`: `TUN` or `SYSTEM` (uppercase mono)
  4. `Port Conflicts`: `[v] None detected` (green) or `[!] N conflict(s)` (amber)
- **Sanitized Export & Redaction**: Cleans all Bearer tokens, passwords, subscription keys, and secret hashes. `[REDACTED]` replaces sensitive log tokens.
- **Actions**:
  - `Generate Diagnostics`: Calls native command `export_system_diagnostics` and populates preview.
  - `Copy JSON`: Writes pretty-printed JSON directly to system clipboard with confirmation toast.
  - `Download Report`: Generates `xlink-diagnostics-{timestamp}.json` download anchor.
  - `Show / Hide Preview`: Toggles scrollable mono `<pre>` preview block (`max-h-60`).

#### 8.2 Configuration Backup & Restore Flow
- **Archive Security Note**: Marked with `[ Contains secrets ]` badge to emphasize private credentials in JSON.
- **Inspection Lifecycle**:
  1. User selects file via `[ Inspect backup file ]` (file input limited to `.json,application/json`, max 32 MB).
  2. Calls native validator `validate_backup_archive`.
  3. Renders preview showing app version, profile counts, routing rule counts, and profile tags.
  4. User selectively toggles checkboxes: `restoreProfiles`, `restoreSettings`, `restoreRouting`.
  5. Restore is locked if proxy is connected: `[!] Disconnect the proxy to enable restore.`
  6. Clicking `[ Restore selected sections ]` triggers warning confirmation dialog via `@tauri-apps/plugin-dialog` (`ask`).
  7. On success, reloads all stores (`settings`, `profiles`, `routing`) and raises confirmation toast.

#### 8.3 Routing Rule & DNS Simulation
- **Input**: Accepts IPv4, IPv6, CIDR blocks, or fully qualified domain names (`192.168.1.1`, `google.com`, `10.0.0.0/8`).
- **Evaluation**: Calls native IPC command `simulate_routing`.
- **Result Metrics**:
  - `Destination`: Evaluated destination with `[ IP ]` or `[ DOMAIN ]` badge.
  - `Target Outbound`: Color-coded badge (`[ DIRECT ]` secondary, `[ BLOCK ]` destructive, `[ PROXY ]` primary).
  - `Match Reason`: Textual explanation of routing logic branch.
  - `Rule Matched`: Specific rule type and ID, or `Default Fallback`.
  - `Network Scope / DNS`: LAN bypass flag and active DNS resolution strategy (`prefer_ipv4`, `prefer_ipv6`).

---

## 9. Validation, Port Status & Elevation States

### 9.1 Port Validation & Status Badges
Ports are validated in real-time through `PortField` (`min={1}`, `max={65535}`, integer regex):

| Port State | Badge Indicator | Visual Styling & State Meaning |
| :--- | :--- | :--- |
| **Port Available** | `* Free` | Muted subtle dot (`text-foreground/60`), ready to bind |
| **Port Bound by X-Link** | `* X-Link` | Active session listener dot (`text-foreground/60`) |
| **Port Conflict (External)**| `* Conflict` | Destructive alert (`text-destructive`), occupied by another process |
| **Unchecked / In Flight** | `* Unchecked` | Warning amber dot (`text-amber-500`), probe pending |
| **Out of Range (<1,>65535)**| `* Invalid` | Destructive alert (`text-destructive`), invalid port integer |

#### Inline Draft Validation Error
When an invalid value is typed, input commits are deferred and an alert span mounts below the field:

```text
+----------------------------------------------------------------------------------------+
|HTTP Proxy Port                                                            * Invalid    |
|[ 70000                                                                                ]|
|Enter a port from 1 to 65535.                                                           |
+----------------------------------------------------------------------------------------+
```

### 9.2 Connected Restart Policy
Settings updates while connected to a proxy session are governed by `applyPolicy.ts`:
- **Restart Class (`'restart'`)**: Ports, MTU, DNS, sniffing, API. Triggers debounced (800ms) validated cold restart.
```text
+----------------------------------------------------------------------------------------+
|[!] Port and listener changes are applied with a validated restart while connected;     |
|    on failure the previous values are restored.                                        |
+----------------------------------------------------------------------------------------+
```
- **Defer Class (`'defer'`)**: `proxyMode` preference changes defer to the next manual connection without restarting.
```text
+----------------------------------------------------------------------------------------+
|[!] Proxy mode is a saved preference; the running session keeps its current mode until  |
|    you reconnect.                                                                      |
+----------------------------------------------------------------------------------------+
```
- **Live Class (`'live'`)**: `theme`, `closeToTray`, `autostart`, `geoLookupEnabled`. Applied immediately in UI.

### 9.3 Backup Restore Safety Guard
When the proxy connection is active, restore operations are blocked to prevent process corruption:

```text
+----------------------------------------------------------------------------------------+
|[ Restore selected sections (disabled) ]                                                |
|Disconnect the proxy to enable restore.                                                 |
+----------------------------------------------------------------------------------------+
```

---

## 10. Mobile Responsive Arrangement (`≤ 700px`)

At `≤ 700px` (matching the breakpoint in `material-theme.css`), the fixed vertical rail switches to an auto-scrolling horizontal chip bar, and two-column grids collapse into a single stacked column:

```text
+--------------------------------------------------------------------------------+
|  [ = ]    Settings                                                             |
|           Proxy mode, TUN, sniffing, DNS and API options                       |
|--------------------------------------------------------------------------------|
|  HORIZONTAL TABS (.settings-navigation, overflow-x: auto)                      |
|  [ General ]*  [ TUN Mode ]  [ Sniffing ]  [ DNS ]  [ API ]  [ Diagnostics ]   |
|--------------------------------------------------------------------------------|
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  Proxy Mode                                                              |  |
|  |--------------------------------------------------------------------------|  |
|  |  Proxy Mode: TUN vs System Proxy                                    (o)  |  |
|  |  Toggle between native routing (TUN) and OS settings                     |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  App Preferences                                                         |  |
|  |--------------------------------------------------------------------------|  |
|  |  Close to Tray                                                      (o)  |  |
|  |  Minimize to system tray on close                                        |  |
|  |                                                                          |  |
|  |  Launch on System Login                                             ( )  |  |
|  |  Launch minimized on user login                                          |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  Inbound Ports                                                           |  |
|  |--------------------------------------------------------------------------|  |
|  |  Use Separate HTTP & SOCKS5 Ports                                   ( )  |  |
|  |                                                                          |  |
|  |  Mixed inbound port (recommended)                               * X-Link |  |
|  |  [ 7892                                                                ] |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  Appearance                                                              |  |
|  |  [ Light ]                  [ Dark ]*                  [ System ]        |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
|  +--------------------------------------------------------------------------+  |
|  |  [Logo] X-Link                                                 [v0.8.0]  |  |
|  |         Empowered by sing-box                                            |  |
|  +--------------------------------------------------------------------------+  |
|                                                                                |
+--------------------------------------------------------------------------------+
```

### Responsive Adaptations:
1. **Secondary Nav**: `.settings-layout` adopts `grid-template-columns: minmax(0, 1fr)`. `.settings-navigation` adopts `flex-direction: row; overflow-x: auto; position: static;`.
2. **Pill Buttons**: `.settings-nav-item` sets `flex: none; width: auto;` with smooth horizontal touch-scrolling.
3. **Card Padding**: Reduced from `22px` to `18px` (`.settings-content .material-card`).
4. **Stacked Form Controls**: DNS 3-column server inputs, port fields, and diagnostics metric cards collapse from horizontal multi-column grids to full-width vertical stacks.
5. **Simulation Bar**: Destination input and `[ Simulate ]` button stack vertically (`flex-col sm:flex-row`).
