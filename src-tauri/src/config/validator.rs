use std::path::Path;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

pub async fn validate_singbox_config(
    app: &AppHandle,
    config_path: &Path,
) -> Result<(), String> {
    let sidecar = app.shell().sidecar("sing-box")
        .map_err(|e| format!("Failed to find sing-box sidecar: {}", e))?;

    let output = sidecar
        .args(["check", "-c", config_path.to_str().unwrap()])
        .env("ENABLE_DEPRECATED_GEOSITE", "true")
        .output()
        .await
        .map_err(|e| format!("Failed to execute sing-box check: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let err_str = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(format!("sing-box check failed: {}", err_str.trim()))
    }
}
