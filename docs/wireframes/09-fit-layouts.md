---
title: "Wireframe 09 — Fit-to-Window Layouts at Exact Scale"
aliases: ["Fit Layouts", "Scaled Wireframes", "Window Fit Wireframes"]
tags:
  - wireframe
  - ui
  - layout
  - spec
created: 2026-11-06
updated: 2026-11-06
---

# Wireframe 09 — Fit-to-Window Layouts at Exact Scale

**Navigation**: [[00-Home-MOC]] | **Geometry rules**: [[wireframes/00-window-and-grid]] | **Tokens & components**: [[09-Design-System]] | **Spec**: [[specs/003-responsive-fit-layout/spec]]

> [!NOTE] How to read these
> Every diagram is drawn at **1 column = 10 px, 1 row = 20 px**. The outer frame is the webview client area: **100 × 34 chars = 1000 × 680 px** for the default window, **72 × 26 chars = 720 × 520 px** for the minimum window. Rail = 22 cols (220 px) or 8 cols (80 px). Anything labelled *internal scroll* is a `.scroll-region`; **nothing else scrolls**. `>` marks the active destination. Names, numbers and domains are illustrative.
> The generator is `tools/gen_wireframes.py` in this vault — edit it and re-run instead of hand-editing the art.

Content shown follows the **source-aligned** notes ([[wireframes/02-dashboard]] … [[wireframes/08-settings]]). Where older notes disagree (server table vs. server chips, Outbound Groups), the layout reserves the same slot so either variant fits the same budget.

---

## 1. Shell + Dashboard — Expanded, 1000 × 680

Hero strip 120 px · main grid 448 px · server list and chart card are the only regions that size to the window.

```text
+---------------------+----------------------------------------------------------------------------+
| [X] X-Link          | Dashboard                                          (o) Protected 01:24:10  |
| Network control     | Your connection at a glance                                                |
|                     |----------------------------------------------------------------------------|
|> [spd] Dashboard    | +- Connect --- TUN routing + +- Live telemetry -------------------- Live + |
|                     | | Connected                | |                                           | |
|  [fld] Profiles     | | via US-West-Reality      | | Download  Upload    Active    Core        | |
|                     | | [ Disconnect          ]  | | 14.2 MB/s 1.8 MB/s  42        38 MB       | |
|  [rte] Routing      | | Session active 01:24:10  | |                                           | |
|                     | +--------------------------+ +-------------------------------------------+ |
|  [hub] Connections  |                                                                            |
|                     | +- Servers (12) --------- [Ping all] + +- Live traffic ------------ Live + |
|  [trm] Logs         | | [ Search servers...          ]     | | Down 14.2 MB/s    Up 1.8 MB/s   | |
|                     | | Server            Proto   Ping     | |                                 | |
|  [set] Settings     | |                                    | | 20 MB/s |        .--.           | |
|                     | | * US-West-Reality VLESS   42ms     | |         |   .--'    '--.        | |
|                     | |                                    | |         | .'            '.      | |
|                     | |   SG-Direct-Hy2   HY2     58ms     | |  0 B/s  +-------------------*   | |
|                     | |                                    | |                                 | |
|                     | |   JP-Tokyo-Trojan TROJAN  91ms     | |                                 | |
|                     | |                                    | | chart fills card                | |
|                     | |   Frankfurt-WG    WG      143ms    | |                                 | |
|                     | |                                    | +---------------------------------+ |
|                     | |   UK-London-VMess VMESS   188ms    | +- Session -----------------------+ |
|                     | |                                    | | Profile  Premium                | |
|                     | |   CA-Toronto-SS   SS      Err      | | Mode     TUN                    | |
|                     | |                                    | | Sockets  42                     | |
|                     | |                                    | | Core     1.13.14                | |
|                     | |                                    | |                                 | |
|                     | |                                    | |                                 | |
|                     | |   ... internal scroll only ...     | |                                 | |
| (o) Connected       | | (list scrolls, page does not)      | |                                 | |
|     Proxy session   | +------------------------------------+ +---------------------------------+ |
+---------------------+----------------------------------------------------------------------------+
```

**Budget:** header 64 · padding 32 · hero 120 · gap 16 · grid 448 = 680. Server list shows ~10 rows then scrolls inside its card. Chart redraws through `ResizeObserver`.

## 2. Dashboard — Minimum window, 720 × 520 (Medium + dense)

Rail collapses to 80 px, subtitle row disappears, the *Session* card and the full chart move behind a **Details** tab inside the Servers card so the hero strip and server list stay on-screen.

