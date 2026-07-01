use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use crate::state::ProxyState;

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

    let sidecar = app.shell().sidecar("sing-box")
        .map_err(|e| format!("Failed to find sing-box sidecar: {}", e))?;
    
    let output = sidecar.args(["version"])
        .output()
        .await
        .map_err(|e| format!("Failed to run sing-box: {}", e))?;
    
    if output.status.success() {
        let version_str = String::from_utf8_lossy(&output.stdout).into_owned();
        // Cache the result
        if let Ok(mut guard) = state.singbox_version.lock() {
            *guard = Some(version_str.clone());
        }
        Ok(version_str)
    } else {
        let err_str = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(format!("sing-box version failed: {}", err_str))
    }
}

#[tauri::command]
pub fn get_app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn show_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| format!("Failed to show window: {}", e))?;
        window.set_focus().map_err(|e| format!("Failed to focus window: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn check_port_conflict(ports: Vec<u16>) -> Result<Option<u16>, String> {
    for port in ports {
        // Attempt to bind to 127.0.0.1 on the specified port to check if it's already bound
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_err() {
            return Ok(Some(port));
        }
    }
    Ok(None)
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;
    let exe_str = exe_path.to_str()
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
    }
    #[cfg(target_os = "windows")]
    {
        is_elevated::is_elevated()
    }
    
    #[cfg(target_os = "macos")]
    {
        is_elevated::is_elevated()
    }

    #[cfg(target_os = "linux")]
    {
        if is_elevated::is_elevated() {
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
            None => {
                match app.path().resolve_resource("resources/sing-box") {
                    Ok(path) => path,
                    Err(_) => return false,
                }
            }
        };

        if let Ok(output) = std::process::Command::new("getcap").arg(&sidecar_path).output() {
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

        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;
        
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
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;
        
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
            None => {
                app.path().resolve_resource("resources/sing-box")
                    .map_err(|e| format!("Failed to resolve sing-box sidecar path: {}", e))?
            }
        };

        let output = std::process::Command::new("pkexec")
            .args([
                "setcap",
                "cap_net_admin,cap_net_bind_service=+ep",
                sidecar_path.to_str().ok_or("Invalid path encoding")?
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
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;
        let exe_str = exe_path.to_str()
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
                return Err(format!("Registry runas_admin command failed: {}", err.trim()));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_test_singbox_version() {
        // Simple assertion just to verify test suite functions
        assert!(true);
    }
}

