---
title: "05 — Rust Backend Architecture (src-tauri)"
aliases: ["Rust Backend", "Sing-Box Engine", "Backend Architecture"]
tags:
  - backend
  - rust
  - sing-box
  - tauri
created: 2026-11-03
updated: 2026-11-03
---

# 05 — Rust Backend Architecture (`src-tauri`)

**Navigation**: [[00-Home-MOC]] | **Prev**: [[04-UI-Wireframes]] | **Next**: [[06-IPC-Bridge-and-Data-Contracts]] | **Related**: [[01-Architecture-Overview]], [[07-AI-Agent-Playbook]]

---

## 1. Backend Module Architecture

```mermaid
flowchart TB
    Main["main.rs / lib.rs\n(Tauri Builder, Plugin Registration, Exit Hook)"]
    State["state.rs\n(ProxyState, ProxyStatus, PreviewedSubscription, LogBuffer)"]
    Constants["constants.rs\n(Ports, Filenames, Sidecar Version Gate 1.13.x)"]
    Tray["tray.rs\n(Dynamic System Tray Menu & Event Handlers)"]
    OS["os/mod.rs\n(WinINet Proxy, IpHlpApi DNS, UAC, Port Owner)"]

    subgraph ConfigEngine["config/ (Configuration Compilation Pipeline)"]
        Adapters["adapters/mod.rs\n(SingBox JSON, Clash YAML, Base64, Share Links)"]
        Rules["rules.rs\n(User Rules, Rule-Sets, LAN Bypass)"]
        Generator["generator.rs\n(Inbounds, Outbounds, FakeIP DNS, ALPN, Route)"]
        Validator["validator.rs\n(sing-box check -c temp_check_*.json)"]
        Sub["subscription.rs\n(HTTP Stream Fetch <=10MB + UA Fallback)"]
    end

    subgraph Commands["commands/ (38 Tauri IPC Handlers)"]
        CProxy["proxy.rs\n(toggle_proxy, 5-Stage Self-Healing, Hot Switch, Stats)"]
        CConfig["config.rs\n(Import, Profiles, Diff Preview, Transactional Commit)"]
        CSystem["system.rs\n(Version Gate, Port Status, Elevation, Autostart, Processes)"]
        CLatency["latency.rs\n(TCP Ping + Clash URL Delay Test)"]
        CGeo["geo.rs\n(Consent-Gated Public IP Geo Lookup)"]
        CDiag["diagnostics.rs\n(Sanitized Bundle + Routing Simulator)"]
        CBackup["backup.rs\n(Archive Export, Validation & Atomic Restore)"]
        CSched["subscription_schedule.rs\n(60s Background Auto-Update Task)"]
        CLogs["logs.rs\n(get_buffered_logs)"]
    end

    Main --> State & Tray & OS & Commands
    Commands --> ConfigEngine & State & OS & Constants
```

---

## 2. Core State & Constants (`state.rs` & `constants.rs`)

### 2.1 `ProxyState` (`src-tauri/src/state.rs`)
Managed as a Tauri singleton via `app.manage(ProxyState::new())`. Every field is wrapped in a `std::sync::Mutex` for thread-safe access across IPC commands, background log readers, and the subscription scheduler:

| Field | Type | Purpose |
| :--- | :--- | :--- |
| `child` | `Mutex<Option<CommandChild>>` | Handle to the **owned** `sing-box` sidecar process spawned by `tauri_plugin_shell`. |
| `status` | `Mutex<ProxyStatus>` | Enum: `Disconnected`, `Connecting`, `Connected` (serializes to lowercase strings). |
| `effective_mode` | `Mutex<Option<String>>` | Actual mode of the running session (`"tun"` or `"system"`). |
| `last_traffic` | `Mutex<Option<(u64, u64, Instant)>>` | Tuple `(upload_total, download_total, timestamp)` used to compute 1-second byte/sec speed deltas when Clash API reports cumulative totals. |
| `log_buffer` | `Arc<Mutex<VecDeque<String>>>` | Ring buffer holding up to **2,000** lines (`MAX_LOG_LINES`) of `sing-box` stdout/stderr output so logs emitted before WebView mount are never lost. |
| `cached_version` | `Mutex<Option<String>>` | Memoized output of `sing-box version`. |
| `subscription_Preview` | `Mutex<Option<PreviewedSubscription>>` | Caches `{ token, profile_id, raw_content, created_at }` for **300 seconds** (`PREVIEW_TTL`) so `apply_previewed_subscription_refresh` commits the exact previewed payload. |
| `system` | `Mutex<sysinfo::System>` | Refreshes process-specific CPU and memory usage for `sing-box` PID telemetry. |

