---
title: "04 — UI Wireframes & Screen Layouts"
aliases: ["UI Wireframes", "Screen Layouts", "Wireframes"]
tags:
  - wireframe
  - ui
  - frontend
created: 2026-11-03
updated: 2026-11-03
---

# 04 — UI Wireframes & Screen Layouts

**Navigation**: [[00-Home-MOC]] | **Prev**: [[03-UI-Components-and-Views]] | **Next**: [[05-Rust-Backend-Architecture]] | **Related**: [[02-Frontend-UI-Architecture]]

> [!NOTE] Window Geometry
> Default native window dimensions (`src-tauri/tauri.conf.json`): **1080 × 700 px** (Minimum: **900 × 620 px**).
> The layout is divided into a fixed **220px Left Navigation Rail (`NavRail`)** and a flexible **860px+ Main Viewport (`ViewShell`)**.

---

## 1. Master Application Shell (`App.tsx` + `NavRail.tsx`)

```text
+------------------------------------------------------------------------------------------------------+
| [X] X-Link                                                                             [_] [□] [✕]   |
+----------------------+-------------------------------------------------------------------------------+
|  [Logo] X-Link       |  VIEW HEADER (ViewShell.tsx)                                                  |
|         Private      |  Title                                          [Header Action Controls...]   |
|         Tunnel       |  Subtitle                                                                     |
|                      +-------------------------------------------------------------------------------+
|  NAVIGATION          |                                                                               |
|  +----------------+  |                                                                               |
|  | (*) Dashboard  |  |                                                                               |
|  +----------------+  |                                                                               |
|  | ( ) Profiles   |  |                                                                               |
|  | ( ) Routing    |  |                         MAIN VIEW BODY                                        |
|  | ( ) Connections|  |                   (ScrollArea or Flex Grid)                                   |
|  | ( ) Logs       |  |                                                                               |
|  | ( ) Settings   |  |                                                                               |
|  +----------------+  |                                                                               |
|                      |                                                                               |
|                      |                                                                               |
|                      |                                          +----------------------------------+ |
|  +----------------+  |                                          | [✓] Connection Established   [x] | |
|  | ● Connected TUN|  |                                          | Secured via Mixed port 7892      | |
|  | [🇺🇸] US-West-1 |  |                                          | ========================-------- | |
|  +----------------+  |                                          +----------------------------------+ |
|  sing-box v1.13.14   |                                                        (ToastContainer.tsx)   |
|  X-Link v0.6.0       |                                                                               |
+----------------------+-------------------------------------------------------------------------------+
```

---

## 2. View 1: Dashboard (`DashboardView.tsx`)