```text
+-------+--------------------------------------------------------------+
| [X]   | Dashboard                                     (o) Protected  |
|       | At a glance                                                  |
|       |--------------------------------------------------------------|
| [*]   | +- Connect ------- TUN + +- Telemetry --------------- Live + |
|       | | Connected            | | Down 14.2 MB/s   Up 1.8 MB/s    | |
| [ ]   | | [ Disconnect   ]     | | Sockets 42       Core 38 MB     | |
|       | | 01:24:10             | | [sparkline.........]            | |
| [ ]   | +----------------------+ +---------------------------------+ |
|       |                                                              |
| [ ]   | +- Servers (12) ------------------------------- [Ping all] + |
|       | | [ Search...        ]                                     | |
| [ ]   | |                                                          | |
|       | | * US-West-Reality    VLESS   42ms                        | |
| [ ]   | |                                                          | |
|       | |   SG-Direct-Hy2      HY2     58ms                        | |
|       | |                                                          | |
|       | |   JP-Tokyo-Trojan    TROJAN  91ms                        | |
|       | |                                                          | |
|       | |                                                          | |
|       | |                                                          | |
|       | |                                                          | |
|       | |                                                          | |
|       | | -- internal scroll; chart moves to a tab --              | |
|  (o)  | +----------------------------------------------------------+ |
+-------+--------------------------------------------------------------+
```

## 3. Profiles — Expanded

Import card is collapsible (`md-filled-tonal-button` "Import profile" toggles it). When collapsed, *Saved profiles* takes the full left column height.

```text
+---------------------+----------------------------------------------------------------------------+
| [X] X-Link          | Profiles                                                   [+ Add server]  |
| Network control     | Subscriptions, files and manual servers                                    |
|                     |----------------------------------------------------------------------------|
|  [spd] Dashboard    | +- Import profile -------------+ +- Servers in Premium Sub -- [Ping all] + |
|                     | | [ Name (optional)         ]  | | [ Search...   ] Auto [6h v]           | |
|> [fld] Profiles     | | [ URL or raw config       ]  | |                                       | |
|                     | | [                         ]  | | Server           Proto  Ping  Act     | |
|  [rte] Routing      | | [                         ]  | |                                       | |
|                     | |                              | | US-West-Reality  VLESS  42ms  [e][x]  | |
|  [hub] Connections  | | Detected: 7 nodes            | |                                       | |
|                     | | VLESS 5 - TROJAN 2           | | SG-Direct-Hy2    HY2    58ms  [e][x]  | |
|  [trm] Logs         | |                              | |                                       | |
|                     | | [Import] [Choose file]       | | JP-Tokyo-Trojan  TROJAN 91ms  [e][x]  | |
|  [set] Settings     | |                              | |                                       | |
|                     | +------------------------------+ | Frankfurt-WG     WG     143ms [e][x]  | |
|                     | +- Saved profiles (3) ---------+ |                                       | |
|                     | | (*) Premium Sub      ACTIVE  | | UK-London-VMess  VMESS  188ms [e][x]  | |
|                     | |     12 nodes - 2h ago        | |                                       | |
|                     | |                              | |                                       | |
|                     | | ( ) Work VPN         FILE    | |                                       | |
|                     | |     4 nodes                  | |                                       | |
|                     | |                              | |                                       | |
|                     | | ( ) Manual           MANUAL  | |                                       | |
|                     | |     1 node                   | |                                       | |
|                     | |                              | |                                       | |
|                     | | -- list scrolls internally --| |                                       | |
|                     | |                              | |                                       | |
|                     | |                              | |                                       | |
|                     | |                              | | -- table scrolls internally --        | |
|                     | |                              | |                                       | |
| (o) Connected       | |                              | | [Diff] [Refresh] [Delete]             | |
|     Proxy session   | +------------------------------+ +---------------------------------------+ |
+---------------------+----------------------------------------------------------------------------+
```

## 4. Routing — Expanded

Global policy and rule sets sit in a fixed 160 px row; the rules table owns all remaining height.

```text
+---------------------+----------------------------------------------------------------------------+
| [X] X-Link          | Routing                                                      [+ Add rule]  |
| Network control     | Final outbound, rule sets and rules                                        |
|                     |----------------------------------------------------------------------------|
|  [spd] Dashboard    | +- Global policy ------------------+ +- Rule sets (2) ---------- [+ Add] + |
|                     | |                                  | |                                   | |
|  [fld] Profiles     | | Final outbound   [ Proxy v ]     | | geosite-cn  remote [Update][x]    | |
|                     | |                                  | |                                   | |
|> [rte] Routing      | | Bypass LAN / private    (o--)    | | geoip-cn    remote [Update][x]    | |
|                     | |                                  | |                                   | |
|  [hub] Connections  | | Applies on next connect          | |                                   | |
|                     | +----------------------------------+ +-----------------------------------+ |
|  [trm] Logs         |                                                                            |
|                     | +- Rules (priority order) -----------------------------------------------+ |
|  [set] Settings     | | On   #   Type            Value               Route    Act              | |
|                     | |                                                                        | |
|                     | | on   1   domain_suffix   example.com         proxy    [e][x]           | |
|                     | |                                                                        | |
|                     | | on   2   geosite         cn                  direct   [e][x]           | |
|                     | |                                                                        | |
|                     | | off  3   process_name    chrome.exe          direct   [e][x]           | |
|                     | |                                                                        | |
|                     | | on   4   ip_cidr         10.0.0.0/8          direct   [e][x]           | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | | -- rules table scrolls internally --                                   | |
| (o) Connected       | |                                                                        | |
|     Proxy session   | +------------------------------------------------------------------------+ |
+---------------------+----------------------------------------------------------------------------+
```

