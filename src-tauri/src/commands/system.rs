use crate::state::ProxyState;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

#[tauri::command]
pub async fn get_singbox_version(
    app: tauri::AppHandle,
    state: tauri::State<'_, ProxyState>,
) -> Result<String, String> {
    // Return cached version if available
    if let Ok(guard) = state.singbox_version.lock() {
        if let Some(ref cached) = *guard {
            return Ok(cached.clone());
        }
    }

    let sidecar = app
        .shell()
        .sidecar("sing-box")
        .map_err(|e| format!("Failed to find sing-box sidecar: {}", e))?;

    let output = sidecar
        .args(["version"])
        .output()
        .await
        .map_err(|e| format!("Failed to run sing-box: {}", e))?;

    let version_str =
        parse_singbox_version_output(output.status.success(), &output.stdout, &output.stderr)?;

    // Cache the result
    if let Ok(mut guard) = state.singbox_version.lock() {
        *guard = Some(version_str.clone());
    }
    Ok(version_str)
}

/// Parses the output and exit status of `sing-box version`.
pub fn parse_singbox_version_output(
    status_success: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<String, String> {
    if !status_success {
        let err_str = String::from_utf8_lossy(stderr).trim().to_string();
        return if err_str.is_empty() {
            Err("sing-box version failed with non-zero exit status".to_string())
        } else {
            Err(format!("sing-box version failed: {}", err_str))
        };
    }

    let out_str = String::from_utf8_lossy(stdout).trim().to_string();
    if out_str.is_empty() {
        return Err("sing-box version output is empty".to_string());
    }

    Ok(out_str)
}

/// Pinned and tested sing-box version bundled with X-Link (defined in scripts/setup-sidecars.cjs).
pub const TESTED_SINGBOX_VERSION: &str = "1.13.14";

/// Supported sing-box major version.
pub const SUPPORTED_SINGBOX_MAJOR: u64 = 1;

/// Supported sing-box minor version series.
pub const SUPPORTED_SINGBOX_MINOR: u64 = 13;

/// Structured semantic version representation for sing-box sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingBoxVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub prerelease: Option<String>,
    pub build: Option<String>,
    pub raw: String,
}

impl SingBoxVersion {
    /// Formats the semantic version components as "major.minor.patch" (plus prerelease/build if present).
    #[allow(dead_code)]
    pub fn semver_string(&self) -> String {
        let mut s = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if let Some(ref pre) = self.prerelease {
            s.push('-');
            s.push_str(pre);
        }
        if let Some(ref b) = self.build {
            s.push('+');
            s.push_str(b);
        }
        s
    }

    /// Parses a semantic version string (e.g. "1.13.14", "v1.13.14", "1.13.14-rc.1").
    pub fn parse(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("Version string cannot be empty".to_string());
        }

        let clean = trimmed.strip_prefix(['v', 'V']).unwrap_or(trimmed);

        let (without_build, build) = match clean.split_once('+') {
            Some((pre_build, b)) => {
                if b.is_empty() {
                    return Err(format!(
                        "Invalid version '{}': empty build metadata after '+'",
                        input
                    ));
                }
                (pre_build, Some(b.to_string()))
            }
            None => (clean, None),
        };

        let (version_core, prerelease) = match without_build.split_once('-') {
            Some((core, pre)) => {
                if pre.is_empty() {
                    return Err(format!(
                        "Invalid version '{}': empty prerelease tag after '-'",
                        input
                    ));
                }
                (core, Some(pre.to_string()))
            }
            None => (without_build, None),
        };

        let parts: Vec<&str> = version_core.split('.').collect();
        if parts.len() != 3 {
            return Err(format!(
                "Invalid version format '{}': expected exactly 3 numeric parts (major.minor.patch)",
                input
            ));
        }

        let major = parts[0]
            .parse::<u64>()
            .map_err(|_| format!("Invalid major version '{}' in '{}'", parts[0], input))?;
        let minor = parts[1]
            .parse::<u64>()
            .map_err(|_| format!("Invalid minor version '{}' in '{}'", parts[1], input))?;
        let patch = parts[2]
            .parse::<u64>()
            .map_err(|_| format!("Invalid patch version '{}' in '{}'", parts[2], input))?;

        Ok(Self {
            major,
            minor,
            patch,
            prerelease,
            build,
            raw: input.trim().to_string(),
        })
    }
}