```text
+------------------------------------------------------------------------------------------------------+
|  Dashboard                                     [Profile: Premium Sub v] [Mode: TUN] [● Protected 01:24:10]
|  Connect, switch servers and watch live traffic                                                      |
+------------------------------------------------------------------------------------------------------+
|  [!] (Conditional Banner) TUN mode needs Administrator rights; connecting now will use System Proxy  |
|                                                                                     [ Run as Admin ] |
+------------------------------------------------------------------------------------------------------+
|  HERO CONTROL STRIP (.dashboard-control-strip)                                                       |
|  +----------------------------------------+--------------------------------------------------------+ |
|  |  +--------------+                      |  Download        |  Upload          |  Active | Engine | |
|  |  |   ( ⏻ )      |  Connected           |  14.2 MB/s       |  1.8 MB/s        |  42     | 38 MB  | |
|  |  | Disconnect   |  🇺🇸 US-West-Reality  |  1.42 GB total   |  184.5 MB total  |  :7892  | 1.2%   | |
|  |  +--------------+  [⚡ URL Test: 84ms] |                  |                  |  Mixed  | PID 912| |
|  +----------------------------------------+--------------------------------------------------------+ |
|                                                                                                      |
|  MAIN SPLIT GRID (.dashboard-main-grid)                                                              |
|  +----------------------------------------------------+--------------------------------------------+ |
|  |  Servers [12]       [🔍 Filter servers...] [⚡ Ping]|  Throughput       ↓ 14.2 MB/s  ↑ 1.8 MB/s  | |
|  +----------------------------------------------------+--------------------------------------------+ |
|  |  Server                   | Protocol | Latency     |   20 MB/s | - - - - - - - - - - - - - -●   | |
|  | --------------------------+----------+------------ |           |          ╭───╮        ╭───╯    | |
|  | ● 🇺🇸 US-West-Reality      | [VLESS]  | 84ms URL    |   10 MB/s | - - ╭───╯   ╰────────╯         | |
|  |   1.6 GB used             |          | [TCP][URL]✎🗑|           | ────╯                          | |
|  | --------------------------+----------+------------ |    0 B/s  +----------------------------●   | |
|  | ○ 🇸🇬 SG-Direct-Hy2        | [HY2]    | 42ms URL    |                        (TrafficChart.tsx)  | |
|  |   420 MB used             |          | [TCP][URL]✎🗑+--------------------------------------------+ |
|  | --------------------------+----------+------------ |  Session               [Reset Usage Stats] | |
|  | ○ 🇯🇵 Tokyo-Trojan-WS      | [TROJAN] | 118ms TCP   +--------------------------------------------+ |
|  |                           |          | [TCP][URL]✎🗑|  Profile           Premium Sub             | |
|  | --------------------------+----------+------------ |  Selected server   US-West-Reality         | |
|  | ○ 🇩🇪 Frankfurt-WG         | [WG]     | 165ms TCP   |  Mode              TUN                     | |
|  |                           |          | [TCP][URL]✎🗑|  Active sockets    42                      | |
|  |                                                    |  Core              v1.13.14                | |
|  |                                                    |  Admin access      Elevated                | |
|  +----------------------------------------------------+--------------------------------------------+ |
+------------------------------------------------------------------------------------------------------+
```

---

## 3. View 2: Profiles (`ConfigView.tsx`) & Subscription Diff Modal

### 3.1 Main Profiles Screen
```text
+------------------------------------------------------------------------------------------------------+
|  Profiles                                      [🔍 Search profiles...] [+ Empty Profile] [+ Add Server]
|  Manage subscriptions, configuration files and servers                                               |
+------------------------------------------------------------------------------------------------------+
|  SAVED PROFILES (2)                                  |  IMPORT PROFILE (Sticky Right Panel)          |
|  +------------------------------------------------+  |  +------------------------------------------+ |
|  | [🌐] Premium Sub        [SUBSCRIPTION] [ACTIVE]|  |  | [📥] Import Profile                      | |
|  |      12 servers · 2h ago · Active: US-West     |  |  |------------------------------------------| |
|  |  +------------------------------------------+  |  |  | Profile Name                             | |
|  |  | [🔄 Auto-update: Every 6 hours        v] |  |  |  | [ My Subscription                      ] | |
|  |  +------------------------------------------+  |  |  |                                          | |
|  |  [✓ Active]              [📊 Diff] [🔄] [🗑]   |  |  | Subscription URL or Raw Configuration    | |
|  +------------------------------------------------+  |  |                               [📋 Paste] | |
|                                                      |  | +--------------------------------------+ | |
|  +------------------------------------------------+  |  | | vless://uuid@1.2.3.4:443?security=   | | |
|  | [🖧] Self-Hosted Nodes                 [MANUAL] |  |  | | reality&sni=cloudflare.com#US-Node   | | |
|  |      3 servers · 3d ago · Active: Home-WG      |  |  | | trojan://pass@5.6.7.8:443#SG-Node    | | |
|  |                                                |  |  | +--------------------------------------+ | |
|  |  [▶ Use Profile]          [👁 Browse]   [🗑]   |  |  |                                          | |
|  +------------------------------------------------+  |  | +--------------------------------------+ | |
|                                                      |  | | [✓] 2 servers detected               | | |
|                                                      |  | | VLESS: 1 · TROJAN: 1                 | | |
|                                                      |  | | [US-Node] [SG-Node]                  | | |
|                                                      |  | +--------------------------------------+ | |
|                                                      |  |                                          | |
|                                                      |  | [      ⬇ Import from Text             ] | |
|                                                      |  | ------------ OR IMPORT FILE ------------ | |
|                                                      |  | [   📂 Choose File (.json, .yaml, .txt)] | |
|                                                      |  +------------------------------------------+ |
+------------------------------------------------------------------------------------------------------+
```

