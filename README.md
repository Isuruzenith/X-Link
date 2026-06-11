# X-Link Desktop Proxy Client

[![Build and Release](https://github.com/Isuruzenith/X-Link/actions/workflows/release.yml/badge.svg)](https://github.com/Isuruzenith/X-Link/actions/workflows)
[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](https://github.com/Isuruzenith/X-Link)
[![Platform](https://img.shields.io/badge/platform-Windows-blue.svg)]()
[![License](https://img.shields.io/badge/license-MIT-green.svg)]()

**X-Link** is a state-of-the-art desktop proxy client built with **Tauri v2**, **React**, **TypeScript**, and **Rust**. Powered by a custom **sing-box** core sidecar, X-Link provides high-performance, low-latency proxy tunnels with virtual network interface (TUN) support and an elegant, modern visual interface.

---

## Key Features

- **🚀 Multiple Protocol Support**: Native integration for VLESS, VMESS, Trojan, Shadowsocks, Hysteria 2, TUIC, Wireguard, SSH, SOCKS, and HTTP.
- **🛡️ Virtual TUN Interface**: Establishes system-wide proxy tunnels using `wintun.dll` (runs elevated with automated Windows privilege escalation helper).
- **📋 Profile & Subscription Manager**: Easily import subscription configurations, paste profile links from the clipboard, or import custom JSON configurations.
- **🎨 State-Aware System Tray**: Full tray-to-minimize behavior with dynamic, context-aware status icons (Connected, Connecting, Disconnected) and context menus.
- **📊 Real-time Monitoring**: Real-time traffic speedometers, active connections counter, and an interactive canvas-driven traffic graph.
- **⚙️ Advanced Routing & DNS Routing Rules**: Configure GeoIP / GeoSite routing rules, DNS rule server filters, and specify FakeIP/caching behavior.
- **🔌 Sniffing, Mux & API Settings**: Deep packet traffic sniffing, TCP multiplexing (H2Mux / Brutal), and REST API bindings.

---

## Technology Stack

- **Frontend**: React 19, TypeScript, Vite, CSS (Custom Design System with Glassmorphic Elements)
- **Backend**: Rust, Tauri v2
- **Core Engine**: `sing-box` (v1.10.x+ as a sidecar process)
- **TUN Driver**: `wintun.dll` (dynamically copied and configured on boot)

---

## Directory Structure

```text
X-Link/
├── .github/workflows/   # CI/CD Release workflows
├── src/                 # React Frontend application
│   ├── components/      # UI elements & domain editors
│   ├── views/           # Tab view interfaces (Dashboard, Profiles, Settings, etc.)
│   ├── utils/           # Frontend storage helpers and API interfaces
│   └── App.tsx          # Main entry component & state orchestration
├── src-tauri/           # Rust Tauri App
│   ├── src/             # Core Rust modules
│   │   ├── commands/    # Tauri command invocations (System, Profile, Proxy actions)
│   │   ├── config/      # Profile conversion parsers & adapters (Clash / raw URIs)
│   │   ├── os/          # System-level configurations (Windows proxy registers)
│   │   ├── state.rs     # Core shared Rust thread-safe state managers
│   │   └── tray.rs      # System tray setup & dynamically built context menus
│   ├── binaries/        # Precompiled sing-box sidecars & wintun.dll binaries
│   └── tauri.conf.json  # Tauri app settings (permissions, bundle targets, icons)
```

---

## Development Setup

### Prerequisites

Ensure you have the following installed on your machine:
1. **Node.js** (v18.x or v20.x recommended)
2. **Rust & Cargo** (Stable toolchain)
3. **Windows C++ Build Tools** (via Visual Studio Installer)

### Installation & Run

1. Clone the repository:
   ```bash
   git clone https://github.com/Isuruzenith/X-Link.git
   cd X-Link
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Run the development server:
   ```bash
   npm run tauri dev
   ```

---

## Release Builds & CI/CD

This project uses an automated GitHub Actions release workflow configured in [.github/workflows/release.yml](.github/workflows/release.yml).

### Automated Github Action Release
Pushing a semver tag triggers a production build on a `windows-latest` runner:
```bash
# Tag and push to trigger CI
git tag v0.1.0
git push origin v0.1.0
```
This automatically compiles the application, embeds the sidecar dependencies, and publishes `.exe` & `.msi` installers to a **Draft Release** in your repository.

### Manual Local Build
To manually build a production release installer on your local machine, run:
```bash
npx tauri build
```
The compiled output will be generated inside `src-tauri/target/release/bundle/`.

---

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
