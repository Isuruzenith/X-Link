---
title: "07 — AI Agent Playbook & Modification Guide"
aliases: ["AI Playbook", "Invariants", "Developer Guide"]
tags:
  - ai-context
  - playbook
  - architecture
  - invariants
created: 2026-11-03
updated: 2026-11-03
---

# 07 — AI Agent Playbook & Modification Guide

**Navigation**: [[00-Home-MOC]] | **Prev**: [[06-IPC-Bridge-and-Data-Contracts]] | **Related**: [[01-Architecture-Overview]], [[02-Frontend-UI-Architecture]], [[05-Rust-Backend-Architecture]]

---

## 1. The 7 Non-Negotiable Engineering Invariants

> [!DANGER] Read Before Editing Any Code
> Every AI agent working on X-Link **must** preserve these 7 invariants. Breaking any of them will cause routing loops, TLS handshake failures, state corruption, or security regressions.

### Invariant 1: VLESS/VMess WebSocket & HTTPUpgrade ALPN Stripping
- **Where**: `src-tauri/src/config/generator.rs` (`generate_config`)
- **Rule**: Whenever an outbound has `type == "vless" || type == "vmess"` with `transport.type == "ws" || "httpupgrade"` and `tls.enabled == true`, any `"h2"` (HTTP/2) entry **must** be stripped from `tls.alpn`. If the remaining ALPN array is empty, it **must** be set to `["http/1.1"]`.
- **Why**: CDNs (Cloudflare, Nginx) negotiate HTTP/2 (`h2`) by default if ALPN is omitted or includes `h2`. RFC 6455 WebSocket `Upgrade` headers and HTTPUpgrade transports use HTTP/1.1 text framing; negotiating `h2` causes immediate stream resets (`EOF`) or binary framing parser errors (`malformed HTTP response`). Protocols legitimately requiring `h2` (e.g. gRPC, native HTTP/2, TCP Reality Vision) retain explicit `h2`.

### Invariant 2: TUN Route Exclusions & Locale-Independent DNS Discovery
- **Where**: `src-tauri/src/config/generator.rs` (`build_inbounds`) & `src-tauri/src/os/mod.rs` (`get_system_dns_server`)
- **Rule**: `tun-in` `route_exclude_address` **must** always include:
  1. RFC-1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), loopback (`127.0.0.0/8`), link-local (`169.254.0.0/16`), CGNAT (`100.64.0.0/10`), multicast (`224.0.0.0/4`).
  2. Bootstrap DNS (`1.1.1.1/32`, `8.8.8.8/32`).
  3. The active physical adapter's system DNS server (`{system_dns}/32`) discovered via the Windows C API `GetAdaptersAddresses` (never use `ipconfig` regex parsing, which breaks on non-English Windows).
  4. All pre-resolved outbound proxy server IPs (`resolve_outbound_ips`).
- **Why**: Without these exclusions, `local-dns` queries or encrypted outbound proxy packets get re-captured by the TUN interface, creating an infinite routing loop.

### Invariant 3: Dual-Store Synchronization (`@tauri-apps/plugin-store` vs Rust `std::fs`)
- **Where**: `src/utils/store.ts` (`storeHelper`) & `src/stores/*.ts`
- **Rule**: `@tauri-apps/plugin-store` caches JSON files in memory in the JavaScript runtime. Whenever a Rust command writes directly to `profiles.json`, `settings.json`, or `routing.json` on disk (e.g., `create_empty_profile`, `refresh_subscription`, `apply_previewed_subscription_refresh`, `import_config_backup`, or background `subscription-auto-updated`), the frontend **must** call `await storeHelper.reloadProfiles()` / `reloadSettings()` / `reloadRouting()` (`store.reload({ ignoreDefaults: true })`) before reading from the store or calling `store.save()`.
- **Why**: If the frontend calls `store.save()` without reloading after a native write, the stale in-memory JS cache will overwrite and erase the changes written by Rust!