### 3.2 Subscription Update Diff Preview Modal (`diffPreview` Active)
```text
+------------------------------------------------------------------------------------------------------+
|  +------------------------------------------------------------------------------------------------+  |
|  |  [📊] Subscription Update Preview                                                          [✕] |  |
|  |       Premium Sub                                                                              |  |
|  |------------------------------------------------------------------------------------------------|  |
|  |  +------------------------+     +------------------------+     +----------------------------+  |  |
|  |  |         BEFORE         |     |         AFTER          |     |          RETAINED          |  |  |
|  |  |           12           |     |           14           |     |             11             |  |  |
|  |  +------------------------+     +------------------------+     +----------------------------+  |  |
|  |                                                                                                |  |
|  |  [!] Selected node "US-Old-1" was removed in this update. Will switch to "US-West-Reality".    |  |
|  |                                                                                                |  |
|  |  ADDED NODES (+3)                                                                              |  |
|  |  +------------------------------------------------------------------------------------------+  |  |
|  |  | [+ US-West-Reality-02]  [+ JP-Tokyo-Hy2-03]  [+ UK-London-VLESS]                         |  |  |
|  |  +------------------------------------------------------------------------------------------+  |  |
|  |                                                                                                |  |
|  |  REMOVED NODES (-1)                                                                            |  |
|  |  +------------------------------------------------------------------------------------------+  |  |
|  |  | [- US-Old-1 (delisted)]                                                                  |  |  |
|  |  +------------------------------------------------------------------------------------------+  |  |
|  |------------------------------------------------------------------------------------------------|  |
|  |                                                                [ Cancel ]  [ 🔄 Apply Update ] |  |
|  +------------------------------------------------------------------------------------------------+  |
+------------------------------------------------------------------------------------------------------+
```

---

## 4. Slide-Over Drawer: Server Node Editor (`NodeEditor.tsx`)

```text
                                            +----------------------------------------------------------+
                                            |  Edit Server Node                                    [✕] |
                                            |  Update protocol, transport and TLS settings             |
                                            |----------------------------------------------------------|
                                            |  +----------------------------------------------------+  |
                                            |  |  [  Basic  ]  |     Transport     |       TLS      |  |
                                            |  +----------------------------------------------------+  |
                                            |                                                          |
                                            |  Protocol                                                |
                                            |  [ VLESS                                              v] |
                                            |  +----------------------------------------------------+  |
                                            |  | Lightweight proxy protocol supporting XTLS Reality |  |
                                            |  +----------------------------------------------------+  |
                                            |                                                          |
                                            |  Node Name (Tag) *                                       |
                                            |  [ 🇺🇸 US-West-Reality                                  ] |
                                            |                                                          |
                                            |  Server Address (IP or Domain)   |  Server Port          |
                                            |  [ 198.51.100.42              ]  |  [ 443             ]  |
                                            |                                                          |
                                            |  UUID *                                                  |
                                            |  [ b8f2d1e0-4a12-4c99-8877-123456789abc               ]  |
                                            |                                                          |
                                            |  Flow Control                                            |
                                            |  [ xtls-rprx-vision                                   v] |
                                            |                                                          |
                                            |  (When switched to TLS tab:)                             |
                                            |  [●] Enable TLS                                          |
                                            |  SNI: [ www.cloudflare.com ]  ALPN: [ http/1.1 ]         |
                                            |  uTLS Fingerprint: [ Chrome                           v] |
                                            |  [●] Enable Reality (XTLS)                               |
                                            |  Reality Public Key *: [ xYz123...                    ]  |
                                            |  Reality Short ID:     [ a1b2c3d4                     ]  |
                                            |----------------------------------------------------------|
                                            |  [ Cancel ]                         [   Save Changes   ] |
                                            +----------------------------------------------------------+
```