/// Checks whether a parsed or raw sing-box version satisfies the compatibility contract.
///
/// Compatibility Policy:
/// - Must be a valid semantic version (major.minor.patch).
/// - Must match major version 1 and minor version 13 (sing-box 1.13.x series, tested with 1.13.14).
/// - Versions < 1.13 are rejected (lacking required DNS/routing options and keepalive configurations).
/// - Versions >= 1.14 are explicitly rejected (avoid accepting untested 1.14 with breaking configuration schema changes).
/// - Prerelease versions (e.g., -rc, -beta, -alpha) are rejected to ensure stable proxy operation.
pub fn check_singbox_version_compatibility(raw_input: &str) -> Result<SingBoxVersion, String> {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        return Err("sing-box version output is empty. Cannot verify compatibility.".to_string());
    }

    // Try extracting from standard "sing-box version <SEMVER>" line first,
    // otherwise parse trimmed input directly as semver.
    let version_str = match extract_singbox_version_number(trimmed) {
        Some(v) => v,
        None => trimmed.to_string(),
    };

    let parsed = SingBoxVersion::parse(&version_str).map_err(|err| {
        let preview: String = trimmed
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .chars()
            .take(64)
            .collect();
        format!(
            "Failed to determine sing-box version: unrecognized or malformed version string ('{}'): {}. Expected semantic version format like 1.13.14.",
            preview, err
        )
    })?;

    // Policy check 1: Reject prerelease versions
    if let Some(ref pre) = parsed.prerelease {
        return Err(format!(
            "Unsupported sing-box prerelease version '{}-{}'. X-Link requires a stable release of sing-box {}.x (tested with {}).",
            format!("{}.{}.{}", parsed.major, parsed.minor, parsed.patch),
            pre,
            SUPPORTED_SINGBOX_MINOR,
            TESTED_SINGBOX_VERSION
        ));
    }

    // Policy check 2: Major version mismatch
    if parsed.major != SUPPORTED_SINGBOX_MAJOR {
        return Err(format!(
            "Unsupported sing-box major version {}. X-Link requires sing-box {}.{}.x (tested with {}).",
            parsed.major,
            SUPPORTED_SINGBOX_MAJOR,
            SUPPORTED_SINGBOX_MINOR,
            TESTED_SINGBOX_VERSION
        ));
    }

    // Policy check 3: Minor version lower bound (< 1.13)
    if parsed.minor < SUPPORTED_SINGBOX_MINOR {
        return Err(format!(
            "Unsupported sing-box version '{}.{}.{}'. X-Link requires sing-box {}.{}.x (tested with {}). Versions prior to 1.13 lack required routing and configuration features.",
            parsed.major,
            parsed.minor,
            parsed.patch,
            SUPPORTED_SINGBOX_MAJOR,
            SUPPORTED_SINGBOX_MINOR,
            TESTED_SINGBOX_VERSION
        ));
    }

    // Policy check 4: Minor version upper bound (>= 1.14)
    if parsed.minor > SUPPORTED_SINGBOX_MINOR {
        return Err(format!(
            "Unsupported sing-box version '{}.{}.{}'. X-Link is tested with sing-box {} and does not support sing-box 1.14+ due to incompatible configuration schema changes.",
            parsed.major,
            parsed.minor,
            parsed.patch,
            TESTED_SINGBOX_VERSION
        ));
    }

    Ok(parsed)
}

/// Checks the cached sing-box version string in `ProxyState` against the compatibility policy.
///
/// Returns `Some(Ok(version))` if cached and compatible, `Some(Err(err))` if cached but incompatible,
/// or `None` if no version is cached yet.
pub fn check_cached_singbox_compatibility(
    state: &ProxyState,
) -> Option<Result<SingBoxVersion, String>> {
    if let Ok(guard) = state.singbox_version.lock() {
        if let Some(ref cached) = *guard {
            return Some(check_singbox_version_compatibility(cached));
        }
    }
    None
}

/// Verifies runtime availability and version compatibility of the actual sing-box sidecar binary.
///
/// If a valid compatible version is already cached in `ProxyState`, it is returned immediately.
/// Otherwise, it executes the sidecar with `version`, parses and checks the output, caches the
/// verified version string on success, and returns `Ok(SingBoxVersion)`.
///
/// Fails with an actionable, non-secret error if the binary is unavailable, fails execution,
/// returns malformed output, or has an unsupported/prerelease version.
pub async fn ensure_sidecar_compatible(app: &tauri::AppHandle) -> Result<SingBoxVersion, String> {
    if let Some(state) = app.try_state::<ProxyState>() {
        if let Some(res) = check_cached_singbox_compatibility(&state) {
            return res;
        }
    }

    let sidecar = app
        .shell()
        .sidecar("sing-box")
        .map_err(|e| format!("sing-box sidecar binary is unavailable: {}", e))?;

    let output = sidecar
        .args(["version"])
        .output()
        .await
        .map_err(|e| format!("Failed to execute sing-box sidecar version check: {}", e))?;

    let raw_version =
        parse_singbox_version_output(output.status.success(), &output.stdout, &output.stderr)
            .map_err(|e| format!("sing-box version check failed: {}", e))?;

    let version = check_singbox_version_compatibility(&raw_version)?;

    if let Some(state) = app.try_state::<ProxyState>() {
        if let Ok(mut guard) = state.singbox_version.lock() {
            *guard = Some(raw_version);
        }
    }

    Ok(version)
}

/// Extracts semantic version number from sing-box version output (e.g. "1.13.14").
#[allow(dead_code)]
pub fn extract_singbox_version_number(version_output: &str) -> Option<String> {
    for line in version_output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("sing-box version ") {
            let ver = rest.split_whitespace().next().unwrap_or("").trim();
            if !ver.is_empty() {
                return Some(ver.to_string());
            }
        }
    }
    None
}

#[tauri::command]
pub fn get_app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn is_minimized_launch() -> bool {
    std::env::args().any(|arg| arg == "--minimized")
}