### Invariant 4: Parent Death Binding & Zombie Prevention
- **Where**: `src-tauri/src/state.rs` (`SidecarChild`), `src-tauri/src/os/*.rs`, `src-tauri/src/bin/*-supervisor.rs`
- **Rule**: Never run blind system-wide termination (`taskkill /F /IM sing-box.exe` or `pkill sing-box`). Instead, bind all child process lifecycles directly to the host OS supervisor:
  1. **Windows**: The child process is added to a Windows Job Object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, ensuring immediate kernel-level termination upon parent exit.
  2. **Linux**: Launched via an unprivileged companion supervisor (`xlink-linux-supervisor`) with `libc::PR_SET_PDEATHSIG` set to `SIGTERM`, monitoring parent `stdin` EOF, and escalating to `SIGKILL` after 3 seconds.
  3. **macOS**: Launched via a root companion supervisor (`xlink-darwin-supervisor`) deployed in `/Library/Application Support/X-Link/helpers/` (mode `0755`, owned by `root:wheel`), invoked via `sudo -n`, and monitored via `kqueue` `EVFILT_PROC` / `NOTE_EXIT` and `stdin` EOF heartbeats.
  4. **Value-Move Termination**: `SidecarChild::kill(mut self)` consumes `self` by value, closing standard input pipes (`child.stdin.take()`) to trigger clean graceful shutdown with defensive fallback to `child.start_kill()`.
- **Why**: Eliminates orphan zombie processes across ungraceful parent crashes, power loss, or force-quits without risking termination of unrelated processes on the host machine.

### Invariant 5: Explicit Mode Transitions & Connected-Apply Policy
- **Where**: `src/utils/applyPolicy.ts`, `src/utils/connectedApply.ts`, `src/stores/connectionStore.ts`, `src/stores/settingsStore.ts`, `src-tauri/src/commands/proxy.rs`
- **Rule**:
  - **No Silent TUN Fallback (User Action)**: In v0.9.0, silent `modeOverride = 'system'` fallback during standard user connection attempts is strictly forbidden. If TUN mode initiation fails due to missing OS elevation or missing virtual adapter drivers, the application must present a structured diagnostic error modal rather than silently downgrading the connection security posture.
  - **Automated Graceful Fallback for Zero-Data Networks**: As documented in [[10-Zero-Data-and-Proxy-Node-Troubleshooting]], when connecting over restrictive Zero-Data ISP cellular networks (such as Dialog) where raw UDP/TCP packets are dropped and cause TUN probe timeouts, the Rust proxy controller (`commands/proxy.rs`) executes an automated graceful fallback to System Proxy mode to prevent a total connectivity blackout while logging the fallback event.
  - **Deferred Mode Switching**: `proxyMode` is a **`defer`** setting: changing it while connected saves the preference for the next session and must **not** silently restart or switch the active session's mode.
  - **Debounced Rollback Pipeline**: All `restart`-class settings and routing edits while connected must flow through the `800ms` debounced `createApplier` pipeline so invalid edits or failed restarts automatically roll back to the pre-edit `baseline`.

### Invariant 6: Authenticated Local Clash Controller
- **Where**: `src-tauri/src/config/generator.rs` & `src/utils/store.ts` (`migrateApiSettings`)
- **Rule**: `experimental.clash_api` must bind strictly to `127.0.0.1:{apiPort}` and must **never** be emitted into `configs/active.json` if `apiSecret.trim()` is empty. All Rust commands querying Clash API (`get_traffic_stats`, `get_active_connections`, `switch_node_hot`, `test_proxy_url_latency`) must attach `Authorization: Bearer {secret}`.

### Invariant 7: Privacy Consent Gate for Geolocation
- **Where**: `src-tauri/src/commands/geo.rs` (`lookup_node_geo`) & `src/stores/profileStore.ts` (`fetchNodeGeo`)
- **Rule**: External GeoIP lookups (`freeipapi.com`) are disabled by default (`geoLookupEnabled: false`) and must verify `settings.geoLookupEnabled == true` in **both** frontend and Rust backend, and must only send literal global public IP addresses (never domain names or private IPs).

---

## 2. Step-by-Step Recipes for Common Modifications

### Recipe A: How to Add a New Setting
1. **Define Type & Default (`src/utils/store.ts`)**:
   - Add the property to `export interface Settings` and `export const DEFAULT_SETTINGS`.
2. **Classify Apply Behavior (`src/utils/applyPolicy.ts`)**:
   - If the setting is purely UI/OS-side, add its key to `LIVE_KEYS`.
   - If it should only take effect on next connect, add to `DEFER_KEYS`.
   - Otherwise, it defaults to `'restart'` (automatically handled by the debounced cold-restart applier). Add any range validation to `validateSettingsForApply()`.
3. **Add Initial State (`src/stores/settingsStore.ts`)**:
   - Add the default value to the initial `settings` object in `useSettingsStore`.
