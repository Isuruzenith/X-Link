use tauri::State;
use uuid::Uuid;
use std::time::SystemTime;
use tauri::Manager;
use crate::config::adapters::adapt;
use crate::config::generator::generate_singbox_config;
use crate::config::validator::validate_singbox_config;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub profile_type: String, // "url", "file", "manual"
    pub subscription_url: Option<String>,
    pub config_path: String,
    pub last_updated: u64,
    pub node_count: u32,
}

#[tauri::command]
pub async fn import_subscription(
    app: tauri::AppHandle,
    state: State<'_, crate::state::ProxyState>,
    url: String,
    name: String,
) -> Result<Profile, String> {
    // 1. Fetch subscription content strictly via Rust backend (bypass CORS)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client.get(&url)
        .header("User-Agent", "clash") // Some providers block simple scrapers
        .send()
        .await
        .map_err(|e| format!("subscription_fetch_failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("subscription_fetch_failed: HTTP Status {}", resp.status()));
    }

    let bytes = resp.bytes().await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    // 2. Adapt configuration
    let outbounds = adapt(&bytes)?;
    let node_count = outbounds.len() as u32;

    // 3. Generate new Profile metadata
    let id = Uuid::new_v4().to_string();
    let config_path = crate::config::get_config_path(&app, &id)?;

    // 4. Generate sing-box JSON config based on active settings.json proxyMode
    let proxy_mode = {
        let mut mode = "system".to_string();
        if let Ok(mut path) = app.path().app_data_dir() {
            path.push("settings.json");
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(m) = val.get("proxyMode").and_then(|v| v.as_str()) {
                            mode = m.to_string();
                        }
                    }
                }
            }
        }
        mode
    };

    let mixed_port = *state.mixed_port.lock().unwrap();
    let (dns_address, sni_host, listen_address) = {
        let mut dns: Option<String> = None;
        let mut sni = "".to_string();
        let mut wifi = false;
        if let Ok(mut path2) = app.path().app_data_dir() {
            path2.push("settings.json");
            if path2.exists() {
                if let Ok(content2) = std::fs::read_to_string(&path2) {
                    if let Ok(val2) = serde_json::from_str::<serde_json::Value>(&content2) {
                        if let Some(d) = val2.get("dnsAddress").and_then(|v| v.as_str()) {
                            if !d.trim().is_empty() {
                                dns = Some(d.to_string());
                            }
                        }
                        if let Some(s) = val2.get("sniHost").and_then(|v| v.as_str()) {
                            sni = s.to_string();
                        }
                        if let Some(w) = val2.get("wifiSharing").and_then(|v| v.as_bool()) {
                            wifi = w;
                        }
                    }
                }
            }
        }
        let addr = if wifi { "0.0.0.0".to_string() } else { "127.0.0.1".to_string() };
        let resolved_dns = crate::config::resolve_dns_address(dns.as_deref());
        (resolved_dns, sni, addr)
    };
    let generated_config = generate_singbox_config(&app, mixed_port, outbounds, &proxy_mode, &dns_address, &sni_host, &listen_address)?;

    // 5. Write to temporary file for validation
    let temp_id = format!("{}_temp", id);
    let temp_path = crate::config::get_config_path(&app, &temp_id)?;
    std::fs::write(&temp_path, &generated_config)
        .map_err(|e| format!("Failed to write temp config: {}", e))?;

    // 6. Validate using embedded sing-box check
    let validation_res = validate_singbox_config(&app, &temp_path).await;
    
    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    // Report error if validation fails
    validation_res?;

    // 7. Write to actual profile config path
    std::fs::write(&config_path, generated_config)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    let last_updated = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    Ok(Profile {
        id,
        name,
        profile_type: "url".to_string(),
        subscription_url: Some(url),
        config_path: config_path.to_str().unwrap().to_string(),
        last_updated,
        node_count,
    })
}