---

## 5. View 3: Routing (`RoutingView.tsx`)

```text
+------------------------------------------------------------------------------------------------------+
|  Routing                             [● Connected — edits restart tunnel] [+ Add Rule] [+ Add Rule Set]
|  Decide which domains, IPs and apps use the proxy or connect directly                                |
+------------------------------------------------------------------------------------------------------+
|  ROUTING PRIMARY COLUMN (.routing-primary)          |  ROUTING ASIDE (.routing-aside)                |
|  +-----------------------------------------------+  |  +-------------------------------------------+ |
|  | (Inline Editor when +Add Rule or Edit open)   |  |  | Global Routing Policy  [Requires restart] | |
|  | Match Type          | Target Action           |  |  |-------------------------------------------| |
|  | [ Process Name   v] | [ Direct             v] |  |  | Default Route (Final Outbound)            | |
|  | Pick running process: [ chrome.exe         v] |  |  | [ Proxy (Tunnel all unmatched traffic) v] | |
|  | Process name (.exe):  [ chrome.exe          ] |  |  |                                           | |
|  | Notes:                [ Bypass browser      ] |  |  | Bypass LAN & Private Networks        [●]  | |
|  | [ ] Invert Match        [Cancel] [Save Rule]  |  |  | Send local network traffic directly       | |
|  +-----------------------------------------------+  |  +-------------------------------------------+ |
|                                                     |                                                |
|  +-----------------------------------------------+  |  +-------------------------------------------+ |
|  | Routing Rules                             [4] |  |  | Rule Sets                             [1] | |
|  |-----------------------------------------------|  |  |-------------------------------------------| |
|  | #  | On  | Type         | Value     | Route   |  |  | geosite-ads                               | |
|  |----+-----+--------------+-----------+---------|  |  | remote · binary                      🔄 🗑| |
|  | #1 | [●] | [GEOIP]      | private   | [DIRECT]|✎🗑|  | https://.../geosite-category-ads-all.srs  | |
|  |    |     |              | Bypass LAN|         |  |  | Updated: 11/03/2026, 09:00 AM             | |
|  |----+-----+--------------+-----------+---------|  |  +-------------------------------------------+ |
|  | #2 | [●] | [GEOSITE]    | private   | [DIRECT]|✎🗑|                                                |
|  |----+-----+--------------+-----------+---------|  |                                                |
|  | #3 | [●] | [PROCESS]    | steam.exe | [DIRECT]|✎🗑|                                                |
|  |----+-----+--------------+-----------+---------|  |                                                |
|  | #4 | [●] | [PROTOCOL]   | dns       | [DNS]   |✎🗑|                                                |
|  +-----------------------------------------------+  |                                                |
+------------------------------------------------------------------------------------------------------+
```

---

## 6. View 4: Connections (`ConnectionsView.tsx`)

