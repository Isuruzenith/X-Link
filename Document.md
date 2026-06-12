# X-Link Core Developer Architecture & Networking Documentation

This document provides a comprehensive technical overview of the X-Link application architecture, connection lifecycle management, advanced routing mechanisms, and DNS configurations designed to prevent deadlocks and optimize performance on Windows.

---

## 1. High-Level System Architecture

X-Link is built on a decoupled three-tier architecture:

```mermaid
graph TD
    UI[Frontend: React + TS + Vite] <-- Tauri IPC (invoke/emit) --> Rust[Backend: Tauri + Rust]
    Rust <-- Sidecar Pipe (stdio/control) --> SB[Proxy Core: sing-box Sidecar]
    SB <-- Routing Rules --> NET[Network Interface / Internet]
```

* **Frontend (React + Vite + TypeScript):** Handles user interactions, manages state for profiles/settings, displays real-time connection status/uptime, and streams bandwidth traffic charts.
* **Backend (Tauri + Rust):** Acts as the supervisor. It bypasses CORS to fetch subscription profiles, validates generated configurations using the sing-box check validator, monitors system tray events, triggers UAC admin elevation gates for TUN mode, and monitors process health.
* **Core Engine (sing-box sidecar):** Spawns as a background sidecar process. It handles tunneling (VLESS, VMess, Trojan, Shadowsocks) and implements the active routing rules.

---

## 2. Profile Parsing & Configuration Adapters

When subscriptions are imported via URL, local file, or clipboard, X-Link executes a unified format dispatcher engine to translate them into native sing-box configurations:

```
Subscription Raw Bytes
       │
       ▼
[Strip Whitespace / Newlines] ──► Try Base64 Decode ──► Success? (Base64 Sub)
       │                                                    │
       ▼ (Fail: Plain Sub)                                  ▼ (Use Decoded Bytes)
[Detect Format] ──► Clash YAML  ──► clash::adapt()
                ──► Raw URIs    ──► raw_uri::adapt()
                ──► Sing-Box JS ──► Native JSON extract
```

### Key Adapter Robustness Features:
1. **Multiline Base64 Decoding:** Standard base64 decoding fails if there are newlines or tabs inside the payload (common with MIME sub files). `adapters::adapt()` strips all whitespace before decoding, and falls back to undecoded bytes if the payload is plain text (like Clash YAML or raw URI list).
2. **VMess Percent-Decoding:** VMess URIs containing percent-encoded characters or padding (e.g., ending with `%3D%3D` instead of `==`) are decoded via `percent_encoding::percent_decode_str()` before base64 parsing.
3. **Clash YAML Field Mapping:** Maps complex Clash fields to sing-box configurations:
   * **TLS & SNI:** Maps `tls: true`, `servername`, and `sni` fields to `tls.server_name` and `tls.enabled`.
   * **REALITY:** Parses `reality-opts` (including `public-key` and `short-id`).
   * **Transport layers:** Translates `ws-opts` (path and host headers), `grpc-opts` (`grpc-service-name`), and `h2-opts`/`http` to sing-box transport objects.

---

## 3. Advanced Routing Architecture (TUN Mode)

When connected in **TUN Mode**, X-Link creates a virtual network interface and routes all system traffic through it. To prevent loop routing and ensure performance, several networking layers are set up:

### A. Virtual Adapter Allocation
* Creates a virtual interface named `X-Link`.
* Configures an IPv4 network segment `172.19.0.1/30` on the interface.
* IPv6 addresses are intentionally omitted to force IPv4-only resolution inside the tunnel and prevent browser connection hangs on bad IPv6 proxy routing.

### B. Circular Routing Prevention (Loop Bypass)
If proxy connection packets enter the TUN interface, they will be routed back into the tunnel, causing an infinite loop. X-Link resolves this dynamically:

1. **Proxy Server IP Resolution:** At startup, `resolve_server_ips()` parses the active configuration outbounds, performs DNS resolution on the proxy domains, and collects all server IP addresses as CIDR blocks (e.g., `192.0.2.1/32`).
2. **Route Exclusions:** `build_route_exclude_addresses()` combines the resolved server IPs with RFC 1918 private subnets and the system DNS IP:
   ```rust
   // Private Subnets + Server IPs + System DNS
   let mut addresses = vec![
       "10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16", 
       "127.0.0.0/8", "169.254.0.0/16", "fc00::/7", "fe80::/10"
   ];
   ```
3. **Auto-Route Injection:** These exclusion blocks are passed to the `route_exclude_address` parameter of the sing-box TUN inbound, telling the kernel to route those packets directly via the physical interface, bypassing the proxy.

---

## 4. DNS Architecture & Deadlock Prevention

DNS configuration is the most critical component in TUN mode. A misconfigured DNS detour will lead to a system-wide internet drop (DNS Deadlock).

```
                      DNS Query (Port 53)
                              │
                              ▼
                     [Sing-Box Intercept]
                              │
               ┌──────────────┴──────────────┐
               ▼ (Normal App Queries)        ▼ (Proxy Server Domain Query)
          [proxy-dns]                   [local-dns]
               │                             │
          (via Proxy)                 (via Direct Detour)
               │                             │
               ▼                             ▼
        [Proxy Tunnel]               [Physical NIC / System DNS]
```

### A. Hijack-DNS Rule
The routing configuration intercepts all port 53 traffic:
```json
{ "protocol": "dns", "action": "hijack-dns" }
```
This forces all applications on the host system to use the sing-box internal DNS resolver.

### B. Two-Server DNS Strategy
To prevent the deadlock where the app cannot resolve the proxy server's domain name because the proxy tunnel is not yet open:
* **`proxy-dns` (App Traffic):** Resolves queries using `tcp://1.1.1.1` detoured through the `proxy` outbound. This ensures queries go securely through the tunnel and avoids DNS leak issues.
* **`local-dns` (Bootstrap Traffic):** Resolves queries using the local system DNS (detected from `ipconfig /all` or `/etc/resolv.conf`) detoured through `direct`.
* **The detoured rule:** Any outbound connection to resolve the proxy server's own endpoint domain matches `{ "outbound": "any", "server": "local-dns" }` and goes directly through physical NIC.

### C. DNS Loop Prevention
During startup, `get_system_dns_address()` parses system adapters. It explicitly ignores X-Link's own virtual DNS addresses (`172.19.0.2` and `fdfe:dcba:9876::2`) to prevent recursive lookup loops where sing-box queries itself.

---

## 5. Spawning & Connection Lifecycle

The sidecar process lifecycle is managed strictly in `proxy.rs` to prevent orphaned process leaks:

1. **UAC Admin Gate:** Before entering TUN mode, the frontend triggers `is_elevated` check. If false, it prompts the UAC elevation flow to spawn the backend as Administrator.
2. **Pre-startup Cleanup:** Runs a preemptive `taskkill` on any running `sing-box.exe` instances to release ports.
3. **Spawning Sidecar:** Launches the `sing-box` sidecar binary with runtime flags:
   - `-c <config_path>`
   - `ENABLE_DEPRECATED_GEOSITE=true` environment variable.
4. **Log Streaming:** Pipes `Stdout` and `Stderr` streams to a ring-buffer and emits them as custom events to the frontend (`sing-box-log`).
5. **Health-check Gate:** Sleeps for `1500ms` and polls for premature termination. If the sidecar terminates (e.g. binding failure), the state is rolled back.
6. **Rollbacks & Cleanups:**
   - If sidecar startup fails, the state resets to `Disconnected`.
   - If setting/enabling the system proxy fails after startup, `perform_clean_cleanup()` runs to terminate the sidecar, reset states, and clean system proxies.
