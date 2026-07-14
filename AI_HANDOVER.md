# AI Handover — X-Link Architecture Guide

> **Audience**: AI coding agents and new developers.
> Read this document **before** making changes to the X-Link codebase.

This guide documents the critical, non-obvious architectural decisions in X-Link. Violating these invariants **will** break connectivity for users, often silently.

---

## Table of Contents

- [1. VLESS ALPN Self-Healing](#1-vless-alpn-self-healing)
- [2. Routing Loop Exclusions](#2-routing-loop-exclusions)
- [3. FakeIP DNS Optimizations](#3-fakeip-dns-optimizations)
- [4. Connection Self-Healing (5-Stage Fallback)](#4-connection-self-healing-5-stage-fallback)
- [5. Key File Map](#5-key-file-map)

---

## 1. VLESS ALPN Self-Healing

**Location**: [`src-tauri/src/config/generator.rs`](src-tauri/src/config/generator.rs) — outbound post-processing loop

### The Problem

When a VLESS or VMess outbound uses **WebSocket** transport over TLS, the TLS handshake negotiates an Application-Layer Protocol. If `h2` (HTTP/2) is included in the ALPN list, sing-box will attempt RFC 8441 WebSocket-over-HTTP/2 tunneling. However, the vast majority of reverse proxies (Nginx, Caddy, HAProxy) and upstream Xray/V2Ray servers **do not support RFC 8441** and will immediately drop the connection with an `EOF` or `RST` error.

### The Rule

> **INVARIANT**: Any outbound with `transport.type == "ws"` MUST have its TLS ALPN forced to `["http/1.1"]` only. The `"h2"` value must be stripped.

### How It Works

During config generation, after building each outbound object:

1. If TLS is enabled and no ALPN is set, default to `["http/1.1"]`.
2. If the transport type is `"ws"`, **unconditionally overwrite** ALPN to `["http/1.1"]` regardless of what the user or URI parser specified.

```rust
// Self-heal: If transport type is "ws", force ALPN to http/1.1.
if transport.get("type") == Some("ws") {
    tls.insert("alpn", json!(["http/1.1"]));
}
```

### Do NOT

- Allow user-provided ALPN values to pass through unchecked for WebSocket outbounds.
- Add `"h2"` to any WebSocket outbound's ALPN list.
- Remove or weaken this post-processing step.

---

## 2. Routing Loop Exclusions

**Location**: [`src-tauri/src/config/mod.rs`](src-tauri/src/config/mod.rs) — `build_route_exclude_addresses()`

### The Problem

In TUN mode, all system traffic is routed through the virtual network interface. Without explicit exclusions, traffic destined for the proxy server itself, local DNS, or LAN resources would loop back through the tunnel — creating a routing loop that kills connectivity.

### The Rule

> **INVARIANT**: The `route_exclude_address` list in the TUN inbound must always include RFC-1918 ranges, loopback addresses, the system's DNS server IP, and ALL proxy server IPs.

### Excluded Address Ranges (Hardcoded)

| Range | Purpose |
|-------|---------|
| `10.0.0.0/8` | Private network (Class A) |
| `172.16.0.0/12` | Private network (Class B) |
| `192.168.0.0/16` | Private network (Class C) |
| `127.0.0.0/8` | Loopback |
| `169.254.0.0/16` | Link-local |
| `fc00::/7` | IPv6 Unique Local |
| `fe80::/10` | IPv6 Link-local |

### Dynamic Exclusions

In addition to the hardcoded ranges, the following are added dynamically at config generation time:

1. **System DNS server IP** — resolved from the user's active network adapter (parsed as `/32` or `/128`).
2. **Proxy server IPs** — each server hostname is DNS-resolved, and the resulting IPs are appended to the exclusion list (via `resolve_server_ips()`).

### Do NOT

- Remove any of the hardcoded RFC-1918 or loopback exclusions.
- Skip DNS resolution of proxy server hostnames before building the exclusion list.
- Assume proxy servers are always specified by IP (they are often hostnames).

---

## 3. FakeIP DNS Optimizations

**Location**: [`src-tauri/src/config/generator.rs`](src-tauri/src/config/generator.rs) — DNS section builder, [`src-tauri/src/state.rs`](src-tauri/src/state.rs) — defaults

### The Problem

In TUN mode, the system's DNS queries are intercepted by sing-box. Using real DNS resolution for every connection introduces latency and potential DNS leaks (where queries bypass the tunnel). FakeIP solves both by returning synthetic IP addresses from a reserved range, deferring real resolution to the remote proxy server.

### The Rule

> **INVARIANT**: When `dns_mode == "fakeip"`, the DNS section must include a FakeIP server with `inet4_range: "198.18.0.0/15"` and a catch-all rule routing A/AAAA queries to it. Private/local domains must be excluded via the FakeIP filter.

### How It Works

1. **FakeIP Range**: `198.18.0.0/15` (65,536 addresses from the IANA benchmark range). This range is safe to use because it's reserved for network testing and will never conflict with real internet addresses.

2. **FakeIP Filter** (default: `geosite:private`): Domains matching this filter bypass FakeIP and resolve via the `local-dns` server instead. This prevents local resources (`.lan`, `.local`, `.internal`, `.home.arpa`, `localhost`) from getting fake IPs.

3. **DNS Server Chain**:
   - `local-dns` → system resolver, detoured `direct` (for local/private domains)
   - `remote-dns` → encrypted DNS (e.g., `https://dns.google/dns-query`), detoured via `proxy`
   - `fakeip-dns` → FakeIP server (catch-all for A/AAAA queries)

4. **DNS Hijack**: `inet4_address` on the TUN interface is set to `172.19.0.1/30` and DNS is hijacked to ensure all system DNS queries flow through sing-box.

### Do NOT

- Change the FakeIP range from `198.18.0.0/15` without understanding the downstream implications (e.g., route rules, firewall expectations).
- Remove the `geosite:private` filter — this will break local network access.
- Set `ENABLE_DEPRECATED_LEGACY_DNS_FAKEIP_OPTIONS` env var to `"false"` — it's required for backwards compatibility with some sing-box versions.

---

## 4. Connection Self-Healing (5-Stage Fallback)

**Location**: [`src-tauri/src/commands/proxy.rs`](src-tauri/src/commands/proxy.rs) — `patch_config_for_fallback()` and the retry loop

### The Problem

Different ISPs, networks, and server configurations can cause TLS handshake failures for different reasons: censorship fingerprinting, DNS poisoning, TLS version incompatibility, or TUN driver issues. A single connection strategy cannot cover all scenarios.

### The Rule

> **INVARIANT**: The connection startup must attempt up to 5 progressively more permissive configurations before declaring failure. If in TUN mode and all 5 fail, it must automatically roll back to System Proxy mode.

### Fallback Sequence

| Attempt | Strategy | What Changes |
|---------|----------|--------------|
| 1 (default) | Chrome uTLS fingerprint | Initial config as generated — standard approach |
| 2 | Firefox uTLS fingerprint | `utls.fingerprint` changed to `"firefox"` on all VLESS/VMess/Trojan outbounds |
| 3 | Native Go TLS | `utls.enabled` set to `false` — disables fingerprint spoofing entirely |
| 4 | Public DNS bootstrap | Adds `1.1.1.1` and `8.8.8.8` as direct-detoured DNS servers; redirects proxy domain resolution to them |
| 5 | TUN compatibility profile | Applies `apply_tun_compatibility_profile()` — relaxes strict routing and interface settings |
| **Rollback** | System Proxy mode | If all 5 TUN attempts fail, re-generates config for `"system"` mode and retries once |

### How It Works

1. The config is generated and written to disk.
2. sing-box is spawned as a sidecar process.
3. After spawning, a health check (`run_startup_health_checks()`) probes the proxy to verify connectivity.
4. If the health check fails, the sidecar is killed, the config file is patched in-place by `patch_config_for_fallback(attempt)`, and the loop retries.
5. On success, the loop breaks and the connection is declared active.

### Do NOT

- Skip the health check after spawning — a successful spawn does not mean the proxy works.
- Change the fallback order — it's designed to go from most restrictive (best security) to most permissive.
- Remove the TUN → System Proxy rollback — this is the user's safety net when TUN drivers fail entirely.
- Add more than 500ms delay between retry attempts — the user is watching a "Connecting..." spinner.

---

## 5. Key File Map

| File | Responsibility |
|------|---------------|
| `src-tauri/src/config/generator.rs` | Generates the complete sing-box JSON config (outbounds, DNS, routes, TUN) |
| `src-tauri/src/config/mod.rs` | Route exclusion builder, network adapter detection, TUN compatibility profiles |
| `src-tauri/src/config/adapters/raw_uri.rs` | Parses `vless://`, `vmess://`, `trojan://`, `ss://` URIs into structured node data |
| `src-tauri/src/commands/proxy.rs` | Connection lifecycle: spawn, health-check, fallback loop, rollback |
| `src-tauri/src/commands/latency.rs` | Latency pinging with DoH bypass for FakeIP environments |
| `src-tauri/src/state.rs` | Global application state: settings, TUN defaults, FakeIP defaults |
| `scripts/setup-sidecars.cjs` | Pre-build script that downloads the correct `sing-box` binary + `wintun.dll` |
| `src/App.tsx` | Main React entry point and state orchestration |
