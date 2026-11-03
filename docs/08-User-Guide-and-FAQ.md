---
title: "08 — User Guide, Platform Quickstarts & Operational Runbooks"
aliases: ["User Guide", "FAQ", "Troubleshooting", "Runbooks", "Quickstart"]
tags:
  - user-guide
  - runbook
  - troubleshooting
  - faq
  - operations
created: 2026-11-03
updated: 2026-11-03
---

# 08 — User Guide, Platform Quickstarts & Operational Runbooks

**Navigation**: [[00-Home-MOC]] | **Prev**: [[07-AI-Agent-Playbook]] | **Related**: [[01-Architecture-Overview]], [[03-UI-Components-and-Views]], [[06-IPC-Bridge-and-Data-Contracts]]

---

## 1. Introduction & Core Concepts

X-Link is a high-performance, cross-platform private proxy desktop client engineered for security, stability, and zero-jank responsiveness. Powered by **Tauri v2** and the **sing-box v1.13+** core engine, X-Link provides complete protocol versatility alongside native desktop ergonomics.

### 1.1 Operating Modes: TUN vs. System Proxy

| Dimension | TUN Mode (Transparent Network Adapter) | System Proxy Mode |
| :--- | :--- | :--- |
| **Interception Layer** | Network Layer (L3 Virtual TUN Adapter) | Application Layer (OS Proxy Settings) |
| **Traffic Scope** | All system traffic (TCP, UDP, ICMP, DNS) across all processes | HTTP/HTTPS/SOCKS-aware applications (Browsers, Curl, etc.) |
| **Process Privileges** | Requires elevated OS network helper / driver privileges | Runs entirely as standard unprivileged user |
| **Supported Protocols** | Full support for UDP-native protocols (Hysteria2, TUIC, WireGuard, Gaming, VoIP) | TCP applications; UDP depends on client SOCKS5 support |
| **Driver / Helper** | `wintun.dll` (Windows), `utun` + root helper (macOS), capabilities (Linux) | Built-in OS APIs (`WinINet`, `networksetup`, `gsettings`) |
| **Best For** | Full-system privacy, online gaming, CLI tools, and terminal sessions | Lightweight web browsing and environments without admin rights |

---

## 2. Platform-Specific Quickstart Guides

### 2.1 Windows Installation & Setup
1. **Prerequisites**: Windows 10 (Build 19041+) or Windows 11 64-bit.
2. **Installation**:
   - Run the X-Link installer (`X-Link_x64-setup.exe`).
   - The installer bundles the tested `sing-box.exe` and `wintun.dll` network driver.
3. **Running in System Proxy Mode**:
   - Launch X-Link from the Start Menu. System Proxy mode works immediately without requiring Administrator privileges.
