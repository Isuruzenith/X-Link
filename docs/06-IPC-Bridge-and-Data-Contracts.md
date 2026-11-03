---
title: "06 — IPC Bridge, Events & Disk Data Contracts"
aliases: ["IPC Bridge", "Tauri Commands", "Data Contracts", "JSON Schemas"]
tags:
  - ipc
  - contracts
  - backend
  - frontend
created: 2026-11-03
updated: 2026-11-03
---

# 06 — IPC Bridge, Events & Disk Data Contracts

**Navigation**: [[00-Home-MOC]] | **Prev**: [[05-Rust-Backend-Architecture]] | **Next**: [[07-AI-Agent-Playbook]] | **Related**: [[02-Frontend-UI-Architecture]], [[05-Rust-Backend-Architecture]]

---

## 1. Complete Tauri `invoke()` Command Reference (49 Commands)

All commands are registered in `src-tauri/src/lib.rs` via `tauri::generate_handler![...]`. Note that Tauri v2 automatically maps Rust `snake_case` parameter names to JavaScript `camelCase` keys in the `invoke('cmd', { camelCaseArg })` payload.

### 1.1 Proxy Lifecycle & Telemetry (`src-tauri/src/commands/proxy.rs`)

| Command Name | JS Arguments (`camelCase`) | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `get_proxy_status` | `{}` | `{ status: 'connected' \| 'connecting' \| 'disconnected', effectiveProxyMode: 'tun' \| 'system' \| null }` | Returns authoritative native proxy status and active mode. |
| `toggle_proxy` | `{ start: boolean, selectedOutboundTag: string \| null, modeOverride?: 'tun' \| 'system' \| null }` | `string` (`"started"` or `"stopped"`) | Starts (with 5-stage self-healing) or stops the owned `sing-box` child process and manages OS system proxy. |
| `reload_active_config` | `{ selectedOutboundTag: string \| null }` | `string` (`"reloaded"`) | Validates candidate config first; if connected, stops old child and restarts with rollback protection. |
| `switch_node_hot` | `{ tag: string }` | `string` (`"switched"`) | Hot-switches the active node via Clash API `PUT /proxies/proxy` if `apiEnabled` & connected; otherwise cold-restarts. |
| `get_traffic_stats` | `{}` | `TrafficStats` (`uploadSpeed`, `downloadSpeed`, `uploadTotal`, `downloadTotal`, `activeConnections`, `corePid`, `memoryBytes`, `cpuPercent`) | Queries Clash API `/connections` + `sysinfo` for live 1-second telemetry. |
| `get_active_connections` | `{}` | `ConnectionsPayload` (`connections: ActiveConnection[]`, `totalConnections`, `downloadTotal`, `uploadTotal`) | Proxies `GET http://127.0.0.1:{apiPort}/connections` with 800ms timeout and top-100 throughput windowing. |
| `close_connection` | `{ id: string }` | `()` | Proxies `DELETE http://127.0.0.1:{apiPort}/connections/{id}`. |
| `close_all_connections` | `{}` | `()` | Proxies `DELETE http://127.0.0.1:{apiPort}/connections`. |
| `get_proxy_group_status` | `{ group: string }` | `ProxyGroupInfo` (`group`, `winningTag`, `delay?`) | Queries Clash API `GET /proxies/{group}` to resolve current winning node and latency for automated Outbound Groups. |
| `get_diagnostic_traces` | `{}` | `DiagnosticTracesPayload` (`dnsQueries: DnsLogEntry[]`, `traces: TraceEntry[]`) | Retrieves live ring-buffered telemetry (500 records each) for DNS query exchanges and routing rule matches. |

---

### 1.2 Profile, Node & Subscription Management (`src-tauri/src/commands/config.rs`)