```text
+------------------------------------------------------------------------------------------------------+
|  Connections                                                                       [⚠ Close All]     |
|  See active destinations, routing decisions and transferred data                                     |
+------------------------------------------------------------------------------------------------------+
|  [🔍 Search connections: Host, IP or rule... ]  [ All Tunnels | Proxy Tunnels | Direct ]  42 / 42    |
+------------------------------------------------------------------------------------------------------+
|  Destination ⇅         | Network / Matched Rule | Live speed ⇅ | Total data ⇅ | Uptime ⇅ |   Kill    |
|------------------------+------------------------+--------------+--------------+----------+-----------|
|  api.openai.com:443    | [PROXY › US-West]      | ↓ 1.2 MB/s   | ↓ 18.4 MB    | 4m 12s   |   [✕]     |
|  [TCP] type: tun-in    | rule: final            | ↑ 42.0 KB/s  | ↑ 640.0 KB   |          |           |
|------------------------+------------------------+--------------+--------------+----------+-----------|
|  github.com:443        | [PROXY › US-West]      | ↓ 340.5 KB/s | ↓ 4.1 MB     | 1m 05s   |   [✕]     |
|  [TCP] type: mixed-in  | rule: domain_suffix    | ↑ 12.1 KB/s  | ↑ 128.4 KB   |          |           |
|------------------------+------------------------+--------------+--------------+----------+-----------|
|  192.168.1.1:53        | [DIRECT]               | ↓ 0 B/s      | ↓ 1.2 KB     | 18s      |   [✕]     |
|  [UDP] type: tun-in    | rule: geoip=private    | ↑ 0 B/s      | ↑ 420 B      |          |           |
+------------------------------------------------------------------------------------------------------+
```

---

## 7. View 5: Logs (`LogsView.tsx`)

```text
+------------------------------------------------------------------------------------------------------+
|  Logs                                                [📋 Copy All] [🗑 Clear] | [●] Autoscroll        |
|  Follow proxy activity and system events as they happen                                              |
+------------------------------------------------------------------------------------------------------+
|  [🔍 Search logs: Find a message... ]  [ All | Info | Warn | Errors | System ]   Showing 148 of 148  |
+------------------------------------------------------------------------------------------------------+
|  09:04:11  SYS   [System] Booting X-Link Core using "Premium Sub" [US-West-Reality]...               |
|  09:04:12  INFO  [TUN] Wintun interface initialized (stack=gvisor, mtu=1500, address=172.19.0.1/30)  |
|  09:04:12  INFO  [DNS] FakeIP pool ready on 198.18.0.0/15 (bypass=geosite:private)                   |
|  09:04:13  SYS   [System] sing-box established on port Mixed:7892.                                   |
|  09:05:01  WARN  [WARN] connection to 198.51.100.99:443 timed out; retrying via fallback             |
|  09:05:30  SYS   [System] Direct TCP test complete: 12/12 nodes succeeded.                           |
|                                                                                                      |
|                                  (Virtualized via react-window List)                                 |
+------------------------------------------------------------------------------------------------------+
```

---

## 8. View 6: Settings (`SettingsView.tsx` — All 6 Sections)

