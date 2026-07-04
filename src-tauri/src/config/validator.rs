use std::path::Path;
use tauri::AppHandle;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

pub async fn validate_singbox_config(app: &AppHandle, config_path: &Path) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let _ = std::fs::create_dir_all(&app_data_dir);

    let sidecar = app
        .shell()
        .sidecar("sing-box")
        .map_err(|e| format!("Failed to find sing-box sidecar: {}", e))?;

    let output = sidecar
        .args([
            "check",
            "-c",
            config_path.to_str().unwrap(),
            "-D",
            app_data_dir.to_str().unwrap(),
        ])
        .env("ENABLE_DEPRECATED_GEOSITE", "true")
        .env("ENABLE_DEPRECATED_GEOIP", "true")
        .env("ENABLE_DEPRECATED_LEGACY_DNS_FAKEIP_OPTIONS", "true")
        .env("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS", "true")
        .env("ENABLE_DEPRECATED_OUTBOUND_DNS_RULE_ITEM", "true")
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