#[tauri::command]
pub fn show_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .show()
            .map_err(|e| format!("Failed to show window: {}", e))?;
        window
            .set_focus()
            .map_err(|e| format!("Failed to focus window: {}", e))?;
    }
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortStatus {
    port: u16,
    ownership: &'static str,
}

fn classify_port_ownership(
    bound: bool,
    active: bool,
    owner_pid: Option<u32>,
    child_pid: Option<u32>,
) -> &'static str {
    if !bound {
        "free"
    } else if owner_pid.is_some() && owner_pid == child_pid && active {
        "own"
    } else if owner_pid.is_some() {
        "external"
    } else {
        "unknown"
    }
}

#[cfg(target_os = "windows")]
fn socket_listener_owners(port: u16, family: u32, tcp: bool) -> Option<Vec<u32>> {
    use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID,
        MIB_UDP6ROW_OWNER_PID, MIB_UDPROW_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
        UDP_TABLE_OWNER_PID,
    };
    use windows::Win32::Networking::WinSock::AF_INET;

    let mut size = 0u32;
    // SAFETY: A null buffer with size 0 requests the required table size.
    let result = unsafe {
        if tcp {
            GetExtendedTcpTable(
                None,
                &mut size,
                false,
                family,
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            )
        } else {
            GetExtendedUdpTable(None, &mut size, false, family, UDP_TABLE_OWNER_PID, 0)
        }
    };
    if result != ERROR_INSUFFICIENT_BUFFER.0 || !(4..=8 * 1024 * 1024).contains(&size) {
        return None;
    }

    let mut buffer = vec![0u32; (size as usize).div_ceil(4)];
    // SAFETY: The allocation is at least `size` bytes and aligned for the table header.
    let result = unsafe {
        if tcp {
            GetExtendedTcpTable(
                Some(buffer.as_mut_ptr().cast()),
                &mut size,
                false,
                family,
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            )
        } else {
            GetExtendedUdpTable(
                Some(buffer.as_mut_ptr().cast()),
                &mut size,
                false,
                family,
                UDP_TABLE_OWNER_PID,
                0,
            )
        }
    };
    if result != 0 || size < 4 || size as usize > buffer.len() * 4 {
        return None;
    }

    let bytes = size as usize;
    let count = buffer[0] as usize;
    let row_size = match (tcp, family == AF_INET.0 as u32) {
        (true, true) => std::mem::size_of::<MIB_TCPROW_OWNER_PID>(),
        (true, false) => std::mem::size_of::<MIB_TCP6ROW_OWNER_PID>(),
        (false, true) => std::mem::size_of::<MIB_UDPROW_OWNER_PID>(),
        (false, false) => std::mem::size_of::<MIB_UDP6ROW_OWNER_PID>(),
    };
    if count > (bytes - 4) / row_size {
        return None;
    }

    let base = buffer.as_ptr().cast::<u8>();
    let mut owners = Vec::new();
    for index in 0..count {
        // SAFETY: The checked count and row size keep every read within the buffer.
        let ptr = unsafe { base.add(4 + index * row_size) };
        let (local_port, pid) = match (tcp, family == AF_INET.0 as u32) {
            (true, true) => {
                let row = unsafe { std::ptr::read_unaligned(ptr.cast::<MIB_TCPROW_OWNER_PID>()) };
                (row.dwLocalPort, row.dwOwningPid)
            }
            (true, false) => {
                let row = unsafe { std::ptr::read_unaligned(ptr.cast::<MIB_TCP6ROW_OWNER_PID>()) };
                (row.dwLocalPort, row.dwOwningPid)
            }
            (false, true) => {
                let row = unsafe { std::ptr::read_unaligned(ptr.cast::<MIB_UDPROW_OWNER_PID>()) };
                (row.dwLocalPort, row.dwOwningPid)
            }
            (false, false) => {
                let row = unsafe { std::ptr::read_unaligned(ptr.cast::<MIB_UDP6ROW_OWNER_PID>()) };
                (row.dwLocalPort, row.dwOwningPid)
            }
        };
        if u16::from_be(local_port as u16) == port {
            owners.push(pid);
        }
    }
    Some(owners)
}

#[cfg(target_os = "windows")]
fn listener_owner(port: u16) -> Option<u32> {
    use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6};
    let mut owners = Vec::new();
    for tcp in [true, false] {
        owners.extend(socket_listener_owners(port, AF_INET.0 as u32, tcp)?);
        owners.extend(socket_listener_owners(port, AF_INET6.0 as u32, tcp)?);
    }
    if owners.is_empty() || owners.iter().any(|pid| *pid != owners[0]) {
        None
    } else {
        Some(owners[0])
    }
}