#[tauri::command]
pub async fn import_file(
    app: tauri::AppHandle,
    state: State<'_, crate::state::ProxyState>,
    file_path: String,
    name: String,
) -> Result<Profile, String> {
    // 1. Read local file content safely
    let bytes = std::fs::read(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // 2. Adapt configuration via established FormatDispatcher engine
    let outbounds = adapt(&bytes)?;
    let node_count = outbounds.len() as u32;

    // 3. Generate new Profile metadata
    let id = Uuid::new_v4().to_string();
    let config_path = crate::config::get_config_path(&app, &id)?;

    // 4. Generate sing-box JSON config based on active settings.json proxyMode
    let proxy_mode = {
        let mut mode = "system".to_string();
        if let Ok(mut path) = app.path().app_data_dir() {
            path.push("settings.json");
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(m) = val.get("proxyMode").and_then(|v| v.as_str()) {
                            mode = m.to_string();
                        }
                    }
                }
            }
        }
        mode
    };

    let mixed_port = *state.mixed_port.lock().unwrap();
    let (dns_address, sni_host, listen_address) = {
        let mut dns: Option<String> = None;
        let mut sni = "".to_string();
        let mut wifi = false;
        if let Ok(mut path2) = app.path().app_data_dir() {
            path2.push("settings.json");
            if path2.exists() {
                if let Ok(content2) = std::fs::read_to_string(&path2) {
                    if let Ok(val2) = serde_json::from_str::<serde_json::Value>(&content2) {
                        if let Some(d) = val2.get("dnsAddress").and_then(|v| v.as_str()) {
                            if !d.trim().is_empty() {
                                dns = Some(d.to_string());
                            }
                        }
                        if let Some(s) = val2.get("sniHost").and_then(|v| v.as_str()) {
                            sni = s.to_string();
                        }
                        if let Some(w) = val2.get("wifiSharing").and_then(|v| v.as_bool()) {
                            wifi = w;
                        }
                    }
                }
            }
        }
        let addr = if wifi { "0.0.0.0".to_string() } else { "127.0.0.1".to_string() };
        let resolved_dns = crate::config::resolve_dns_address(dns.as_deref());
        (resolved_dns, sni, addr)
    };
    let generated_config = generate_singbox_config(&app, mixed_port, outbounds, &proxy_mode, &dns_address, &sni_host, &listen_address)?;

    // 5. Write to temporary file for validation
    let temp_id = format!("{}_temp", id);
    let temp_path = crate::config::get_config_path(&app, &temp_id)?;
    std::fs::write(&temp_path, &generated_config)
        .map_err(|e| format!("Failed to write temp config: {}", e))?;

    // 6. Validate using embedded sing-box check validator
    let validation_res = validate_singbox_config(&app, &temp_path).await;
    
    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    // Report error if validation fails
    validation_res?;

    // 7. Write to actual profile config path
    std::fs::write(&config_path, generated_config)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    let last_updated = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    Ok(Profile {
        id,
        name,
        profile_type: "file".to_string(),
        subscription_url: Some(file_path),
        config_path: config_path.to_str().unwrap().to_string(),
        last_updated,
        node_count,
    })
}

#[tauri::command]
pub async fn import_from_clipboard(
    app: tauri::AppHandle,
    state: State<'_, crate::state::ProxyState>,
    content: String,
    name: String,
) -> Result<Profile, String> {
    // 1. Read bytes from raw content
    let bytes = content.as_bytes();

    // 2. Adapt configuration via established FormatDispatcher engine
    let outbounds = adapt(bytes)?;
    let node_count = outbounds.len() as u32;

    // 3. Generate new Profile metadata
    let id = Uuid::new_v4().to_string();
    let config_path = crate::config::get_config_path(&app, &id)?;

    // 4. Generate sing-box JSON config based on active settings.json proxyMode
    let proxy_mode = {
        let mut mode = "system".to_string();
        if let Ok(mut path) = app.path().app_data_dir() {
            path.push("settings.json");
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(m) = val.get("proxyMode").and_then(|v| v.as_str()) {
                            mode = m.to_string();
                        }
                    }
                }
            }
        }
        mode
    };

    let mixed_port = *state.mixed_port.lock().unwrap();
    let (dns_address, sni_host, listen_address) = {
        let mut dns: Option<String> = None;
        let mut sni = "".to_string();
        let mut wifi = false;
        if let Ok(mut path2) = app.path().app_data_dir() {
            path2.push("settings.json");
            if path2.exists() {
                if let Ok(content2) = std::fs::read_to_string(&path2) {
                    if let Ok(val2) = serde_json::from_str::<serde_json::Value>(&content2) {
                        if let Some(d) = val2.get("dnsAddress").and_then(|v| v.as_str()) {
                            if !d.trim().is_empty() {
                                dns = Some(d.to_string());
                            }
                        }
                        if let Some(s) = val2.get("sniHost").and_then(|v| v.as_str()) {
                            sni = s.to_string();
                        }
                        if let Some(w) = val2.get("wifiSharing").and_then(|v| v.as_bool()) {
                            wifi = w;
                        }
                    }
                }
            }
        }
        let addr = if wifi { "0.0.0.0".to_string() } else { "127.0.0.1".to_string() };
        let resolved_dns = crate::config::resolve_dns_address(dns.as_deref());
        (resolved_dns, sni, addr)
    };
    let generated_config = generate_singbox_config(&app, mixed_port, outbounds, &proxy_mode, &dns_address, &sni_host, &listen_address)?;

    // 5. Write to temporary file for validation
    let temp_id = format!("{}_temp", id);
    let temp_path = crate::config::get_config_path(&app, &temp_id)?;
    std::fs::write(&temp_path, &generated_config)
        .map_err(|e| format!("Failed to write temp config: {}", e))?;

    // 6. Validate using embedded sing-box check validator
    let validation_res = validate_singbox_config(&app, &temp_path).await;
    
    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    // Report error if validation fails
    validation_res?;

    // 7. Write to actual profile config path
    std::fs::write(&config_path, generated_config)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    let last_updated = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    Ok(Profile {
        id,
        name,
        profile_type: "manual".to_string(),
        subscription_url: Some("Clipboard".to_string()),
        config_path: config_path.to_str().unwrap().to_string(),
        last_updated,
        node_count,
    })
}