```text
+------------------------------------------------------------------------------------------------------+
|  Settings                                                                                            |
|  Proxy mode, TUN, sniffing, DNS and API options                                                      |
+--------------------+---------------------------------------------------------------------------------+
|  SETTINGS NAV      |  SECTION CONTENT (Switches based on active left nav item)                       |
|  +--------------+  |                                                                                 |
|  | [●] General  |  |  === [1. GENERAL TAB] ===                                                       |
|  | [ ] TUN Mode |  |  +-----------------------------------+----------------------------------------+ |
|  | [ ] Sniffing |  |  | Proxy Mode         [Next connect] | Inbound Ports       [Requires restart] | |
|  | [ ] DNS      |  |  | TUN vs System Proxy          [●]  | Separate HTTP & SOCKS5 Ports      [ ]  | |
|  | [ ] API      |  |  |-----------------------------------| HTTP Port [7890] ● Free                | |
|  | [ ] Diag...  |  |  | App Preferences            [Live] | SOCKS Port[7891] ● Free                | |
|  +--------------+  |  | Close to Tray                [●]  | Mixed Port[7892] ● X-Link              | |
|                    |  | Launch on System Login       [ ]  +----------------------------------------+ |
|                    |  | LAN Hotspot Sharing          [ ]  | Appearance                      [Live] | |
|                    |  | Node Geolocation Lookup      [ ]  | [ ☀ Light  |  🌙 Dark  |  🖥 System ]   | |
|                    |  |-----------------------------------+----------------------------------------+ |
|                    |  | Connection Optimization           | [Logo] X-Link Empowered by sing-box    | |
|                    |  | Enable Multiplexing (Mux)    [ ]  |                                [v0.6.0]| |
|                    |  +-----------------------------------+----------------------------------------+ |
|                    |                                                                                 |
|                    |  === [2. TUN MODE TAB] ===                                                      |
|                    |  +----------------------------------------------------------------------------+ |
|                    |  | TUN Interface Configuration                             [Requires restart] | |
|                    |  | Stack: [ gVisor (Full userspace) v]   TUN MTU: [ 1500 ]                    | |
|                    |  | [●] Auto Route   [ ] Auto Redirect   [●] Strict Route   [ ] Endpoint NAT   | |
|                    |  +----------------------------------------------------------------------------+ |
|                    |                                                                                 |
|                    |  === [3. SNIFFING TAB] ===                                                      |
|                    |  +----------------------------------------------------------------------------+ |
|                    |  | [●] Enable Traffic Sniffing                                                | |
|                    |  | [●] HTTP Sniffing  [●] TLS/HTTPS  [●] QUIC/HTTP3  [ ] Override Destination | |
|                    |  +----------------------------------------------------------------------------+ |
|                    |                                                                                 |
|                    |  === [4. DNS TAB] ===                                                           |
|                    |  +----------------------------------------------------------------------------+ |
|                    |  | Primary DNS: [ https://1.1.1.1/dns-query ] [Cloudflare DoH][Google DoH]... | |
|                    |  | Fallback:    [ https://8.8.8.8/dns-query ] | Direct: [ Auto (System DNS) ] | |
|                    |  | Strategy:    [ Prefer IPv4 v]  [●] DNS Caching  [●] DNS Leak Protection    | |
|                    |  | Mode:        [ FakeIP (recommended) v] Range: [198.18.0.0/15] [geosite:..]| |
|                    |  +----------------------------------------------------------------------------+ |
|                    |                                                                                 |
|                    |  === [5. API TAB] ===                                                           |
|                    |  +----------------------------------------------------------------------------+ |
|                    |  | [●] Enable External API   Port: [ 9090 ]  Secret: [ 8f9a2b... ] [🔄]       | |
|                    |  | [ ] Allow CORS   [✓ API active at http://127.0.0.1:9090 — Yacd] [📋 Copy]  | |
|                    |  +----------------------------------------------------------------------------+ |
|                    |                                                                                 |
|                    |  === [6. DIAGNOSTICS TAB] ===                                                   |
|                    |  +----------------------------------------------------------------------------+ |
|                    |  | System Health & Bundle                                      [Privacy Safe] | |
|                    |  | [Admin: Yes] [Core: Running] [Mode: TUN] [Conflicts: None]                 | |
|                    |  | [📋 Generate Diagnostics] [Copy JSON] [⬇ Download Report]  [Show Preview]  | |
|                    |  |----------------------------------------------------------------------------| |
|                    |  | Configuration Backup & Restore                          [Contains secrets] | |
|                    |  | [⬇ Export backup]  [📂 Inspect backup file]                                | |
|                    |  |----------------------------------------------------------------------------| |
|                    |  | Routing Rule & DNS Simulation                                   [Simulate] | |
|                    |  | Destination: [ google.com                                 ] [▶ Simulate]   | |
|                    |  | Result: google.com [DOMAIN] -> [PROXY] (Final Outbound · FakeIP 198.18/15) | |
|                    |  +----------------------------------------------------------------------------+ |
+--------------------+---------------------------------------------------------------------------------+
```