### 2.2 `constants.rs` (`src-tauri/src/constants.rs`)
- **Files**: `SETTINGS_FILE = "settings.json"`, `PROFILES_FILE = "profiles.json"`, `ROUTING_FILE = "routing.json"`, `ACTIVE_CONFIG_FILE = "active.json"`, `CONFIGS_DIR = "configs"`, `PROFILES_DIR = "profiles"`, `RULE_SETS_DIR = "rule-sets"`.
- **Default Ports**: `DEFAULT_HTTP_PORT = 7890`, `DEFAULT_SOCKS_PORT = 7891`, `DEFAULT_MIXED_PORT = 7892`, `DEFAULT_API_PORT = 9090`.
- **Sidecar Version Gate**: `BUNDLED_TESTED_SINGBOX_VERSION = "1.13.14"`, `SUPPORTED_SINGBOX_VERSION_PREFIX = "1.13."`.

---

## 3. Configuration Engine (`src-tauri/src/config/`)

### 3.1 Multi-Format Adapter (`src-tauri/src/config/adapters/mod.rs`)
`parse_config_content(raw_content: &str) -> Result<Value, String>` automatically detects and converts any of 4 input formats into a canonical `sing-box` JSON object containing `"outbounds"`:
1. **Native `sing-box` JSON**: Starts with `{` and contains an `"outbounds"` array.
2. **Base64-Encoded Subscription**: Decodes standard/URL-safe Base64 (with automatic padding repair) and parses the resulting newline-separated share links.
3. **Raw Share Links (`parse_share_links`)**:
   - `vless://uuid@host:port?type=ws&security=reality&sni=...&pbk=...&sid=...&fp=chrome&flow=xtls-rprx-vision#Tag`
   - `vmess://base64(json)` (v2rayN JSON format: `add`, `port`, `id`, `aid`, `net`, `type`, `host`, `path`, `tls`, `sni`, `alpn`, `fp`, `ps`)
   - `trojan://password@host:port?security=tls&sni=...&type=grpc&serviceName=...#Tag`
   - `ss://base64(method:password)@host:port#Tag` or `ss://base64(method:password@host:port)#Tag`
   - `hysteria2://` or `hy2://password@host:port?sni=...&insecure=1&obfs=salamander&obfs-password=...#Tag`
   - `tuic://uuid:password@host:port?congestion_control=bbr&udp_relay_mode=native&sni=...&alpn=h3#Tag`
   - `wg://private_key@host:port?public_key=...&address=10.0.0.2/32&mtu=1420&reserved=0,0,0#Tag`
4. **Clash / Mihomo YAML (`parse_clash_yaml`)**:
   - Parses `proxies:` sequence using `serde_yaml`, converting `vmess`, `vless`, `trojan`, `ss`, `hysteria2`, `tuic`, `wireguard`, `socks5`, and `http` entries (including `ws-opts`, `grpc-opts`, `reality-opts`, `client-fingerprint`) into `sing-box` outbounds.
- **Unique Tag Deduplication (`ensure_unique_tags`)**: Appends ` (2)`, ` (3)` if duplicate outbound tags exist in a subscription.

---

### 3.2 `sing-box` Config Generator (`src-tauri/src/config/generator.rs`)
`generate_config()` transforms the parsed profile outbounds + `settings.json` + `routing.json` + `selected_outbound_tag` into the final runtime configuration:

1. **Outbound Sanitization & Filtering**:
   - Strips internal/meta outbounds (`direct`, `block`, `dns-out`, `selector`, `urltest`).
   - Normalizes fields for `sing-box 1.13+` (`wireguard` peer arrays, `ssh` `user` field, `shadowsocks` cipher validation).
   - **Critical ALPN Fix**: If `type` is `vless` or `vmess` and transport is `ws` or `httpupgrade` with TLS enabled, forces `tls["alpn"] = ["http/1.1"]` so CDNs (Cloudflare/Nginx) do not negotiate HTTP/2 and reject the WebSocket `Upgrade` handshake.
   - **Multiplexing (`smux`)**: If `settings.multiplex == true`, injects `multiplex: { enabled: true, protocol: "smux", max_connections: 4, min_streams: 4, max_streams: 0 }` on supported TCP-multiplexable protocols (`vless`, `vmess`, `trojan`, `shadowsocks`), skipping QUIC-native protocols (`hysteria2`, `tuic`) and `xtls-rprx-*` flows.
   - Wraps all proxy nodes in a master `"selector"` outbound tagged `"proxy"` (`interrupt_exist_connections: true`) with `default` set to the user's `selected_outbound_tag`, plus a `"direct"` outbound.

