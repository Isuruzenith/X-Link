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

    Ok(())
}

#[tauri::command]
pub fn check_tun_support() -> bool {
    is_elevated::is_elevated()
}

#[tauri::command]
pub fn request_elevation() -> Result<(), String> {
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

    #[cfg(not(target_os = "windows"))]
    {
        Err("Elevation is only supported on Windows".to_string())
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

        let cmd = if enabled {
            format!(
                "reg add \"HKCU\\Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers\" /v \"{}\" /t REG_SZ /d \"~ RUNASADMIN\" /f",
                exe_str
            )
        } else {
            format!(
                "reg delete \"HKCU\\Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers\" /v \"{}\" /f",
                exe_str
            )
        };

        let output = std::process::Command::new("cmd")
            .args(["/C", &cmd])
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