fn port_is_bound(port: u16) -> bool {
    use std::net::{TcpListener, UdpSocket};

    // Probe both wildcard and loopback binds: operating systems can permit a wildcard
    // bind to coexist with a listener bound only to a specific local address.
    let tcp_v4_loopback = TcpListener::bind(("127.0.0.1", port)).is_err();
    let tcp_v4_wildcard = TcpListener::bind(("0.0.0.0", port)).is_err();
    let tcp_v6_loopback = TcpListener::bind(("[::1]", port)).is_err();
    let tcp_v6_wildcard = TcpListener::bind(("[::]", port)).is_err();
    let udp_v4_loopback = UdpSocket::bind(("127.0.0.1", port)).is_err();
    let udp_v4_wildcard = UdpSocket::bind(("0.0.0.0", port)).is_err();
    let udp_v6_loopback = UdpSocket::bind(("[::1]", port)).is_err();
    let udp_v6_wildcard = UdpSocket::bind(("[::]", port)).is_err();
    tcp_v4_loopback
        | tcp_v4_wildcard
        | tcp_v6_loopback
        | tcp_v6_wildcard
        | udp_v4_loopback
        | udp_v4_wildcard
        | udp_v6_loopback
        | udp_v6_wildcard
}

#[tauri::command]
pub fn check_port_status(
    state: tauri::State<'_, ProxyState>,
    ports: Vec<u16>,
) -> Result<Vec<PortStatus>, String> {
    let settings = state.get_settings();
    let running = state.get_status() == crate::state::ConnectionStatus::Connected;
    let active_ports = if settings.use_separate_ports {
        vec![settings.http_port, settings.socks_port]
    } else {
        vec![settings.mixed_port]
    };
    let child_pid = state
        .child_process
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|child| child.pid()));
    Ok(ports
        .into_iter()
        .map(|port| {
            let bound = port_is_bound(port);
            #[cfg(target_os = "windows")]
            let owner = if bound { listener_owner(port) } else { None };
            #[cfg(not(target_os = "windows"))]
            let owner = None;
            PortStatus {
                port,
                ownership: classify_port_ownership(
                    bound,
                    running && active_ports.contains(&port),
                    owner,
                    child_pid,
                ),
            }
        })
        .collect())
}

#[tauri::command]
pub fn check_port_conflict(ports: Vec<u16>) -> Result<Option<u16>, String> {
    Ok(ports.into_iter().find(|port| port_is_bound(*port)))
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| "Failed to convert executable path to string".to_string())?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        let output = if enabled {
            let value = format!("\"{}\" --minimized", exe_str);
            std::process::Command::new("reg")
                .args([
                    "add",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "X-Link",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &value,
                    "/f",
                ])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output()
                .map_err(|e| format!("Failed to execute autostart registration: {}", e))?
        } else {
            std::process::Command::new("reg")
                .args([
                    "delete",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "X-Link",
                    "/f",
                ])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output()
                .map_err(|e| format!("Failed to execute autostart deletion: {}", e))?
        };

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(format!("Registry autostart command failed: {}", err.trim()));
        }
    }

    #[cfg(target_os = "macos")]
    {
        let home_dir = dirs::home_dir().ok_or("Failed to get user home directory")?;
        let launch_agents_dir = home_dir.join("Library").join("LaunchAgents");
        let plist_file = launch_agents_dir.join("com.xlink.app.plist");

        if enabled {
            std::fs::create_dir_all(&launch_agents_dir)
                .map_err(|e| format!("Failed to create LaunchAgents directory: {}", e))?;

            let content = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
                 <plist version=\"1.0\">\n\
                 <dict>\n\
                     <key>Label</key>\n\
                     <string>com.xlink.app</string>\n\
                     <key>ProgramArguments</key>\n\
                     <array>\n\
                         <string>{}</string>\n\
                         <string>--minimized</string>\n\
                     </array>\n\
                     <key>RunAtLoad</key>\n\
                     <true/>\n\
                 </dict>\n\
                 </plist>",
                exe_str
            );

            std::fs::write(&plist_file, content)
                .map_err(|e| format!("Failed to write LaunchAgent plist: {}", e))?;
        } else if plist_file.exists() {
            std::fs::remove_file(&plist_file)
                .map_err(|e| format!("Failed to delete LaunchAgent plist: {}", e))?;
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home_dir = dirs::home_dir().ok_or("Failed to get user home directory")?;
        let autostart_dir = home_dir.join(".config").join("autostart");
        let desktop_file = autostart_dir.join("x-link.desktop");

        if enabled {
            std::fs::create_dir_all(&autostart_dir)
                .map_err(|e| format!("Failed to create autostart directory: {}", e))?;

            let content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name=X-Link\n\
                 Comment=X-Link Proxy Client\n\
                 Exec=\"{}\" --minimized\n\
                 Terminal=false\n\
                 X-GNOME-Autostart-enabled=true\n",
                exe_str
            );

            std::fs::write(&desktop_file, content)
                .map_err(|e| format!("Failed to write desktop autostart entry: {}", e))?;
        } else if desktop_file.exists() {
            std::fs::remove_file(&desktop_file)
                .map_err(|e| format!("Failed to delete desktop autostart entry: {}", e))?;
        }
    }

    Ok(())
}

