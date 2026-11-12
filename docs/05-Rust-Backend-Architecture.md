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
    State["state.rs\n(ProxyState, SidecarChild, Ring Buffers, Persistent Clash Client)"]
    Constants["constants.rs\n(Ports, Filenames, Sidecar Version Gate 1.13.x)"]
    Tray["tray.rs\n(Dynamic System Tray Menu & Event Handlers)"]
    OS["os/mod.rs\n(PlatformNetworkManager: windows.rs, darwin.rs, linux.rs)"]
    Supervisors["bin/\n(xlink-darwin-supervisor.rs, xlink-linux-supervisor.rs)"]

    subgraph ConfigEngine["config/ (Configuration Compilation Pipeline)"]
        Adapters["adapters/mod.rs\n(SingBox JSON, Clash YAML, Base64, Share Links)"]
        Rules["rules.rs\n(User Rules, Outbound Groups, Rule-Sets, LAN Bypass)"]
        Generator["generator.rs\n(Inbounds, Outbounds, Outbound Groups, FakeIP, Debug Log)"]
        Validator["validator.rs\n(Kahn's DAG Cycle Validator & sing-box check)"]
        Sub["subscription.rs\n(HTTP Stream Fetch <=10MB + UA Fallback)"]
    end

    subgraph Commands["commands/ (61 Tauri IPC Handlers)"]
        CProxy["proxy.rs\n(toggle_proxy, start/stop/reload_tunnel, select_active_node, Detour Validation, Ring Buffer Ingestion, Clash API)"]
        CConfig["config.rs\n(Import, Profiles, Diff Preview, Preview Singbox Config, Bulk Apply Bug Host, Routing Rules)"]
        CSystem["system.rs\n(Version Gate, Port Status, check_tun_support_status, Autostart)"]
        CLatency["latency.rs\n(TCP Ping + Clash URL Delay Test)"]
        CGeo["geo.rs\n(Consent-Gated Public IP Geo Lookup)"]
        CDiag["diagnostics.rs\n(Sanitized Bundle with Ring Buffers + Routing Simulator)"]
        CBackup["backup.rs\n(Encrypted v2 Backup AES-256-GCM + Argon2id Validation/Restore)"]
        CSched["subscription_schedule.rs\n(60s Background Auto-Update Task)"]
        CLogs["logs.rs\n(get_buffered_logs, get_logs, clear_logs)"]
    end

    Main --> State & Tray & OS & Commands
    OS --> Supervisors
    Commands --> ConfigEngine & State & OS & Constants