4. **Running in TUN Mode**:
   - When switching to TUN mode in **Settings $\rightarrow$ TUN Mode** or clicking **Connect** with TUN selected, Windows will present a User Account Control (UAC) prompt if helper privileges are required.
   - *Kernel Safety*: X-Link binds the proxy engine to a Windows Job Object (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`). If X-Link crashes or is terminated from Task Manager, Windows immediately cleans up the background engine and restores network routing.

### 2.2 macOS Installation & Setup
1. **Prerequisites**: macOS 12 Monterey or later (Apple Silicon & Intel).
2. **Installation**:
   - Mount the DMG and drag `X-Link.app` to `/Applications`.
3. **Window Ergonomics**:
   - X-Link features native macOS overlay window styling. Navigation tabs and header action bars automatically inset by 72px (`pt-[72px]`) to clear the native traffic light buttons (close, minimize, zoom).
4. **Unprivileged GUI & Companion Supervisor**:
   - In accordance with macOS security guidelines, the desktop GUI executes strictly as an unprivileged user.
   - When TUN mode is activated for the first time, X-Link requests administrative approval to install the companion supervisor (`xlink-darwin-supervisor`) into `/Library/Application Support/X-Link/helpers/` (mode `0755`, owned by `root:wheel`) and stages a restricted sudoers entry (`/private/etc/sudoers.d/xlink-singbox`).
   - Subsequent TUN connections execute seamlessly via non-interactive `sudo -n` with parent death binding via `kqueue`.

### 2.3 Linux Installation & Setup
1. **Prerequisites**: Modern Linux distribution with systemd, glibc 2.31+, and Polkit.
2. **Installation**:
   - Available via AppImage, `.deb`, or `.rpm`.
3. **Capability Elevation (Non-Root GUI)**:
   - X-Link never runs the graphical WebKit browser as `root`.
   - On initial launch, X-Link copies helper binaries to persistent storage (`~/.local/share/x-link/helpers/`) and requests capability elevation via Polkit:
     ```bash
     pkexec setcap cap_net_admin,cap_net_bind_service=+ep ~/.local/share/x-link/helpers/sing-box
     ```
   - Process supervision is managed by `xlink-linux-supervisor` utilizing Linux `libc::PR_SET_PDEATHSIG` to guarantee zero zombie processes upon parent termination.

---

## 3. Key Feature Guides

### 3.1 Managing Profiles & Subscriptions
- **Importing Profiles**:
  - Navigate to **Profiles (`ConfigView`)**.
  - Paste any subscription URL, Base64-encoded string, Clash/Mihomo YAML file, or raw share link (`vless://`, `vmess://`, `trojan://`, `ss://`, `hy2://`, `tuic://`, `wg://`).
  - *Live Preview*: As you type or paste, X-Link dry-runs parsing and displays detected server counts and protocol breakdowns.
- **Subscription Diff Preview**:
  - Before applying a subscription refresh, click the **Diff (`📊`)** icon on any subscription card.
  - Inspect added servers (`+`), removed servers (`-`), and retained servers. If the active server was deleted by the provider, X-Link highlights the automatic fallback node before you commit.
- **Auto-Update Scheduling**:
  - Configure subscription cards to update automatically in the background (every `1h`, `6h`, `12h`, or `24h`).

### 3.2 Outbound Groups & Multi-Hop Chaining
X-Link exposes full sing-box outbound routing capabilities directly in the user interface:
- **`urltest` (Auto-Fastest)**: Automatically selects the server with the lowest round-trip latency to a health-check endpoint (Cloudflare/Google). Health-check intervals are strictly floored at 300s to preserve battery and bandwidth.
- **`fallback` (Auto-Failover)**: Directs traffic through the primary server and automatically fails over down the priority list if health checks fail.
- **`selector` (Manual Group)**: Allows grouping servers (e.g., "Streaming", "Gaming") for targeted rule routing.
- **Detour Chaining (Relay $\rightarrow$ Exit)**: Chain two proxy nodes together (e.g., Entry Relay in Tokyo $\rightarrow$ Exit Node in US). Detours are verified before connection via **Kahn's Directed Acyclic Graph (DAG) cycle validator** to prevent circular proxy loops.

### 3.3 Custom Split-Tunneling & Rule Sets
Configure intelligent traffic routing in **Routing (`RoutingView`)**:
- **Match Types**: `domain`, `domain_suffix`, `domain_keyword`, `domain_regex`, `ip_cidr`, `geoip`, `geosite`, `port`, `process_name`, and `rule_set`.
- **Target Actions**: Route matched traffic to `Proxy`, `Direct`, `Block`, `DNS`, or any custom **Outbound Group**.
- **Process Split-Tunneling**: Route specific apps (e.g., `steam.exe`, `discord.exe`) directly while routing browsers through the tunnel. On Windows, click **"Pick running process"** to select active applications from a live system process list.
- **Rule Sets**: Subscribe to compiled binary `.srs` rule sets (e.g., `geosite-category-ads-all`, `geoip-cn`) for high-speed offline matching.

### 3.4 Self-Describing Encrypted v2 Backups
- **Exporting Backups**:
  - In **Settings $\rightarrow$ Diagnostics**, click **Export backup**.
  - Enter an optional passphrase to encrypt your archive using **AES-256-GCM** with an **Argon2id** key derivation function.
- **Inspecting & Restoring**:
  - Click **Inspect backup file** to review archive metadata.
  - If the archive is encrypted, enter your passphrase in the secure challenge modal to unlock the envelope.
  - Selectively restore **Profiles & Node Configs**, **App Settings**, or **Routing Rules**.
  - *Safety Guard*: Restoration is strictly disabled while connected to prevent active socket corruption.

### 3.5 Real-Time Telemetry & Socket Inspection
- **Throughput Chart**: View live upload and download bandwidth curves with 30-sample smooth cubic Bézier rendering.
- **Socket Inspector (`ConnectionsView`)**:
  - Inspect active network connections with host, IP, protocol (`TCP`/`UDP`), inbound adapter (`tun-in`/`mixed-in`), matched routing rule, and live per-socket throughput.
  - Virtualized rendering via `react-window` guarantees smooth 60 FPS scrolling even with hundreds of concurrent connections.
  - Terminate individual misbehaving sockets or click **Close All** to flush connection pools.

---

## 4. Operational Triage Runbooks

### Runbook 1: Host Internet Connectivity Lost After Force-Quit or Crash

#### Problem Summary
The system reports no internet access, web browsers display `ERR_PROXY_CONNECTION_FAILED`, or domain names fail to resolve after X-Link or the operating system crashed while connected.

#### Root Cause
X-Link was abruptly terminated before it could issue the synchronous OS cleanup command to unregister system proxy registry keys or tear down the virtual TUN adapter.

#### Triage Steps by Operating System

##### On Windows:
1. **Relaunch X-Link**:
   - Launching X-Link automatically triggers the startup self-healing hook, which reads `proxy_state_backup.json` and resets system proxy settings.
2. **Manual System Proxy Reset**:
   - Open **Windows Settings** $\rightarrow$ **Network & Internet** $\rightarrow$ **Proxy**.
   - Under *Manual proxy setup*, ensure **"Use a proxy server"** is toggled **Off**.
3. **Verify via PowerShell**:
   ```powershell
   # Check Windows Internet Settings registry values:
   Get-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings" | Select-Object ProxyEnable, ProxyServer
   
   # Force proxy disable if still active:
   Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings" -Name ProxyEnable -Value 0
   ```
4. **Remove Orphaned Wintun Adapters**:
   - Open Device Manager (`devmgmt.msc`).
   - Expand **Network adapters**. If an adapter named `wintun` or `X-Link TUN` is present with a warning sign, right-click and select **Uninstall device**.

##### On macOS:
1. **Reset Network Proxy via Terminal**:
   ```bash
   # List active network services:
   networksetup -listallnetworkservices
   
   # Disable web and secure web proxies for Wi-Fi:
   networksetup -setwebproxystate "Wi-Fi" off
   networksetup -setsecurewebproxystate "Wi-Fi" off
   networksetup -setsocksfirewallproxystate "Wi-Fi" off
   ```
2. **Check for Orphaned `utun` Interfaces**:
   ```bash
   ifconfig | grep utun
   ```
   *(Orphaned utun interfaces automatically deallocate upon companion supervisor exit; if stuck, a reboot will flush virtual kernel interfaces).*

##### On Linux:
1. **Reset GNOME Desktop Proxy**:
   ```bash
   gsettings set org.gnome.system.proxy mode 'none'
   ```
2. **Clear Environment Variables**:
   ```bash
   unset http_proxy https_proxy all_proxy HTTP_PROXY HTTPS_PROXY ALL_PROXY
   ```

---

### Runbook 2: Inbound Port Conflicts (`7890`, `7891`, `7892`, `9090`)

#### Problem Summary
X-Link displays a red banner: *"Port Conflict: Port 7892 is already in use by another application."* Connecting fails or crashes immediately.

#### Root Cause
Another proxy client (e.g., Clash, v2rayN, Docker, or Fiddler) is actively binding local listener ports.

#### Triage Steps
1. **Identify the Conflicting Process**:
   - On Windows:
     ```powershell
     Get-NetTCPConnection -LocalPort 7892 -ErrorAction SilentlyContinue | Select-Object LocalPort, OwningProcess, State
     Get-Process -Id <OwningProcessId>
     ```
   - On macOS / Linux:
     ```bash
     lsof -i :7892
     ```
2. **Change X-Link Listener Ports**:
   - In X-Link, open **Settings $\rightarrow$ General**.
   - Under **Inbound Ports**, update `Mixed Port` to an available port (e.g., `17892` or `27892`).
   - If using the External API, update `API Port` from `9090` to `19090`.
3. **Real-Time Verification**:
   - The port badges in Settings will immediately update to `● Free` (green).

---

### Runbook 3: Missing Helper / Driver Permissions for TUN Mode

#### Problem Summary
When attempting to connect in TUN mode, X-Link displays: *"TUN mode requires elevated helper privileges"* or connection fails with `driverMissing` / `requiresElevation`.

#### Triage Steps

##### On Windows:
1. **Check Driver File**:
   - Verify that `wintun.dll` exists in the application directory alongside `X-Link.exe`.
   - If missing, re-run the official installer or extract `wintun.dll` from the release archive.
2. **Grant Elevation**:
   - Click the **"Grant Privileges"** button on the Dashboard banner.
   - Accept the Windows UAC elevation prompt.

##### On macOS:
1. **Verify Sudoers Staging**:
   - Open Terminal and inspect `/private/etc/sudoers.d/xlink-singbox`:
     ```bash
     ls -la /private/etc/sudoers.d/xlink-singbox
     ```
   - It must have permissions `-r--r-----` (`0440`) and be owned by `root:wheel`.
2. **Verify Helper Binary**:
   - Check `/Library/Application Support/X-Link/helpers/xlink-darwin-supervisor`.
   - If permissions are corrupted, remove the directory and trigger re-elevation from X-Link Settings:
     ```bash
     sudo rm -rf "/Library/Application Support/X-Link/helpers"
     sudo rm -f "/private/etc/sudoers.d/xlink-singbox"
     ```

##### On Linux:
1. **Verify Binary Capabilities**:
   ```bash
   getcap ~/.local/share/x-link/helpers/sing-box
   ```
   *Expected output*: `cap_net_admin,cap_net_bind_service=ep`.
2. **Re-Apply Capabilities**:
   ```bash
   sudo setcap cap_net_admin,cap_net_bind_service=+ep ~/.local/share/x-link/helpers/sing-box
   ```

---

### Runbook 4: Cyclic Detour Errors (`CYCLIC_DETOUR`)

#### Problem Summary
Attempting to connect or save a configuration fails with error:
`ConfigValidationError: Cyclic detour chain detected: [Node-A -> Node-B -> Node-A]`.

#### Root Cause
Kahn's Directed Acyclic Graph (DAG) cycle validator detected an infinite loop in your 2-hop detour configuration. Node A specifies Node B as its relay detour, while Node B specifies Node A as its detour.

#### Triage Steps
1. **Locate the Offending Nodes**:
   - The error message lists the exact `offendingTags` and `cyclePath`.
2. **Break the Cycle**:
   - Open **Profiles (`ConfigView`)**.
   - Edit the affected nodes in the slide-over **Node Editor**.
   - Set the `Detour` field of the terminal exit node to `None (Direct Egress)`.
3. **Save and Reconnect**:
   - Once the detour chain forms a valid acyclic graph, configuration compilation succeeds.

---

## 5. Frequently Asked Questions (FAQ)

### Q1: What is the difference between TCP ping and URL delay test?
- **TCP Ping (`test_node_latency`)**: Measures raw network round-trip time between your computer and the proxy server's IP/port via a TCP handshake. It does not test whether the proxy protocol, TLS certificate, or credentials work.
- **URL Delay Test (`test_proxy_url_latency`)**: Sends an actual encrypted HTTP GET request through the entire tunnel to an external test server (e.g., `https://cp.cloudflare.com/generate_204`). It confirms end-to-end tunnel operation, proxy authentication, and true usable internet latency.

### Q2: Why does TUN mode use 198.18.0.0/15 for DNS?
X-Link uses **FakeIP (RFC 5735 benchmark range)** for TUN mode. When an application queries a domain name, X-Link instantly assigns a temporary local IP from the `198.18.0.0/15` pool. This eliminates local DNS latency, prevents ISP DNS leaks, and delegates remote DNS resolution directly to the exit proxy node.

### Q3: Does X-Link send telemetry or browse history to external servers?
**No.** X-Link is fully open-source and local-first.
- All routing rules, configurations, and connection stats are stored locally on your machine in `%APPDATA%\com.isuru.x-link\` (Windows) or `~/.config/com.isuru.x-link/` (macOS/Linux).
- *Node Geolocation Lookup* is **opt-in** (`settings.geoLookupEnabled: false` by default). When enabled, it only queries public server IP addresses (never hostnames, domains, or user IPs) via `freeipapi.com`.

### Q4: Can I use X-Link alongside local development servers (Docker, WSL2, Localhost)?
**Yes.** X-Link's TUN mode generator automatically configures `route_exclude_address` covering:
- Loopback (`127.0.0.0/8`, `::1`)
- Local Subnets (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`)
- Link-Local and Multicast (`169.254.0.0/16`, `224.0.0.0/4`)
- Host DNS servers and pre-resolved proxy server IPs
Local connections and development servers will bypass the tunnel without interruption.

### Q5: How do I export diagnostic logs to report an issue?
1. Open **Settings $\rightarrow$ Diagnostics**.
2. Click **Generate Diagnostics**.
3. Review the sanitized preview: all passwords, UUIDs, private keys, and secrets are automatically redacted (`[REDACTED]`).
4. Click **Download Report** or **Copy JSON** and attach it to your bug report.