#[tauri::command]
pub fn check_tun_support(app: tauri::AppHandle) -> bool {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        crate::os::is_elevated()
    }

    #[cfg(target_os = "linux")]
    {
        if crate::os::is_elevated() {
            return true;
        }

        // Check sidecar capabilities on Linux
        let current_exe = match std::env::current_exe() {
            Ok(path) => path,
            Err(_) => return false,
        };
        let current_dir = match current_exe.parent() {
            Some(path) => path,
            None => return false,
        };

        let mut sidecar_path = None;
        if let Ok(entries) = std::fs::read_dir(current_dir) {
            for entry in entries.flatten() {
                let filename = entry.file_name().to_string_lossy().into_owned();
                if filename.starts_with("sing-box") {
                    sidecar_path = Some(entry.path());
                    break;
                }
            }
        }

        let sidecar_path = match sidecar_path {
            Some(path) => path,
            None => match app
                .path()
                .resolve("resources/sing-box", tauri::path::BaseDirectory::Resource)
            {
                Ok(path) => path,
                Err(_) => return false,
            },
        };

        if let Ok(output) = std::process::Command::new("getcap")
            .arg(&sidecar_path)
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("cap_net_admin") {
                return true;
            }
        }
        false
    }
}

#[tauri::command]
pub fn request_elevation(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

        let exe_path =
            std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;

        let mut verb: Vec<u16> = std::ffi::OsStr::new("runas").encode_wide().collect();
        verb.push(0);

        let mut file: Vec<u16> = exe_path.as_os_str().encode_wide().collect();
        file.push(0);

        // The elevated process waits for this instance to exit before the
        // single-instance plugin claims the application mutex.
        let handoff = format!("--elevation-handoff={}", std::process::id());
        let mut params: Vec<u16> = std::ffi::OsStr::new(&handoff).encode_wide().collect();
        params.push(0);

        unsafe {
            let result = ShellExecuteW(
                None,
                PCWSTR(verb.as_ptr()),
                PCWSTR(file.as_ptr()),
                PCWSTR(params.as_ptr()),
                None,
                SW_SHOW,
            );

            if result.0 as isize <= 32 {
                return Err("Elevation request denied or failed".to_string());
            }
        }

        // ShellExecuteW only confirms launch. Leave the elevated instance to
        // claim the mutex after this one finishes its owned-process cleanup.
        crate::tray::perform_clean_cleanup(&app);
        std::process::exit(0);
    }

    #[cfg(target_os = "macos")]
    {
        let exe_path =
            std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;

        let script = format!(
            "do shell script \"'{}'\" with administrator privileges",
            exe_path.to_str().ok_or("Invalid path encoding")?
        );

        let _ = std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|e| format!("Failed to run osascript: {}", e))?;

        std::process::exit(0);
    }

    #[cfg(target_os = "linux")]
    {
        let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let current_dir = current_exe.parent().ok_or("Failed to get exe directory")?;

        let mut sidecar_path = None;
        if let Ok(entries) = std::fs::read_dir(current_dir) {
            for entry in entries.flatten() {
                let filename = entry.file_name().to_string_lossy().into_owned();
                if filename.starts_with("sing-box") {
                    sidecar_path = Some(entry.path());
                    break;
                }
            }
        }

        let sidecar_path = match sidecar_path {
            Some(path) => path,
            None => app
                .path()
                .resolve("resources/sing-box", tauri::path::BaseDirectory::Resource)
                .map_err(|e| format!("Failed to resolve sing-box sidecar path: {}", e))?,
        };

        let output = std::process::Command::new("pkexec")
            .args([
                "setcap",
                "cap_net_admin,cap_net_bind_service=+ep",
                sidecar_path.to_str().ok_or("Invalid path encoding")?,
            ])
            .output()
            .map_err(|e| format!("Failed to run pkexec: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(format!("Failed to set capabilities: {}", err.trim()));
        }

        Ok(())
    }
}

#[tauri::command]
pub fn set_runas_admin(enabled: bool) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;
        let exe_str = exe_path
            .to_str()
            .ok_or_else(|| "Failed to convert executable path to string".to_string())?;

        // In debug builds, avoid registering target\debug\xlink.exe with RUNASADMIN.
        // Doing so causes `cargo run` / `tauri dev` to fail with OS error 740 (elevation required)
        // when launched from a standard non-elevated terminal.
        if cfg!(debug_assertions) {
            let _ = std::process::Command::new("reg")
                .args([
                    "delete",
                    "HKCU\\Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers",
                    "/v",
                    exe_str,
                    "/f",
                ])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output();

            if enabled {
                log::debug!(
                    "Skipping set_runas_admin(true) in debug mode to prevent cargo error 740"
                );
                return Ok(());
            }
        }

        let mut command = std::process::Command::new("reg");
        if enabled {
            command.args([
                "add",
                "HKCU\\Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers",
                "/v",
                exe_str,
                "/t",
                "REG_SZ",
                "/d",
                "~ RUNASADMIN",
                "/f",
            ]);
        } else {
            command.args([
                "delete",
                "HKCU\\Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers",
                "/v",
                exe_str,
                "/f",
            ]);
        }

        let output = command
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output()
            .map_err(|e| format!("Registry exec failed: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            if enabled || !err.contains("unable to find the specified registry key") {
                return Err(format!(
                    "Registry runas_admin command failed: {}",
                    err.trim()
                ));
            }
        }
    }

    Ok(())
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
}