4. **Consume in Rust Config Generator (`src-tauri/src/config/generator.rs`)**:
   - Read `settings.and_then(|s| s.get("yourNewKey"))` with a matching fallback default and emit into the generated `sing-box` JSON.
5. **Expose in UI (`src/views/SettingsView.tsx`)**:
   - Add a `<FormSwitchRow>`, `<FormSelect>`, or `<FormInput>` in the appropriate `settingsTab` section (`general`, `tun`, `sniff`, `dns`, `api`, `diagnostics`).

---

### Recipe B: How to Add a New Tauri IPC Command
1. **Implement Command (`src-tauri/src/commands/<module>.rs`)**:
   - Annotate with `#[tauri::command]` and return `Result<T, String>` where `T: serde::Serialize`.
2. **Export & Register (`src-tauri/src/commands/mod.rs` & `src-tauri/src/lib.rs`)**:
   - Ensure the module is public in `commands/mod.rs` and add `commands::<module>::<your_command>` to `tauri::generate_handler![...]` in `src-tauri/src/lib.rs`.
3. **Invoke from Frontend**:
   - Call `await invoke<ReturnType>('your_command', { camelCaseParam })`.
   - If the command mutates `settings.json`, `profiles.json`, or `routing.json` on disk, immediately call `await storeHelper.reload*()` in the calling Zustand store action.

---

### Recipe C: How to Add or Modify an Outbound Proxy Protocol
1. **Share Link / Clash YAML Parsing (`src-tauri/src/config/adapters/mod.rs`)**:
   - Add `parse_<proto>_url()` in `parse_share_links()` and/or a match arm in `convert_clash_proxy()`.
2. **Generator Normalization (`src-tauri/src/config/generator.rs`)**:
   - Ensure `generate_config()` preserves protocol-specific fields and excludes UDP/tunnel protocols from TCP `smux` multiplexing if incompatible.
3. **Frontend Node Editor (`src/stores/nodeEditorStore.ts` & `src/components/domain/NodeEditor.tsx`)**:
   - Add form state fields to `NodeEditorState`, hydrate them in `openEditor(node)`, serialize them in `saveEditor()`, and render the inputs in `NodeEditor.tsx`.

---

## 3. Verification & Testing Commands

When verifying changes in `C:\Projects\X-Link`:
- **Frontend Unit Tests (Vitest)**:
  ```powershell
  npm test
  ```
  *(Runs `vitest run` across `src/utils/*.test.ts` and `src/stores/*.test.ts`)*
- **Frontend Typecheck & Production Bundle**:
  ```powershell
  npm run build
  ```
  *(Runs `tsc && vite build`)*
- **Rust Backend Compilation & Unit Tests**:
  ```powershell
  cargo test --manifest-path src-tauri/Cargo.toml
  ```
  *(Runs all unit tests across `generator.rs`, `adapters/mod.rs`, `rules.rs`, `os/mod.rs`, `proxy.rs`, `config.rs`, `diagnostics.rs`, `backup.rs`, `geo.rs`, and `system.rs`)*

---

## 4. UI Architecture & Modification Workflow

UI development follows the design patterns documented in [[09-Design-System]], [[04-UI-Wireframes]], and [[02-Frontend-UI-Architecture]]: **design alignment → plan → tasks → verified commit**. Before editing anything under `src/components`, `src/views` or `src/*.css`:

1. Review [[09-Design-System]], [[04-UI-Wireframes]], and [[03-UI-Components-and-Views]].
2. UI styling work **never** changes Rust commands, IPC payloads, store schemas or the 7 invariants above without architectural approval.
3. Verify changes with `npm test` and `npm run build`.

### Recipe D: Add an icon
1. Add the name to `src/components/ui/icon-names.ts` (typed allow-list) — the compiler rejects unknown names.
2. Use `<Icon name="…" size={20} />`; never a raw `<md-icon>`.
3. Run `npm run build`. Verify in a `tauri build --debug` window, not only `tauri dev`.

### Recipe E: Add or change a view so it fits the window
1. Body is `grid-template-rows: …; min-height: 0`; exactly one `.scroll-region` per list/table.
2. No `vh`/`vw`, no fixed `max-height`, no raw `@media` — respect desktop window bounds (canonical `950×650`, minimum `380×550`).
3. Long text gets `min-width: 0` + ellipsis + `title`.