## 5. Connections — Expanded

Toolbar is one row (`40 px`); the virtualized list fills the rest. Under 840 px the *x / y active* counter hides first.

```text
+---------------------+----------------------------------------------------------------------------+
| [X] X-Link          | Connections                                                   [Close all]  |
| Network control     | Live sockets through the tunnel                                            |
|                     |----------------------------------------------------------------------------|
|  [spd] Dashboard    | [ Search host, IP or rule... ]  (All)(Proxy)(Direct)       42 / 42 active  |
|                     |                                                                            |
|  [fld] Profiles     | +- Active sockets -------------------------------------------------------+ |
|                     | | Destination         Net Rule/Chain      Speed     Total   Time  Kill   | |
|  [rte] Routing      | |                                                                        | |
|                     | | api.github.com:443  TCP proxy > US-West 120 KB/s  4.2 MB  01:02 [x]    | |
|> [hub] Connections  | |                                                                        | |
|                     | | 1.1.1.1:53          UDP direct          0 B/s     2 KB    00:11 [x]    | |
|  [trm] Logs         | |                                                                        | |
|                     | | cdn.example.net:443 TCP proxy > US-West 2.1 MB/s  88 MB   12:40 [x]    | |
|  [set] Settings     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | | -- virtualized rows (36px), list scrolls internally --                 | |
| (o) Connected       | |                                                                        | |
|     Proxy session   | +------------------------------------------------------------------------+ |
+---------------------+----------------------------------------------------------------------------+
```

## 6. Logs — Expanded

```text
+---------------------+----------------------------------------------------------------------------+
| [X] X-Link          | Logs                                                       [Copy] [Clear]  |
| Network control     | Core and app events                                                        |
|                     |----------------------------------------------------------------------------|
|  [spd] Dashboard    | [ Find a message...  ] (All)(Info)(Warn)(Err)(Sys)  Auto (--o)             |
|                     |                                                                            |
|  [fld] Profiles     | +- Console ---------------------------------------- Showing 2000 of 2000 + |
|                     | | 12:00:01  INFO  [TUN] interface up                                     | |
|  [rte] Routing      | | 12:00:01  INFO  [DNS] fakeip pool ready                                | |
|                     | | 12:00:04  WARN  [WARN] node slow: SG-Direct-Hy2                        | |
|  [hub] Connections  | | 12:00:09  ERR   [ERR] dial tcp: i/o timeout                            | |
|                     | | 12:00:10  SYS   [Self-Heal] stage 2 uTLS firefox                       | |
|> [trm] Logs         | |                                                                        | |
|                     | |                                                                        | |
|  [set] Settings     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | |                                                                        | |
|                     | | -- 28px rows, virtualized, internal scroll --                          | |
| (o) Connected       | |                                                                        | |
|     Proxy session   | +------------------------------------------------------------------------+ |
+---------------------+----------------------------------------------------------------------------+
```

## 7. Settings — Expanded

Secondary nav is a fixed `180 px` column; only the content pane scrolls. Under Medium the secondary nav becomes a horizontal `md-tabs` strip above the pane.