#[cfg(target_os = "windows")]
pub fn list_running_processes() -> Vec<ProcessInfo> {
    #[repr(C)]
    struct PROCESSENTRY32W {
        dw_size: u32,
        cnt_usage: u32,
        th32_process_id: u32,
        th32_default_heap_id: usize,
        th32_module_id: u32,
        cnt_threads: u32,
        th32_parent_process_id: u32,
        pc_pri_class_base: i32,
        dw_flags: u32,
        sz_exe_file: [u16; 260],
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(dw_flags: u32, th32_process_id: u32) -> isize;
        fn Process32FirstW(h_snapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        fn Process32NextW(h_snapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        fn CloseHandle(h_object: isize) -> i32;
    }

    const TH32CS_SNAPPROCESS: u32 = 0x00000002;
    let mut procs = Vec::new();
    let mut seen = std::collections::HashSet::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == 0 || snapshot == -1 {
            return procs;
        }

        let mut entry = std::mem::zeroed::<PROCESSENTRY32W>();
        entry.dw_size = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let null_pos = entry
                    .sz_exe_file
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.sz_exe_file.len());
                let exe_name = String::from_utf16_lossy(&entry.sz_exe_file[..null_pos]);
                let trimmed = exe_name.trim();
                if !trimmed.is_empty() && seen.insert(trimmed.to_lowercase()) {
                    procs.push(ProcessInfo {
                        name: trimmed.to_string(),
                        pid: entry.th32_process_id,
                    });
                }
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    procs.sort_by_key(|p| p.name.to_lowercase());
    procs
}

#[cfg(target_os = "linux")]
pub fn list_running_processes() -> Vec<ProcessInfo> {
    let mut procs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if let Ok(pid) = name.parse::<u32>() {
                    let comm_path = entry.path().join("comm");
                    if let Ok(comm) = std::fs::read_to_string(comm_path) {
                        let trimmed = comm.trim().to_string();
                        if !trimmed.is_empty() && seen.insert(trimmed.to_lowercase()) {
                            procs.push(ProcessInfo { name: trimmed, pid });
                        }
                    }
                }
            }
        }
    }
    procs.sort_by_key(|p| p.name.to_lowercase());
    procs
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn list_running_processes() -> Vec<ProcessInfo> {
    Vec::new()
}

