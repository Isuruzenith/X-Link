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
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
    }
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

        let mut params: Vec<u16> = std::ffi::OsStr::new("").encode_wide().collect();
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

        // Elevation succeeded, shut down this non-elevated window instance
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