2. **Inbound Generation (`build_inbounds`)**:
   - **Listener Binding**: Binds to `127.0.0.1` by default, or `0.0.0.0` when `wifiSharing: true`.
   - **Ports**: Creates either a single `"mixed"` inbound (`mixedPort: 7892`) or separate `"http"` (`7890`) + `"socks"` (`7891`) inbounds when `useSeparatePorts: true`.
   - **TUN Inbound (`proxyMode == "tun"`)**:
     - Interface address: `["172.19.0.1/30"]`.
     - Stack: `gvisor` (default), `mixed`, or `system`; MTU: `1500` (clamped `576..=9000`).
     - **`route_exclude_address` (Anti-Routing-Loop Invariant)**:
       - Excludes RFC-1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), loopback (`127.0.0.0/8`), link-local (`169.254.0.0/16`, `fe80::/10`), CGNAT (`100.64.0.0/10`), multicast (`224.0.0.0/4`), and bootstrap DNS (`1.1.1.1/32`, `8.8.8.8/32`).
       - Dynamically queries the active Windows physical adapter's DNS server via `os::get_system_dns_server()` (`GetAdaptersAddresses`) and appends `{dns_ip}/32`.
       - Pre-resolves all outbound proxy server hostnames/IPs (`resolve_outbound_ips`) and appends `{server_ip}/32` or `/128` so encrypted proxy traffic never re-enters the TUN adapter.

3. **DNS Engine (`build_dns`)**:
   - Parses `primaryDns`, `fallbackDns`, and `directDns` into structured `sing-box 1.12+` typed server objects (`https`, `tls`, `quic`, `tcp`, `udp`, `local`) via `parse_dns_url()`.
   - When `dnsMode == "fakeip"`, adds a `"fakeip"` server (`inet4_range: "198.18.0.0/15"`, `inet6_range: "fc00::/18"`) and routes `A`/`AAAA` queries to `fakeip-dns` while bypassing `fakeipFilter` (`geosite:private`) to `local-dns`.

4. **Route Compilation (`src/config/rules.rs` + `generator.rs`)**:
   - Injects `sniff` inbound action (`http`, `tls`, `quic`) and `hijack-dns` protocol rule.
   - Compiles user `routing.json` rules (`domain`, `ip_cidr`, `process_name`, `rule_set`, etc.) and converts legacy `geoip`/`geosite` rules into remote SagerNet `.srs` `rule_set` references (`sing-box 1.12+` removed legacy GeoIP/Geosite databases).
   - Appends LAN bypass (`ip_is_private: true -> direct`) when `bypassLan: true`.
   - Sets `default_domain_resolver` to `local-dns` so outbound proxy domains resolve without circular dependencies.

5. **Clash API Controller (`experimental.clash_api`)**:
   - Enabled only when `apiEnabled: true` and `apiSecret` is non-empty. Binds to `127.0.0.1:{apiPort}` with Bearer secret authentication and optional CORS origins (`https://yacd.haishan.me`, `https://metacubex.github.io`, `http://localhost:*`).

---

## 4. Windows OS Integration (`src-tauri/src/os/mod.rs`)

1. **`ProxyStateManager` (Crash-Safe System Proxy Restoration)**:
   - Before modifying `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings`, snapshots the existing `ProxyEnable`, `ProxyServer`, and `ProxyOverride` values to `app_data_dir()/proxy_state_backup.json` with `managed_by_xlink: true`.
   - Calls Windows `WinINet` `InternetSetOptionW(INTERNET_OPTION_SETTINGS_CHANGED)` and `InternetSetOptionW(INTERNET_OPTION_REFRESH)` so browsers pick up changes immediately.
   - On disconnect, app exit (`RunEvent::Exit`), or next boot after a crash (`recover_if_needed()`), restores the user's exact pre-session registry state.
2. **Locale-Independent System DNS Discovery (`get_system_dns_server`)**:
   - Calls Windows IP Helper API `GetAdaptersAddresses` (`AF_INET`) to inspect active (`IfOperStatusUp`), non-loopback, non-tunnel (`IF_TYPE_TUNNEL`) network adapters, skipping virtual adapters (`wintun`, `sing-box`, `wireguard`, `tap`, `tun`) and returning the first valid non-loopback IPv4 DNS server via `select_best_dns_server()`.
3. **UAC Elevation & Run-As-Admin Persistence**:
   - `is_elevated()`: Inspects the current process access token (`OpenProcessToken` + `GetTokenInformation(TokenElevation)`).
   - `request_elevation` (`system.rs`): Launches a new elevated instance via PowerShell `Start-Process -Verb RunAs` and exits the non-elevated instance.
   - `set_runas_admin` (`system.rs`): Sets or removes `RUNASADMIN` in `HKCU\Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers`.
4. **Port Conflict & Ownership Inspection (`get_port_owner_pid`)**:
   - Runs `netstat -ano -p tcp` on Windows to determine whether a bound port is owned by X-Link's own `sing-box` child (`own`), another external application (`external`), or `free`.