```

---

## 2. Core State & Constants (`state.rs` & `constants.rs`)

### 2.1 `ProxyState` (`src-tauri/src/state.rs`)
Managed as a Tauri singleton via `app.manage(ProxyState::new())`. Wrapped in a thread-safe container for access across IPC commands, background stdout line-readers, and scheduled tickers:

| Field | Type | Purpose |
| :--- | :--- | :--- |
| `child_process` | `Mutex<Option<SidecarChild>>` | Handle to the owned sidecar process, wrapping either `tauri_plugin_shell::process::CommandChild` or `tokio::process::Child` with unified `pid()` and value-move `kill()` methods. |
| `status` | `Mutex<ProxyStatus>` | Enum: `Disconnected`, `Connecting`, `Connected` (serializes to lowercase strings). |
| `effective_mode` | `Mutex<Option<String>>` | Actual mode of the running session (`"tun"` or `"system"`). |
| `dns_buffer` | `Mutex<VecDeque<DnsLogEntry>>` | Fixed-capacity ring buffer holding up to **500** recent DNS query exchanges (`[dns] exchanged ...`) parsed from stdout. |
| `trace_buffer` | `Mutex<VecDeque<TraceEntry>>` | Fixed-capacity ring buffer holding up to **500** recent routing match decisions (`[router] match ...`) parsed from stdout. |
| `clash_client` | `reqwest::Client` | Persistent singleton HTTP client configured with connection pooling and an **800ms** read timeout for all Clash REST API queries. |
| `prev_connection_bytes`| `Mutex<HashMap<String, ConnectionByteSnapshot>>` | Instantaneous per-socket upload/download delta tracker pruned each cycle to prevent memory leaks from closed connections. |
| `log_buffer` | `Arc<Mutex<VecDeque<String>>>` | Ring buffer holding up to **2,000** lines (`MAX_LOG_LINES`) of raw stdout/stderr output. |
| `cached_version` | `Mutex<Option<String>>` | Memoized output of `sing-box version`. |
| `subscription_preview` | `Mutex<Option<PreviewedSubscription>>` | Caches `{ token, profile_id, raw_content, created_at }` for **300 seconds** (`PREVIEW_TTL`) so `apply_previewed_subscription_refresh` commits the exact previewed payload. |
| `system` | `Mutex<sysinfo::System>` | Refreshes process-specific CPU and memory usage for sidecar PID telemetry. |

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
`generate_singbox_config()` transforms the parsed profile outbounds + `settings.json` + `routing.json` + Outbound Groups + host DNS server IPs into the final runtime configuration:

1. **Outbound Sanitization, Detours & Group Compilation**:
   - Strips internal/meta outbounds (`direct`, `block`, `dns-out`).
   - Normalizes fields for `sing-box 1.13+` (`wireguard` peer arrays, `ssh` `user` field, `shadowsocks` cipher validation).
   - **Outbound Group Compilation**:
     - `urltest`: Emitted as `type: "urltest"`, target test URL, health-check interval (floor 300s, default 600s), and tolerance (50ms).
     - `fallback`: Emitted as `type: "urltest"` with hysteresis tolerance.
     - `selector`: Emitted as native `type: "selector"`.
     - **Empty Group Fallback**: If all member nodes in an Outbound Group are pruned or invalid, the compiler automatically populates the group with dialable nodes from the active profile (capped at the first 10 nodes for automated pools). If no dialable nodes exist, the group compiles as a `selector` pointing to `["direct"]`, NEVER `"proxy"`, strictly preventing mutual circular selector recursion.
   - **Master Root Selector**: Wraps all nodes in a master `"selector"` outbound tagged `"proxy"` (`interrupt_exist_connections: true`), prepending all valid `OutboundGroup` tags before individual dialable node tags (`[group_tags..., node_tags...]`), enabling Clash API selector switching (`PUT /proxies/proxy`).
   - **Critical ALPN Fix**: If `type` is `vless` or `vmess` and transport is `ws` or `httpupgrade` with TLS enabled, forces `tls["alpn"] = ["http/1.1"]` so CDNs (Cloudflare/Nginx) do not negotiate HTTP/2 and reject the WebSocket `Upgrade` handshake.
   - **Multiplexing (`smux`)**: If `settings.multiplex == true`, injects `multiplex: { enabled: true, protocol: "smux", max_connections: 4, min_streams: 4, max_streams: 0 }` on supported TCP-multiplexable protocols (`vless`, `vmess`, `trojan`, `shadowsocks`), skipping QUIC-native protocols (`hysteria2`, `tuic`) and `xtls-rprx-*` flows.

2. **Detour Graph & DAG Cycle Validator (`src-tauri/src/config/validator.rs`)**:
   - Enforces Kahn's Directed Acyclic Graph (DAG) cycle detection across 2-hop detour chains (Relay $\rightarrow$ Exit):
     `validate_detour_graph(nodes: &[SingBoxOutbound], groups: &[OutboundGroup]) -> Result<(), ConfigValidationError>`
   - Strips dangling detour references before topological sort. Enforces protocol constraints (blocking UDP-over-TCP HTTP relays) and reserved tag namespace protection (`proxy`, `direct`, `block`, `dns`).
   - Returns structured `ConfigValidationError` containing `code: "CYCLIC_DETOUR"`, `offending_tags`, and `cycle_path`.

3. **Inbound Generation (`build_inbounds`)**:
   - **Listener Binding**: Binds to `127.0.0.1` by default, or `0.0.0.0` when `wifiSharing: true`.
   - **Ports**: Creates either a single `"mixed"` inbound (`mixedPort: 7892`) or separate `"http"` (`7890`) + `"socks"` (`7891`) inbounds when `useSeparatePorts: true`.
   - **TUN Inbound (`proxyMode == "tun"`)**:
     - Interface address: `["172.19.0.1/30"]`.
     - Interface name: Dynamically allocated kernel interface on macOS (omitting hardcoded `"interface_name"` to allow OS `utun` assignment); `wintun` on Windows.
     - Stack: `gvisor` (default), `mixed`, or `system`; MTU: `1500` (clamped `576..=9000`).
     - **`route_exclude_address` (Anti-Routing-Loop Invariant)**:
       - Excludes RFC-1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), loopback (`127.0.0.0/8`), link-local (`169.254.0.0/16`, `fe80::/10`), CGNAT (`100.64.0.0/10`), multicast (`224.0.0.0/4`), and bootstrap DNS (`1.1.1.1/32`, `8.8.8.8/32`).
       - Dynamically discovers host DNS server IPs via `PlatformNetworkManager::get_system_dns_servers()` and appends `{dns_ip}/32`.
       - Pre-resolves all outbound proxy server hostnames/IPs (`resolve_outbound_ips`) and appends `{server_ip}/32` or `/128` so encrypted proxy traffic never re-enters the TUN adapter.

4. **Structured Logging & Ring Buffer Telemetry**:
   - Configures `"log": { "level": "debug", "timestamp": true }` so that routing decisions (`[router] match ...`) and DNS exchanges (`[dns] exchanged ...`) are emitted to standard output.
   - Dedicated asynchronous stdout readers in `proxy.rs` parse and stream events into 500-record native Rust ring buffers (`dns_buffer` and `trace_buffer`) exposed via `get_diagnostic_traces`.

---

## 4. Cross-Platform OS Control Plane (`src-tauri/src/os/`)

X-Link abstracts operating system network configuration behind the asynchronous `PlatformNetworkManager` trait (`src-tauri/src/os/mod.rs`), providing platform-native implementations:

```rust
#[async_trait]
pub trait PlatformNetworkManager: Send + Sync {
    async fn check_tun_support(&self) -> Result<TunSupportStatus, String>;
    async fn request_platform_elevation(&self, app: &tauri::AppHandle) -> Result<(), String>;
    async fn enable_system_proxy(&self, host: &str, port: u16) -> Result<(), String>;
    async fn disable_system_proxy(&self) -> Result<(), String>;
    async fn get_system_dns_servers(&self) -> Result<Vec<IpAddr>, String>;
    fn restore_system_proxy_sync(&self);
}
```

### 4.1 Windows (`src-tauri/src/os/windows.rs`)
- **System Proxy**: Manages `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings` (`ProxyEnable`, `ProxyServer`, `ProxyOverride`) with `WinINet` notifications and crash-recovery restoration (`proxy_state_backup.json`).
- **TUN Mode**: Copies `wintun.dll` driver next to the sidecar executable.
- **DNS Discovery**: Queries Windows IP Helper API `GetAdaptersAddresses` (`AF_INET`) to identify non-tunnel active adapter DNS servers.
- **Process Supervision**: Assigns the `sing-box` child process to a Windows Job Object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, guaranteeing immediate kernel cleanup upon parent termination.

### 4.2 macOS (`src-tauri/src/os/darwin.rs`)
- **Unprivileged Desktop GUI**: The WebKit UI runs strictly as an unprivileged user to preserve WebKit sandbox integrity.
- **Root Companion Supervisor (`xlink-darwin-supervisor`)**:
  - Deployed in `/Library/Application Support/X-Link/helpers/` (mode `0755`, owned by `root:wheel`), preventing LPE (CWE-732).
  - Staging Pipeline: Ensures `/private/etc/sudoers.d` exists (`0755`), writes hidden staging dotfile `/.xlink-singbox.tmp` (`0440`), validates via `visudo -c -f`, and atomically moves to `/private/etc/sudoers.d/xlink-singbox`.
  - Spawning: Invoked via `sudo -n` with piped standard descriptors and monitored via `kqueue` `EVFILT_PROC` / `NOTE_EXIT` and `stdin` EOF heartbeats.
- **Dynamic TUN Allocation**: Relies on dynamic macOS `utun` interface creation without hardcoded interface conflicts.
- **DNS Discovery**: Inspects `scutil --dns`.

### 4.3 Linux (`src-tauri/src/os/linux.rs`)
- **Unprivileged Companion Supervisor (`xlink-linux-supervisor`)**:
  - Helper binaries are copied to persistent user-writable storage (`~/.local/share/x-link/helpers/`) supporting extended attributes, ensuring AppImage/SquashFS deployments succeed.
  - Granted `cap_net_admin,cap_net_bind_service=+ep` via `pkexec setcap`.
  - Supervisor sets `libc::PR_SET_PDEATHSIG` to `SIGTERM`, intercepts `SIGINT`/`SIGTERM`, monitors `stdin` EOF, and escalates to `SIGKILL` after 3 seconds.
- **DNS Discovery**: Resolves host DNS via `systemd-resolved` DBus or `/etc/resolv.conf`.
- **System Proxy**: Manages desktop proxy settings via `gsettings` (GNOME) or environment configuration.
