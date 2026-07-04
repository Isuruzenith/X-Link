use crate::commands::proxy::load_tun_settings;
use crate::config::adapters::adapt;
use crate::config::generator::generate_singbox_config;
use crate::config::validator::validate_singbox_config;
use std::path::PathBuf;
use std::time::SystemTime;
use tauri::{Manager, State};

/// Metadata returned to the frontend when importing a profile.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMeta {
    pub id: String,
    pub name: String,
    pub imported_at: u64,
    pub node_count: u32,
}

/// The single, fixed "active" config file path: <app_data_dir>/configs/active.json
/// This is always the file sing-box boots from.
pub fn get_active_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    path.push("configs");
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create configs directory: {}", e))?;
    path.push("active.json");
    Ok(path)
}

/// Per-profile config file: <app_data_dir>/configs/{profile_id}.json
pub fn get_profile_config_path(
    app: &tauri::AppHandle,
    profile_id: &str,
) -> Result<PathBuf, String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    path.push("configs");
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create configs directory: {}", e))?;
    path.push(format!("{}.json", profile_id));
    Ok(path)
}

/// Scratch file used to validate a candidate config before committing it.
fn get_temp_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    path.push("configs");
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create configs directory: {}", e))?;
    path.push("active_temp.json");
    Ok(path)
}

/// Reads proxyMode / dnsAddress / wifiSharing from settings.json (best-effort).
fn read_basic_settings(app: &tauri::AppHandle) -> (String, Option<String>, bool) {
    let mut proxy_mode = "system".to_string();
    let mut dns: Option<String> = None;
    let mut wifi = false;
    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("settings.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(m) = val.get("proxyMode").and_then(|v| v.as_str()) {
                        proxy_mode = m.to_string();
                    }
                    if let Some(d) = val.get("dnsAddress").and_then(|v| v.as_str()) {
                        if !d.trim().is_empty() {
                            dns = Some(d.to_string());
                        }
                    }
                    if let Some(w) = val.get("wifiSharing").and_then(|v| v.as_bool()) {
                        wifi = w;
                    }
                }
            }
        }
    }
    (proxy_mode, dns, wifi)
}

/// Imports a config from raw pasted text/clipboard content, or from a local file path.
/// Always writes to the single fixed "active" config slot -- importing a new config
/// overwrites whatever was previously active, there is no profile list to manage.
#[tauri::command]
pub async fn import_config(
    app: tauri::AppHandle,
    state: State<'_, crate::state::ProxyState>,
    content: Option<String>,
    file_path: Option<String>,
    name: String,
    profile_id: Option<String>,
) -> Result<ProfileMeta, String> {
    // 1. Read raw bytes from whichever source was provided
    let bytes: Vec<u8> = if let Some(fp) = file_path {
        std::fs::read(&fp).map_err(|e| format!("Failed to read file: {}", e))?
    } else if let Some(c) = content {
        c.into_bytes()
    } else {
        return Err("No content or file path provided".to_string());
    };

    // 2. Adapt configuration via the established format-detection engine
    //    (raw URIs, Clash YAML, sing-box JSON, base64 subscription blobs, etc.)
    let outbounds = adapt(&bytes)?;
    let node_count = outbounds.len() as u32;

    // 3. Generate sing-box JSON config based on active settings.json
    let (proxy_mode, dns, wifi) = read_basic_settings(&app);
    let listen_address = if wifi {
        "0.0.0.0".to_string()
    } else {
        "127.0.0.1".to_string()
    };
    let dns_address = crate::config::resolve_dns_address(dns.as_deref());
    let tun_settings = load_tun_settings(&app);
    let mixed_port = state.get_settings().mixed_port;
    let (user_rules, rule_sets) = crate::config::rules::load_routing_rules_from_file(&app);

    let generated_config = generate_singbox_config(
        mixed_port,
        outbounds,
        &proxy_mode,
        &dns_address,
        &listen_address,
        &tun_settings,
        None,
        &user_rules,
        &rule_sets,
    )?;

    // 4. Early guard: validate generated JSON has required structure before
    //    invoking the expensive sing-box binary validator
    {
        let parsed: serde_json::Value = serde_json::from_str(&generated_config)
            .map_err(|e| format!("Generated config is not valid JSON: {}", e))?;
        if parsed
            .get("outbounds")
            .and_then(|o| o.as_array())
            .is_none_or(|a| a.is_empty())
        {
            return Err("Config has no outbounds — nothing to connect to.".to_string());
        }
    }

    // 5. Write to a temporary file and validate with the real sing-box binary
    //    before committing -- this guarantees we never overwrite a working
    //    config with a broken one.
    let temp_path = get_temp_config_path(&app)?;
    std::fs::write(&temp_path, &generated_config)
        .map_err(|e| format!("Failed to write temp config: {}", e))?;

    let validation_res = validate_singbox_config(&app, &temp_path).await;
    let _ = std::fs::remove_file(&temp_path);
    validation_res?;

    // 6. Determine the profile ID — use provided one, or generate a new UUID
    let pid = profile_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // 7. Write to per-profile config AND the active slot
    let profile_path = get_profile_config_path(&app, &pid)?;
    std::fs::write(&profile_path, &generated_config)
        .map_err(|e| format!("Failed to write profile config: {}", e))?;

    let active_path = get_active_config_path(&app)?;
    std::fs::write(&active_path, &generated_config)
        .map_err(|e| format!("Failed to write active config: {}", e))?;

    let imported_at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    Ok(ProfileMeta {
        id: pid,
        name,
        imported_at,
        node_count,
    })
}