| Command Name | JS Arguments (`camelCase`) | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `import_config` | `{ filePath?: string, content?: string, name?: string, profileId?: string }` | `ImportResult` (`{ id, name, importedAt, nodeCount }`) | Fetches URL (if `http(s)://`) or parses raw/file config, validates via `sing-box check`, and writes `profiles/{id}.json` + `configs/active.json`. |
| `preview_import_config` | `{ content: string }` | `ImportPreview` (`{ nodeCount, nodeTypes: Record<string, usize>, sampleNodes: string[] }`) | Dry-run parser for live preview in `ConfigView` (rejects remote URLs to avoid side-effect network requests while typing). |
| `create_empty_profile` | `{ name: string }` | `ProfileMetadata` | Creates a manual profile with an empty `outbounds: []` array and sets it active in `profiles.json`. |
| `preview_subscription_refresh` | `{ profileId: string }` | `SubscriptionDiffPreview` | Downloads subscription, compares new vs existing node tags, caches payload in `ProxyState` with a 5-min `previewToken`, and returns diff summary. |
| `apply_previewed_subscription_refresh` | `{ profileId: string, previewToken: string }` | `ImportResult` | Commits the cached `PreviewedSubscription` payload transactionally without re-fetching from the network. |
| `refresh_subscription` | `{ profileId: string }` | `ImportResult` | Directly downloads, validates, and transactionally commits a subscription update. |
| `run_subscription_auto_updates` | `{}` | `usize` (count of updated profiles) | Checks all subscription profiles with `autoUpdateInterval > 0` and refreshes any that are due. |
| `switch_profile` | `{ profileId: string, selectedNodeTag?: string \| null }` | `string` (`"switched"`) | Copies `profiles/{profileId}.json` $\rightarrow$ `configs/active.json` and reloads the running tunnel with rollback protection. |
| `delete_profile_config` | `{ profileId: string }` | `string` (`"deleted"`) | Deletes `profiles/{profileId}.json` (refuses if `profileId` is still the active profile of a running session). |
| `get_profile_outbounds` | `{ profileId: string }` | `ProxyNode[]` | Reads user-selectable proxy outbounds from `profiles/{profileId}.json`. |
| `get_config_outbounds` | `{}` | `ProxyNode[]` | Reads user-selectable proxy outbounds from `configs/active.json`. |
| `get_active_outbound` | `{}` | `ProxyNode` | Returns the currently selected default outbound node in `configs/active.json`, verifying Outbound Groups before fallback. |
| `update_node` | `{ newOutbound: Value }` | `string` (`"updated"`) | Inserts or updates a node by `tag` in both `configs/active.json` and `profiles/{activeProfileId}.json`, validates via `sing-box check`, and reloads if connected. |
| `delete_node` | `{ tag: string }` | `string` (`"deleted"`) | Deletes a node by `tag` from the active profile and `active.json`, updates `nodeCount`/`selectedNodeTag`, and reloads if connected. |
| `update_rule_set` | `{ ruleSetId: string, url: string }` | `string` (saved file path) | Downloads a remote `.srs` or `.json` rule-set file into `app_data_dir()/rule-sets/`. |

---

### 1.3 System, OS & Privilege Commands (`src-tauri/src/commands/system.rs` & `logs.rs`)