#[tauri::command]
pub fn delete_profile(app: tauri::AppHandle, profile_id: String) -> Result<(), String> {
    let path = crate::config::get_config_path(&app, &profile_id)?;
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|e| format!("Failed to delete config file: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_profile_outbound(app: tauri::AppHandle, profile_id: String) -> Result<serde_json::Value, String> {
    let config_path = crate::config::get_config_path(&app, &profile_id)?;
    if !config_path.exists() {
        return Err("Profile configuration file not found".to_string());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    let outbounds = config.get("outbounds")
        .and_then(|o| o.as_array())
        .ok_or_else(|| "No outbounds array found in configuration".to_string())?;

    for outbound in outbounds {
        let outbound_type = outbound.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("");

        if outbound_type != "selector" 
            && outbound_type != "direct" 
            && outbound_type != "block" 
            && outbound_type != "dns" 
        {
            return Ok(outbound.clone());
        }
    }

    Err("No primary proxy outbound found in configuration".to_string())
}

#[tauri::command]
pub fn get_profile_outbounds(app: tauri::AppHandle, profile_id: String) -> Result<Vec<serde_json::Value>, String> {
    let config_path = crate::config::get_config_path(&app, &profile_id)?;
    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    let outbounds = config.get("outbounds")
        .and_then(|o| o.as_array())
        .ok_or_else(|| "No outbounds array found in configuration".to_string())?;

    let mut nodes = Vec::new();
    for outbound in outbounds {
        let outbound_type = outbound.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("");

        if outbound_type != "selector" 
            && outbound_type != "direct" 
            && outbound_type != "block" 
            && outbound_type != "dns" 
        {
            nodes.push(outbound.clone());
        }
    }

    Ok(nodes)
}

#[tauri::command]
pub async fn update_profile_config(
    app: tauri::AppHandle,
    profile_id: String,
    new_outbound: serde_json::Value,
) -> Result<(), String> {
    let config_path = crate::config::get_config_path(&app, &profile_id)?;
    if !config_path.exists() {
        return Err("Profile configuration file not found".to_string());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let mut config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Find the primary outbound index and its old tag
    let mut primary_idx = None;
    let mut old_tag = None;

    {
        let outbounds = config.get_mut("outbounds")
            .and_then(|o| o.as_array_mut())
            .ok_or_else(|| "No outbounds array found in configuration".to_string())?;

        for (idx, outbound) in outbounds.iter().enumerate() {
            let outbound_type = outbound.get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("");

            if outbound_type != "selector" 
                && outbound_type != "direct" 
                && outbound_type != "block" 
                && outbound_type != "dns" 
            {
                primary_idx = Some(idx);
                old_tag = outbound.get("tag").and_then(|t| t.as_str()).map(|s| s.to_string());
                break;
            }
        }
    }

    let primary_idx = primary_idx.ok_or_else(|| "No primary outbound found to update".to_string())?;
    
    // Replace the primary outbound
    {
        let outbounds = config.get_mut("outbounds")
            .and_then(|o| o.as_array_mut())
            .unwrap();
        outbounds[primary_idx] = new_outbound.clone();
    }

    // If tag changed, update the selector tag list
    if let (Some(old), Some(new)) = (old_tag, new_outbound.get("tag").and_then(|t| t.as_str())) {
        if old != new {
            let outbounds = config.get_mut("outbounds")
                .and_then(|o| o.as_array_mut())
                .unwrap();
            for outbound in outbounds.iter_mut() {
                let outbound_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let outbound_tag = outbound.get("tag").and_then(|t| t.as_str()).unwrap_or("");
                if outbound_type == "selector" && outbound_tag == "proxy" {
                    if let Some(selector_outbounds) = outbound.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
                        for item in selector_outbounds.iter_mut() {
                            if item.as_str() == Some(&old) {
                                *item = serde_json::Value::String(new.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Write to temporary file for validation
    let temp_id = format!("{}_temp", profile_id);
    let temp_path = crate::config::get_config_path(&app, &temp_id)?;
    let pretty_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config to JSON: {}", e))?;

    std::fs::write(&temp_path, &pretty_config)
        .map_err(|e| format!("Failed to write temp config: {}", e))?;

    // Validate using embedded sing-box check validator
    let validation_res = validate_singbox_config(&app, &temp_path).await;
    
    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    // Report error if validation fails
    validation_res?;

    // Write to actual profile config path
    std::fs::write(&config_path, pretty_config)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}