/// Returns the customizable proxy outbounds (i.e. not selector/direct/block/dns)
/// from the active config, for display as selectable server "nodes" in the UI.
#[tauri::command]
pub fn get_config_outbounds(app: tauri::AppHandle) -> Result<Vec<serde_json::Value>, String> {
    let config_path = get_active_config_path(&app)?;
    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    let outbounds = config
        .get("outbounds")
        .and_then(|o| o.as_array())
        .ok_or_else(|| "No outbounds array found in configuration".to_string())?;

    let mut nodes = Vec::new();
    for outbound in outbounds {
        let outbound_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
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

/// Returns the outbound the "proxy" selector currently defaults to (i.e. the
/// node that will actually be used to connect), falling back to the first
/// customizable outbound if no default has been set yet.
#[tauri::command]
pub fn get_active_outbound(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let config_path = get_active_config_path(&app)?;
    if !config_path.exists() {
        return Err("No active configuration found".to_string());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    let outbounds = config
        .get("outbounds")
        .and_then(|o| o.as_array())
        .ok_or_else(|| "No outbounds array found in configuration".to_string())?;

    if let Some(selector) = outbounds
        .iter()
        .find(|o| o.get("type").and_then(|t| t.as_str()) == Some("selector"))
    {
        if let Some(default_tag) = selector.get("default").and_then(|d| d.as_str()) {
            if let Some(found) = outbounds
                .iter()
                .find(|o| o.get("tag").and_then(|t| t.as_str()) == Some(default_tag))
            {
                return Ok(found.clone());
            }
        }
    }

    for outbound in outbounds {
        let outbound_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
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

/// Reads the tag the "proxy" selector currently defaults to, if any.
pub fn get_selected_outbound_tag(app: &tauri::AppHandle) -> Option<String> {
    let config_path = get_active_config_path(app).ok()?;
    if !config_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&config_path).ok()?;
    let config: serde_json::Value = serde_json::from_str(&content).ok()?;
    let outbounds = config.get("outbounds")?.as_array()?;
    let selector = outbounds
        .iter()
        .find(|o| o.get("type").and_then(|t| t.as_str()) == Some("selector"))?;
    selector
        .get("default")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string())
}

/// Edits an existing node's connection parameters in-place (used by the node
/// editor). If the active proxy is currently connected, hot-reloads sing-box
/// with the updated config so changes take effect immediately.
#[tauri::command]
pub async fn update_node(
    app: tauri::AppHandle,
    new_outbound: serde_json::Value,
) -> Result<(), String> {
    let config_path = get_active_config_path(&app)?;
    if !config_path.exists() {
        return Err("No active configuration found".to_string());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let mut config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    let target_tag = new_outbound
        .get("tag")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());
    let mut target_idx = None;
    let mut old_tag: Option<String> = None;

    {
        let outbounds = config
            .get_mut("outbounds")
            .and_then(|o| o.as_array_mut())
            .ok_or_else(|| "No outbounds array found in configuration".to_string())?;

        // Prefer an exact tag match
        for (idx, outbound) in outbounds.iter().enumerate() {
            let t = outbound.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if t == "selector" || t == "direct" || t == "block" || t == "dns" {
                continue;
            }
            let tag = outbound.get("tag").and_then(|v| v.as_str());
            if target_tag.as_deref() == tag {
                target_idx = Some(idx);
                old_tag = tag.map(|s| s.to_string());
                break;
            }
        }

        // Fall back to the first customizable outbound (single-node configs)
        if target_idx.is_none() {
            for (idx, outbound) in outbounds.iter().enumerate() {
                let t = outbound.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if t != "selector" && t != "direct" && t != "block" && t != "dns" {
                    target_idx = Some(idx);
                    old_tag = outbound
                        .get("tag")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    break;
                }
            }
        }
    }

    let target_idx =
        target_idx.ok_or_else(|| "No matching outbound found to update".to_string())?;

    {
        let outbounds = config
            .get_mut("outbounds")
            .and_then(|o| o.as_array_mut())
            .unwrap();
        outbounds[target_idx] = new_outbound.clone();
    }

    // If the tag changed, keep the selector's outbound list / default in sync
    if let (Some(old), Some(new)) = (old_tag, new_outbound.get("tag").and_then(|t| t.as_str())) {
        if old != new {
            if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
                for outbound in outbounds.iter_mut() {
                    if outbound.get("type").and_then(|t| t.as_str()) == Some("selector") {
                        if let Some(list) =
                            outbound.get_mut("outbounds").and_then(|o| o.as_array_mut())
                        {
                            for item in list.iter_mut() {
                                if item.as_str() == Some(old.as_str()) {
                                    *item = serde_json::Value::String(new.to_string());
                                }
                            }
                        }
                        if outbound.get("default").and_then(|d| d.as_str()) == Some(old.as_str()) {
                            outbound["default"] = serde_json::Value::String(new.to_string());
                        }
                    }
                }
            }
        }
    }

    let pretty_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config to JSON: {}", e))?;

    // Validate before committing
    let temp_path = get_temp_config_path(&app)?;
    std::fs::write(&temp_path, &pretty_config)
        .map_err(|e| format!("Failed to write temp config: {}", e))?;
    let validation_res = validate_singbox_config(&app, &temp_path).await;
    let _ = std::fs::remove_file(&temp_path);
    validation_res?;

    std::fs::write(&config_path, &pretty_config)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    if let Ok(Some(profile_id)) = get_active_profile_id(&app) {
        if let Ok(profile_path) = get_profile_config_path(&app, &profile_id) {
            let _ = std::fs::write(&profile_path, &pretty_config);
        }
    }

    // Hot-reload if currently connected, preserving whichever node is selected
    let state = app.state::<crate::state::ProxyState>();
    if state.get_status() == crate::state::ConnectionStatus::Connected {
        let selected_tag = get_selected_outbound_tag(&app);
        let _ = crate::commands::proxy::try_reload_proxy_config(&app, &state, selected_tag).await;
    }

    Ok(())
}

/// Deletes the per-profile config file for a given profile ID.
#[tauri::command]
pub fn delete_profile_config(app: tauri::AppHandle, profile_id: String) -> Result<(), String> {
    let path = get_profile_config_path(&app, &profile_id)?;
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete profile config: {}", e))?;
    }
    Ok(())
}

pub fn set_active_profile_id_in_file(app: &tauri::AppHandle, profile_id: &str) -> Result<(), String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    path.push("profiles.json");

    let mut json = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read profiles.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse profiles.json: {}", e))?
    } else {
        serde_json::json!({})
    };

    json["activeProfileId"] = serde_json::Value::String(profile_id.to_string());

    let content = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Failed to serialize profiles.json: {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write profiles.json: {}", e))?;
    Ok(())
}