| Command Name | JS Arguments (`camelCase`) | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `get_singbox_version` | `{}` | `string` | Runs `sing-box version`, caches the semver string (`"1.13.14"`), and returns it. |
| `get_app_version` | `{}` | `string` | Returns `env!("CARGO_PKG_VERSION")` (`"0.8.0"`). |
| `is_minimized_launch` | `{}` | `boolean` | Returns `true` if process was started with `--minimized`. |
| `show_window` | `{}` | `()` | Reveals, unminimizes, and focuses the `"main"` WebView window. |
| `check_port_conflict` | `{ ports: u16[] }` | `u16 \| null` | Returns the first port in `ports` bound by an external process (ignoring X-Link's own `sing-box` child). |
| `check_port_status` | `{ ports: u16[] }` | `Record<u16, 'free' \| 'own' \| 'external'>` | Classifies each port as `free`, bound by X-Link (`own`), or bound by another app (`external`). |
| `set_autostart` | `{ enabled: boolean }` | `()` | Configures desktop autostart across Windows (Registry), macOS (LaunchAgent), and Linux (XDG autostart). |
| `check_tun_support` | `{}` | `boolean` | Backward-compatible boolean check returning `true` only if `check_tun_support_status == Supported`. |
| `check_tun_support_status` | `{}` | `TunSupportStatus` (`'supported' \| 'requiresElevation' \| 'driverMissing' \| 'unsupported'`) | Structured readiness check verifying drivers (`wintun.dll`), helper binaries, and OS elevation state. |
| `request_elevation` | `{}` | `()` | Requests platform-specific elevation (`RunAs` on Windows, staged sudoers helper on macOS, `pkexec setcap` on Linux). |
| `set_runas_admin` | `{ enabled: boolean }` | `()` | Sets/removes `RUNASADMIN` compatibility flag in Windows Registry. |
| `get_running_processes` | `{}` | `string[]` | Returns sorted, deduplicated process names via `sysinfo` for the Routing rule picker. |
| `get_buffered_logs` | `{}` | `string[]` | Returns all lines currently stored in `ProxyState.log_buffer`. |

---

### 1.4 Latency, GeoIP, Diagnostics & Backup (`latency.rs`, `geo.rs`, `diagnostics.rs`, `backup.rs`)

| Command Name | JS Arguments (`camelCase`) | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `test_node_latency` | `{ server: string, port: u16 }` | `u64` (ms) | Measures direct TCP `TcpStream::connect` handshake latency (5s timeout). |
| `test_all_nodes` | `{ profileId?: string }` | `LatencyResult[]` | Runs concurrent direct TCP pings across all nodes in a profile (max **10** concurrent via `Semaphore`). |
| `test_proxy_url_latency` | `{ tag: string, url?: string, timeoutMs?: u64 }` | `u64` (ms) | Measures real end-to-end tunnel delay via Clash API `GET /proxies/{tag}/delay?url=https://cp.cloudflare.com/generate_204`. |
| `test_all_nodes_url_latency` | `{ profileId?: string, url?: string, timeoutMs?: u64 }` | `LatencyResult[]` | Runs concurrent Clash API `/delay` tests across all nodes (max **6** concurrent via `Semaphore`). |
| `test_active_proxy_url` | `{ url?: string, timeoutMs?: u64 }` | `u64` (ms) | Tests end-to-end delay of the master `"proxy"` selector via Clash API. |
| `lookup_node_geo` | `{ server: string }` | `NodeGeoInfo` | Consent-gated (`settings.geoLookupEnabled == true`) lookup of literal **public** IPs via `https://freeipapi.com/api/json/{ip}`. Never sends domain names or private IPs. |
| `export_system_diagnostics` | `{}` | `DiagnosticsReport` | Generates a sanitized JSON health bundle with `[REDACTED]` secrets, scrubbed IPs, and embedded DNS/trace buffers. |
| `simulate_routing` | `{ destination: string }` | `RoutingSimulationResult` | Offline evaluator explaining which rule (`ip_cidr`, `domain_suffix`, `geoip`, `geosite`, LAN bypass, or `finalOutbound`) and DNS strategy matches a target host/IP. |
| `export_config_backup` | `{ passphrase?: string \| null }` | `Value` (JSON envelope) | Exports settings, profiles, and routing into an encrypted v2 archive (AES-256-GCM + Argon2id) or unencrypted v1 JSON. |
| `validate_backup_archive` | `{ archive: Value, passphrase?: string \| null }` | `BackupValidationResult` | Validates archive envelope, enforcing cryptographic bounds (`1024 KiB <= memoryCost <= 65536 KiB`, `iterations <= 10`, `parallelism = 1`). Returns `requiresPassphrase: true` challenge on encrypted archives when no passphrase is provided. |
| `import_config_backup` | `{ archive: Value, options: RestoreOptions, passphrase?: string \| null }` | `RestoreResult` (`{ message: string }`) | Decrypts and restores selected sections while disconnected, preventing lock-ordering deadlocks and emitting `config-backup-restored`. |

---

## 2. Tauri Event Bus (`emit` $\rightarrow$ `listen`)

| Event Name | Emitter (Rust) | Payload Schema | Listener (`src/App.tsx`) |
| :--- | :--- | :--- | :--- |
| `sing-box-log` | `commands/proxy.rs` (stdout/stderr reader task) | `string` (raw log line) | Classifies log level via `classifySingBoxLog()` and appends to `useLogStore`. |
| `sing-box-terminated` | `commands/proxy.rs` (`CommandEvent::Terminated`) | `i32 \| null` (exit code) | Sets `connectionStore` to `disconnected`, logs exit code, and shows error toast if non-zero. |
| `proxy-status-changed` | `commands/proxy.rs` & `tray.rs` | `'connected' \| 'connecting' \| 'disconnected'` | Syncs `connectionStore` status and queries `get_proxy_status` for `effectiveProxyMode`. |
| `settings-changed` | `tray.rs` | `()` | Calls `storeHelper.reloadSettings()` + `useSettingsStore.getState().initSettings()`. |
| `node-switched` | `tray.rs` (Tray radio menu click) | `{ profileId: string, tag: string }` | Calls `useProfileStore.getState().syncNodeFromTray(profileId, tag)`. |
| `subscription-auto-updated` | `commands/subscription_schedule.rs` | `{ profileId: string, profileName: string, nodeCount: usize }` | Reloads profiles (`storeHelper.reloadProfiles()`), refreshes nodes, and shows info toast. |
| `subscription-update-error` | `commands/subscription_schedule.rs` | `{ profileId: string, profileName: string, error: string }` | Logs error and shows warning toast. |
| `config-backup-restored` | `commands/backup.rs` | `()` | Reloads all frontend stores (`storeHelper.reload*()`) and refreshes Zustand state after backup restoration. |

---

## 3. On-Disk Storage Schemas (`app_data_dir()`)

On Windows, `app.path().app_data_dir()` resolves to `C:\Users\<User>\AppData\Roaming\com.isuru.x-link\`.

```text
%APPDATA%\com.isuru.x-link\
├── settings.json              # Managed by plugin-store + read by Rust generator/commands
├── profiles.json              # Profile list metadata & activeProfileId
├── routing.json               # Custom routing rules & rule-set definitions
├── stats.json                 # Cumulative per-node upload/download/connection counters
├── proxy_state_backup.json    # Crash-recovery snapshot of Windows WinINet proxy settings
├── configs\
│   └── active.json            # Compiled & validated sing-box JSON passed to `sing-box run -c`
├── profiles\
│   └── <uuid>.json            # Raw parsed outbounds JSON for each profile
└── rule-sets\
    └── <tag>.srs              # Cached binary/source rule-sets downloaded by update_rule_set
```

### 3.1 `profiles.json` Schema
```json
{
  "activeProfileId": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "profiles": [
    {
      "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
      "name": "Premium Subscription",
      "type": "subscription",
      "subscriptionUrl": "https://example.com/sub?token=xyz",
      "importedAt": 1730000000000,
      "lastUpdated": 1730003600000,
      "nodeCount": 12,
      "selectedNodeTag": "🇺🇸 US-West-Reality",
      "autoUpdateInterval": 360
    }
  ]
}
```

### 3.2 `routing.json` Schema
```json
{
  "rules": [
    {
      "id": "r1",
      "type": "geoip",
      "value": "private",
      "outbound": "direct",
      "invert": false,
      "notes": "Bypass LAN/private IPs",
      "enabled": true
    }
  ],
  "ruleSets": [
    {
      "id": "rs1",
      "tag": "geosite-ads",
      "type": "remote",
      "format": "binary",
      "url": "https://raw.githubusercontent.com/SagerNet/sing-geosite/rule-set/geosite-category-ads-all.srs",
      "updateInterval": "1d",
      "lastUpdated": 1730000000000
    }
  ]
}
```