```text
+---------------------+----------------------------------------------------------------------------+
| [X] X-Link          | Settings                                           (o) Protected 01:24:10  |
| Network control     | Mode, network and diagnostics                                              |
|                     |----------------------------------------------------------------------------|
|  [spd] Dashboard    | +- Sections -----+ +- Proxy mode -------------------------- Next connect + |
|                     | |                | |                                                     | |
|  [fld] Profiles     | | [*] General    | | (*) TUN        ( ) System proxy                     | |
|                     | |                | |                                                     | |
|  [rte] Routing      | | [ ] TUN Mode   | | TUN needs elevated helper on this OS                | |
|                     | |                | |                                                     | |
|  [hub] Connections  | | [ ] Sniffing   | | [Run as administrator for TUN]                      | |
|                     | |                | |                                                     | |
|  [trm] Logs         | | [ ] DNS        | |                                                     | |
|                     | |                | +-----------------------------------------------------+ |
|> [set] Settings     | | [ ] API        | +- App preferences ----------------------------- Live + |
|                     | |                | |                                                     | |
|                     | | [ ] Diagnostics| | Close to tray              (--o)                    | |
|                     | |                | |                                                     | |
|                     | |                | | Launch on login            (o--)                    | |
|                     | |                | |                                                     | |
|                     | |                | |                                                     | |
|                     | |                | +-----------------------------------------------------+ |
|                     | |                | +- Appearance ---------------------------------- Live + |
|                     | |                | |                                                     | |
|                     | |                | | (Light) (Dark) (System)                             | |
|                     | |                | |                                                     | |
|                     | |                | |                                                     | |
|                     | |                | |                                                     | |
|                     | |                | |                                                     | |
|                     | |                | | -- content pane scrolls internally --               | |
|                     | |                | |                                                     | |
| (o) Connected       | |                | |                                                     | |
|     Proxy session   | +----------------+ +-----------------------------------------------------+ |
+---------------------+----------------------------------------------------------------------------+
```

## 8. Node Editor side sheet — Expanded

Side sheet is `500 px` wide (`min(500px, 100%)`), header and footer pinned, body scrolls. Under Medium the sheet stays `500 px`; under Compact it is full width.

```text
+---------------------+----------------------------------------------------------------------------+
| [X] X-Link          | Dashboard                +- Edit server ----------------------------- [x] +|
| Network control     | Your connection at a glan|                                                ||
|                     |--------------------------| (Basic)(Transport)(TLS)                        ||
|> [spd] Dashboard    |                          |                                                ||
|                     |                          | Protocol          [ VLESS v ]                  ||
|  [fld] Profiles     |                          |                                                ||
|                     |                          | Name (tag)        [ US-West-Reality ]          ||
|  [rte] Routing      |                          |                                                ||
|                     |                          | Server            [ 198.51.100.42 ]            ||
|  [hub] Connections  |                          |                                                ||
|                     |                          | Port              [ 443 ]                      ||
|  [trm] Logs         |            scrim 52%     |                                                ||
|                     |                          | UUID              [ 7f1c...e2 ]                ||
|  [set] Settings     |                          |                                                ||
|                     |                          | Flow              [ xtls-rprx-vision v ]       ||
|                     |                          |                                                ||
|                     |                          |                                                ||
|                     |                          |                                                ||
|                     |                          |                                                ||
|                     |                          |                                                ||
|                     |                          |                                                ||
|                     |                          |                                                ||
|                     |                          |                                                ||
|                     |                          |                                                ||
|                     |                          |                                                ||
|                     |                          | -- body scrolls, footer pinned --              ||
|                     |                          |                                                ||
|                     |                          |                                                ||
|                     |                          |------------------------------------------------||
|                     |                          | [Cancel]                       [Save]          ||
| (o) Connected       |                          |                                                ||
|     Proxy session   |                          +------------------------------------------------+|
+---------------------+----------------------------------------------------------------------------+
```

## 9. Compact (browser dev preview only)

Unreachable in the Tauri window (minimum width 720) but kept working so `npm run dev` in a narrow browser tab and any future mobile target behave.

```text
+------------------------------------------------------+
| [=] Dashboard                 (o) Protected          |
|                                                      |
|------------------------------------------------------|
|                                                      |
| Single column, cards stack,                          |
| page body never scrolls sideways.                    |
|                                                      |
| +- Modal drawer (260px) -----+                       |
| |                            |                       |
| | [X] X-Link                 |                       |
| |                            |                       |
| | [*] Dashboard              |   scrim               |
| | [ ] Profiles               |                       |
| | [ ] Routing                |                       |
| | [ ] Connections            |                       |
| | [ ] Logs                   |                       |
| | [ ] Settings               |                       |
| |                            |                       |
| +----------------------------+                       |
|                                                      |
+------------------------------------------------------+
```

## 10. Per-view fit contract

| View | Fixed-height regions | The single scroll region | Medium (720–839) change | Dense (`height ≤ 600`) change |
| :--- | :--- | :--- | :--- | :--- |
| Dashboard | hero strip | server list (virtualized if > 50) | 2-col grid → server list + tabbed *Chart / Session* | hero 120 → 100 px |
| Profiles | import card (collapsible) | saved-profiles list **and** servers table (two independent scrollers) | single column; import collapsed by default | subtitle hidden |
| Routing | policy + rule-set row | rules table | rule-sets card collapses to an `md-assist-chip` row | subtitle hidden |
| Connections | toolbar | socket list | counter hidden | subtitle hidden |
| Logs | toolbar | log console | level chips → `md-select` | subtitle hidden |
| Settings | – | content pane | secondary nav → `md-tabs` | card padding 16 → 12 |
| Node editor | header, tabs, footer | body | unchanged | unchanged |