/// Switches the active config to a different profile by copying its config to the active slot.
/// If the proxy is currently running, it will be hot-reloaded with the new config.
#[tauri::command]
pub async fn switch_profile(
    app: tauri::AppHandle,
    profile_id: String,
    selected_node_tag: Option<String>,
) -> Result<(), String> {
    // Update the activeProfileId in profiles.json immediately so that any subsequent config generation reads the correct profile nodes
    set_active_profile_id_in_file(&app, &profile_id)?;

    let profile_path = get_profile_config_path(&app, &profile_id)?;
    if !profile_path.exists() {
        return Err(format!(
            "Profile config file not found for ID: {}",
            profile_id
        ));
    }

    let config_content = std::fs::read_to_string(&profile_path)
        .map_err(|e| format!("Failed to read profile config: {}", e))?;

    // If a selected node tag is provided, update the selector's default in the config
    let final_config = if let Some(ref tag) = selected_node_tag {
        let mut config: serde_json::Value = serde_json::from_str(&config_content)
            .map_err(|e| format!("Failed to parse profile config: {}", e))?;

        if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
            for outbound in outbounds.iter_mut() {
                if outbound.get("type").and_then(|t| t.as_str()) == Some("selector") {
                    outbound["default"] = serde_json::Value::String(tag.clone());
                }
            }
        }

        serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?
    } else {
        config_content
    };

    let active_path = get_active_config_path(&app)?;
    std::fs::write(&active_path, &final_config)
        .map_err(|e| format!("Failed to write active config: {}", e))?;

    // Hot-reload if currently connected
    let state = app.state::<crate::state::ProxyState>();
    if state.get_status() == crate::state::ConnectionStatus::Connected {
        let _ =
            crate::commands::proxy::try_reload_proxy_config(&app, &state, selected_node_tag).await;
    }

    Ok(())
}

/// Returns the outbound nodes from a specific profile's config file.
#[tauri::command]
pub fn get_profile_outbounds(
    app: tauri::AppHandle,
    profile_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let config_path = get_profile_config_path(&app, &profile_id)?;
    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read profile config: {}", e))?;
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse profile config: {}", e))?;

    let outbounds = config
        .get("outbounds")
        .and_then(|o| o.as_array())
        .ok_or_else(|| "No outbounds array found in profile config".to_string())?;

    let mut nodes = Vec::new();
    for outbound in outbounds {
        let outbound_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
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

pub fn get_active_profile_id(app: &tauri::AppHandle) -> Result<Option<String>, String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    path.push("profiles.json");
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read profiles.json: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse profiles.json: {}", e))?;
    if let Some(val) = json.get("activeProfileId") {
        if let Some(s) = val.as_str() {
            return Ok(Some(s.to_string()));
        }
        if let Some(s) = val.get("value").and_then(|v| v.as_str()) {
            return Ok(Some(s.to_string()));
        }
    }
    Ok(None)
}