#[tauri::command]
pub fn get_running_processes() -> Result<Vec<ProcessInfo>, String> {
    Ok(list_running_processes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_ownership_distinguishes_free_owned_external_and_unknown() {
        assert_eq!(classify_port_ownership(false, false, None, None), "free");
        assert_eq!(
            classify_port_ownership(true, true, Some(42), Some(42)),
            "own"
        );
        assert_eq!(
            classify_port_ownership(true, true, Some(84), Some(42)),
            "external"
        );
        assert_eq!(
            classify_port_ownership(true, true, None, Some(42)),
            "unknown"
        );
        assert_eq!(
            classify_port_ownership(true, false, Some(84), None),
            "external"
        );
    }

    #[test]
    fn port_is_bound_detects_tcp_and_udp_listeners() {
        use std::net::{TcpListener, UdpSocket};

        let tcp = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        assert!(port_is_bound(tcp.local_addr().unwrap().port()));
        drop(tcp);

        let udp = UdpSocket::bind(("127.0.0.1", 0)).unwrap();
        assert!(port_is_bound(udp.local_addr().unwrap().port()));
        drop(udp);
    }

    #[test]
    fn test_parse_singbox_version_output_valid() {
        let sample_output =
            b"sing-box version 1.13.14\n\nEnvironment: go1.26.4 windows/amd64\nTags: with_gvisor\n";
        let parsed = parse_singbox_version_output(true, sample_output, b"")
            .expect("should parse valid version output");
        assert!(parsed.contains("sing-box version 1.13.14"));

        let version_num = extract_singbox_version_number(&parsed);
        assert_eq!(version_num.as_deref(), Some("1.13.14"));
    }

    #[test]
    fn test_parse_singbox_version_output_process_failure_with_stderr() {
        let stderr = b"fatal: permission denied";
        let err = parse_singbox_version_output(false, b"", stderr).unwrap_err();
        assert_eq!(err, "sing-box version failed: fatal: permission denied");
    }

    #[test]
    fn test_parse_singbox_version_output_process_failure_empty_stderr() {
        let err = parse_singbox_version_output(false, b"", b"").unwrap_err();
        assert_eq!(err, "sing-box version failed with non-zero exit status");
    }

    #[test]
    fn test_parse_singbox_version_output_empty_stdout() {
        let err = parse_singbox_version_output(true, b"   \n\t  ", b"").unwrap_err();
        assert_eq!(err, "sing-box version output is empty");
    }

    #[test]
    fn test_extract_singbox_version_number_various_formats() {
        assert_eq!(
            extract_singbox_version_number("sing-box version 1.14.0-rc.1\n"),
            Some("1.14.0-rc.1".to_string())
        );
        assert_eq!(
            extract_singbox_version_number("   sing-box version 1.12.0   "),
            Some("1.12.0".to_string())
        );
        assert_eq!(extract_singbox_version_number("some random text"), None);
        assert_eq!(extract_singbox_version_number("sing-box version"), None);
        assert_eq!(extract_singbox_version_number(""), None);
    }

    #[test]
    fn test_singbox_version_parse_valid() {
        let v1 = SingBoxVersion::parse("1.13.14").expect("standard semver parses");
        assert_eq!(v1.major, 1);
        assert_eq!(v1.minor, 13);
        assert_eq!(v1.patch, 14);
        assert_eq!(v1.prerelease, None);
        assert_eq!(v1.build, None);
        assert_eq!(v1.semver_string(), "1.13.14");

        let v2 = SingBoxVersion::parse("v1.13.14").expect("leading 'v' stripped");
        assert_eq!(v2.major, 1);
        assert_eq!(v2.minor, 13);
        assert_eq!(v2.patch, 14);

        let v3 = SingBoxVersion::parse("1.13.0").expect("zero patch parses");
        assert_eq!(v3.major, 1);
        assert_eq!(v3.minor, 13);
        assert_eq!(v3.patch, 0);

        let v4 = SingBoxVersion::parse("1.13.14-rc.1").expect("prerelease parses");
        assert_eq!(v4.major, 1);
        assert_eq!(v4.minor, 13);
        assert_eq!(v4.patch, 14);
        assert_eq!(v4.prerelease.as_deref(), Some("rc.1"));
        assert_eq!(v4.semver_string(), "1.13.14-rc.1");

        let v5 =
            SingBoxVersion::parse("1.14.0-alpha.2+20261031").expect("prerelease and build parse");
        assert_eq!(v5.major, 1);
        assert_eq!(v5.minor, 14);
        assert_eq!(v5.patch, 0);
        assert_eq!(v5.prerelease.as_deref(), Some("alpha.2"));
        assert_eq!(v5.build.as_deref(), Some("20261031"));
        assert_eq!(v5.semver_string(), "1.14.0-alpha.2+20261031");

        let v6 = SingBoxVersion::parse("1.13.14+wintun").expect("build metadata parses");
        assert_eq!(v6.major, 1);
        assert_eq!(v6.minor, 13);
        assert_eq!(v6.patch, 14);
        assert_eq!(v6.prerelease, None);
        assert_eq!(v6.build.as_deref(), Some("wintun"));
        assert_eq!(v6.semver_string(), "1.13.14+wintun");
    }

    #[test]
    fn test_singbox_version_parse_malformed() {
        assert!(SingBoxVersion::parse("").is_err());
        assert!(SingBoxVersion::parse("   ").is_err());
        assert!(SingBoxVersion::parse("1.13").is_err());
        assert!(SingBoxVersion::parse("1.13.14.0").is_err());
        assert!(SingBoxVersion::parse("1.13.x").is_err());
        assert!(SingBoxVersion::parse("abc").is_err());
        assert!(SingBoxVersion::parse("1.13.14-").is_err());
        assert!(SingBoxVersion::parse("1.13.14+").is_err());
        assert!(SingBoxVersion::parse("-1.13.14").is_err());
    }

    #[test]
    fn test_check_singbox_version_compatibility_accepted() {
        // Full multiline output as generated by sing-box version
        let sample_output =
            "sing-box version 1.13.14\n\nEnvironment: go1.26.4 windows/amd64\nTags: with_gvisor\n";
        let v1 = check_singbox_version_compatibility(sample_output)
            .expect("sample 1.13.14 is compatible");
        assert_eq!(v1.major, 1);
        assert_eq!(v1.minor, 13);
        assert_eq!(v1.patch, 14);

        // Compact single-line output
        let v2 = check_singbox_version_compatibility("sing-box version 1.13.14")
            .expect("compact 1.13.14 compatible");
        assert_eq!(v2.semver_string(), "1.13.14");

        // Single line with commit info
        let v3 = check_singbox_version_compatibility("sing-box version 1.13.14 (commit: 25a600d)")
            .expect("output with commit compatible");
        assert_eq!(v3.major, 1);
        assert_eq!(v3.minor, 13);
        assert_eq!(v3.patch, 14);

        // Direct semver strings
        let v4 = check_singbox_version_compatibility("1.13.14")
            .expect("direct semver string compatible");
        assert_eq!(v4.patch, 14);

        let v5 =
            check_singbox_version_compatibility("v1.13.14").expect("v-prefixed semver compatible");
        assert_eq!(v5.patch, 14);

        // Base of 1.13 series
        let v6 = check_singbox_version_compatibility("sing-box version 1.13.0")
            .expect("1.13.0 compatible");
        assert_eq!(v6.patch, 0);

        // Subsequent patch in 1.13 series
        let v7 = check_singbox_version_compatibility("sing-box version 1.13.15")
            .expect("1.13.15 compatible");
        assert_eq!(v7.patch, 15);

        // Build metadata attached
        let v8 = check_singbox_version_compatibility("sing-box version 1.13.14+build.20261031")
            .expect("build metadata compatible");
        assert_eq!(v8.patch, 14);
    }

    #[test]
    fn test_check_singbox_version_compatibility_unsupported_future_minor() {
        // Sing-box 1.14 introduces breaking configuration schema changes and deprecation removals
        let err1 = check_singbox_version_compatibility("sing-box version 1.14.0").unwrap_err();
        assert!(err1.contains("1.14+"));
        assert!(err1.contains("incompatible configuration schema changes"));
        assert!(err1.contains("1.14.0"));

        let err2 = check_singbox_version_compatibility("1.14.5").unwrap_err();
        assert!(err2.contains("1.14+"));

        let err3 = check_singbox_version_compatibility("sing-box version 1.15.0").unwrap_err();
        assert!(err3.contains("1.14+"));
    }

    #[test]
    fn test_check_singbox_version_compatibility_unsupported_major() {
        let err1 = check_singbox_version_compatibility("sing-box version 2.0.0").unwrap_err();
        assert!(err1.contains("major version 2"));

        let err2 = check_singbox_version_compatibility("sing-box version 0.13.14").unwrap_err();
        assert!(err2.contains("major version 0"));
    }

    #[test]
    fn test_check_singbox_version_compatibility_unsupported_legacy() {
        let err1 = check_singbox_version_compatibility("sing-box version 1.12.0").unwrap_err();
        assert!(err1
            .contains("Versions prior to 1.13 lack required routing and configuration features"));
        assert!(err1.contains("1.12.0"));

        let err2 = check_singbox_version_compatibility("1.11.8").unwrap_err();
        assert!(err2.contains("Versions prior to 1.13 lack required routing"));
    }

    #[test]
    fn test_check_singbox_version_compatibility_prerelease_rejected() {
        let err1 =
            check_singbox_version_compatibility("sing-box version 1.13.14-rc.1").unwrap_err();
        assert!(err1.contains("Unsupported sing-box prerelease version"));
        assert!(err1.contains("1.13.14-rc.1"));
        assert!(err1.contains("requires a stable release"));

        let err2 =
            check_singbox_version_compatibility("sing-box version 1.13.14-beta.2").unwrap_err();
        assert!(err2.contains("Unsupported sing-box prerelease version"));

        let err3 = check_singbox_version_compatibility("1.14.0-alpha.1").unwrap_err();
        assert!(err3.contains("Unsupported sing-box prerelease version"));

        let err4 =
            check_singbox_version_compatibility("sing-box version 1.13.0-preview.1").unwrap_err();
        assert!(err4.contains("Unsupported sing-box prerelease version"));
    }

    #[test]
    fn test_check_singbox_version_compatibility_malformed_and_empty() {
        let err1 = check_singbox_version_compatibility("").unwrap_err();
        assert!(err1.contains("sing-box version output is empty"));

        let err2 = check_singbox_version_compatibility("   \n\t  ").unwrap_err();
        assert!(err2.contains("sing-box version output is empty"));

        let err3 = check_singbox_version_compatibility("some random text with no version info")
            .unwrap_err();
        assert!(err3.contains("unrecognized or malformed version string"));

        let err4 = check_singbox_version_compatibility("sing-box version ").unwrap_err();
        assert!(err4.contains("unrecognized or malformed version string"));

        let err5 = check_singbox_version_compatibility("sing-box version 1.13").unwrap_err();
        assert!(err5.contains("expected exactly 3 numeric parts"));

        let err6 = check_singbox_version_compatibility("sing-box version 1.13.x").unwrap_err();
        assert!(err6.contains("Invalid patch version"));
    }

    #[test]
    fn test_check_cached_singbox_compatibility() {
        let state = ProxyState::new();

        // Initially no cached version
        assert!(check_cached_singbox_compatibility(&state).is_none());

        // Cache a valid 1.13.14 version
        *state.singbox_version.lock().unwrap() =
            Some("sing-box version 1.13.14\nTags: with_gvisor".to_string());
        let res1 = check_cached_singbox_compatibility(&state).expect("should return cached result");
        assert!(res1.is_ok());
        let v1 = res1.unwrap();
        assert_eq!(v1.semver_string(), "1.13.14");

        // Cache an incompatible 1.14.0 version
        *state.singbox_version.lock().unwrap() = Some("sing-box version 1.14.0".to_string());
        let res2 = check_cached_singbox_compatibility(&state).expect("should return cached result");
        assert!(res2.is_err());
        assert!(res2.unwrap_err().contains("1.14+"));

        // Cache an unsupported prerelease version
        *state.singbox_version.lock().unwrap() = Some("sing-box version 1.13.14-rc.1".to_string());
        let res3 = check_cached_singbox_compatibility(&state).expect("should return cached result");
        assert!(res3.is_err());
        assert!(res3.unwrap_err().contains("prerelease"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_set_runas_admin_debug_mode_safe() {
        // In debug mode, calling set_runas_admin(true) should succeed without leaving RUNASADMIN
        assert!(set_runas_admin(true).is_ok());
    }

    #[test]
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    fn test_list_running_processes_returns_processes() {
        let procs = list_running_processes();
        assert!(
            !procs.is_empty(),
            "Running processes list should not be empty"
        );
        assert!(procs.iter().all(|p| !p.name.is_empty()));
    }

    #[test]
    #[ignore = "requires installed sing-box sidecar binary"]
    fn test_live_singbox_binary_version() {
        let mut cmd =
            std::process::Command::new("src-tauri/binaries/sing-box-x86_64-pc-windows-msvc.exe");
        let output = cmd.arg("version").output().expect("binary should run");
        let ver =
            parse_singbox_version_output(output.status.success(), &output.stdout, &output.stderr)
                .expect("binary should produce valid version string");
        assert!(ver.contains("sing-box version"));
    }
}
