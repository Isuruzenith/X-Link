use tauri::State;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use tauri::Emitter;
use tokio::sync::oneshot;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use std::sync::OnceLock;
use std::path::Path;
use crate::config::generator::TunSettings;

fn get_toggle_lock() -> &'static AsyncMutex<()> {
    static LOCK: OnceLock<AsyncMutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| AsyncMutex::new(()))
}

pub fn load_tun_settings(app: &tauri::AppHandle) -> TunSettings {
    let state = app.state::<crate::state::ProxyState>();
    let s = state.get_settings();
    TunSettings {
        auto_route: s.tun_auto_route,
        auto_redirect: s.tun_auto_redirect,
        strict_route: s.tun_strict_route,
        stack: s.tun_stack,
        mtu: s.tun_mtu,
        endpoint_independent_nat: s.tun_endpoint_independent_nat,
        sniff_enabled: s.sniff_enabled,
        sniff_http: s.sniff_http,
        sniff_tls: s.sniff_tls,
        sniff_quic: s.sniff_quic,
        sniff_override_destination: s.sniff_override_destination,
        dns_mode: s.dns_mode,
        fakeip_range: s.fakeip_range,
        primary_dns: s.primary_dns,
        fallback_dns: s.fallback_dns,
        bypass_lan: s.bypass_lan,
        final_outbound: s.final_outbound,
    }
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TrafficStats {
    pub upload_bytes: u64,
    pub download_bytes: u64,
    pub upload_speed: u64,
    pub download_speed: u64,
    pub active_connections: u32,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStateSerialized {
    pub status: crate::state::ConnectionStatus,
    pub http_port: u16,
    pub socks_port: u16,
    pub mixed_port: u16,
    pub tun_enabled: bool,
    pub uptime: u64,
}

// Global variable to keep track of connection start time
static CONNECTION_START_TIME: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

fn spawn_singbox_sidecar(
    app: &tauri::AppHandle,
    state: &crate::state::ProxyState,
    config_path: &Path,
    session_id: &str,
) -> Result<oneshot::Receiver<Option<i32>>, String> {
    let (mut rx, child) = app
        .shell()
        .sidecar("sing-box")
        .map_err(|e| format!("Failed to initialize sidecar: {}", e))
        .and_then(|s| {
            s.args(["run", "-c", config_path.to_str().unwrap()])
                .env("ENABLE_DEPRECATED_GEOSITE", "true")
                .env("ENABLE_DEPRECATED_GEOIP", "true")
                .spawn()
                .map_err(|e| format!("spawn_failed: {}", e))
        })?;

    {
        let mut process_lock = state.child_process.lock()
            .map_err(|e| format!("state_lock_poisoned: {}", e))?;
        *process_lock = Some(child);
    }

    let app_for_logs = app.clone();
    let (term_tx, term_rx) = oneshot::channel::<Option<i32>>();
    let term_tx = Arc::new(tokio::sync::Mutex::new(Some(term_tx)));
    let session_id_clone = session_id.to_string();

    tokio::spawn(async move {
        let state_for_logs = app_for_logs.state::<crate::state::ProxyState>();
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                    let text = String::from_utf8_lossy(&line).to_string();
                    state_for_logs.push_log(text.clone());
                    let _ = app_for_logs.emit("sing-box-log", text);
                }
                CommandEvent::Terminated(payload) => {
                    let _ = app_for_logs.emit("sing-box-terminated", payload.code);
                    let mut guard = term_tx.lock().await;
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(payload.code);
                    }

                    let is_active = if let Ok(active_id_guard) = state_for_logs.active_session_id.lock() {
                        active_id_guard.as_ref() == Some(&session_id_clone)
                    } else {
                        false
                    };

                    if is_active {
                        state_for_logs.set_status(&app_for_logs, crate::state::ConnectionStatus::Disconnected);
                        *CONNECTION_START_TIME.lock().unwrap() = None;
                        crate::tray::perform_clean_cleanup(&app_for_logs);
                        crate::tray::update_tray(&app_for_logs);
                    } else {
                        state_for_logs.push_log(format!("[System] Orphaned sing-box process (session {}) exited with code {:?}.", session_id_clone, payload.code));
                    }

                    break;
                }
                _ => {}
            }
        }
    });

    Ok(term_rx)
}

async fn wait_for_startup_or_exit(term_rx: &mut oneshot::Receiver<Option<i32>>) -> Result<(), String> {
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    match term_rx.try_recv() {
        Ok(code) => Err(format!("spawn_failed: exited early during startup with code {:?}", code)),
        Err(oneshot::error::TryRecvError::Closed) => {
            Err("spawn_failed: child process terminated unexpectedly during startup".into())
        }
        Err(oneshot::error::TryRecvError::Empty) => Ok(()),
    }
}

async fn probe_via_mixed_proxy(mixed_port: u16) -> Result<(), String> {
    let proxy = reqwest::Proxy::all(format!("http://127.0.0.1:{}", mixed_port))
        .map_err(|e| format!("mixed probe setup failed: {}", e))?;
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("mixed probe client failed: {}", e))?;

    client
        .get("https://www.gstatic.com/generate_204")
        .send()
        .await
        .map_err(|e| format!("mixed probe failed: node/outbound/config did not carry HTTPS traffic ({})", e))?
        .error_for_status()
        .map(|_| ())
        .map_err(|e| format!("mixed probe failed: upstream returned {}", e))
}

async fn probe_via_tun_route() -> Result<(), String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("TUN probe client failed: {}", e))?;

    client
        .get("https://www.gstatic.com/generate_204")
        .send()
        .await
        .map_err(|e| format!("TUN probe failed: OS traffic did not traverse TUN successfully ({})", e))?
        .error_for_status()
        .map(|_| ())
        .map_err(|e| format!("TUN probe failed: upstream returned {}", e))
}

async fn run_startup_health_checks(mixed_port: u16, proxy_mode: &str) -> Result<(), String> {
    probe_via_mixed_proxy(mixed_port).await?;
    if proxy_mode == "tun" {
        probe_via_tun_route().await?;
    }
    Ok(())
}


fn patch_config_for_fallback(config_path: &std::path::Path, attempt: usize) -> Result<(), String> {
    let content = std::fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read active config for fallback patching: {}", e))?;
    let mut config_val: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse active config for fallback patching: {}", e))?;

    match attempt {
        1 => {
            // Attempt 2 (attempt = 1): Firefox uTLS fingerprint
            if let Some(outbounds) = config_val.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
                for outbound in outbounds {
                    let ob_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if ob_type == "vless" || ob_type == "vmess" || ob_type == "trojan" {
                        if let Some(tls) = outbound.get_mut("tls").and_then(|t| t.as_object_mut()) {
                            if let Some(utls) = tls.get_mut("utls").and_then(|u| u.as_object_mut()) {
                                utls.insert("enabled".to_string(), serde_json::Value::Bool(true));
                                utls.insert("fingerprint".to_string(), serde_json::json!("firefox"));
                            }
                        }
                    }
                }
            }
        }
        2 => {
            // Attempt 3 (attempt = 2): Disable uTLS completely (Native Go TLS)
            if let Some(outbounds) = config_val.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
                for outbound in outbounds {
                    let ob_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if ob_type == "vless" || ob_type == "vmess" || ob_type == "trojan" {
                        if let Some(tls) = outbound.get_mut("tls").and_then(|t| t.as_object_mut()) {
                            if let Some(utls) = tls.get_mut("utls").and_then(|u| u.as_object_mut()) {
                                utls.insert("enabled".to_string(), serde_json::Value::Bool(false));
                            }
                        }
                    }
                }
            }
        }
        3 => {
            // Attempt 4 (attempt = 3): DNS Fallback (Switch DNS rules resolving proxy domains to direct query via public bootstrap dns)
            // First, add public DNS servers detoured direct
            if let Some(dns) = config_val.get_mut("dns").and_then(|d| d.as_object_mut()) {
                if let Some(servers) = dns.get_mut("servers").and_then(|s| s.as_array_mut()) {
                    // Check if already contains bootstrap-dns tags
                    if !servers.iter().any(|s| s.get("tag").and_then(|t| t.as_str()) == Some("fallback-bootstrap-1")) {
                        servers.push(serde_json::json!({
                            "tag": "fallback-bootstrap-1",
                            "address": "1.1.1.1",
                            "detour": "direct"
                        }));
                        servers.push(serde_json::json!({
                            "tag": "fallback-bootstrap-2",
                            "address": "8.8.8.8",
                            "detour": "direct"
                        }));
                    }
                }
                
                // Point proxy domain resolution rule to our fallback public DNS instead of local-dns
                if let Some(rules) = dns.get_mut("rules").and_then(|r| r.as_array_mut()) {
                    for rule in rules {
                        // Find rule containing domain list
                        if rule.get("domain").is_some() {
                            rule["server"] = serde_json::json!("fallback-bootstrap-1");
                        }
                    }
                }
            }
        }
        _ => {}
    }

    std::fs::write(config_path, serde_json::to_string_pretty(&config_val).unwrap())
        .map_err(|e| format!("Failed to write fallback config: {}", e))?;
    Ok(())
}

fn stop_startup_child(app: &tauri::AppHandle, state: &crate::state::ProxyState) {
    if let Ok(mut session_lock) = state.active_session_id.lock() {
        *session_lock = None;
    }
    if let Ok(mut process_lock) = state.child_process.lock() {
        if let Some(child) = process_lock.take() {
            let _ = child.kill();
        }
    }
    let _ = crate::os::disable_system_proxy();
    *CONNECTION_START_TIME.lock().unwrap() = None;
    crate::tray::update_tray(app);
}

#[tauri::command]
pub fn get_proxy_status(state: State<'_, crate::state::ProxyState>) -> Result<ProxyStateSerialized, String> {
    let status = state.get_status();
    let uptime = if let Some(start) = *CONNECTION_START_TIME.lock().unwrap() {
        start.elapsed().as_secs()
    } else {
        0
    };

    let settings = state.get_settings();
    let tun_enabled = status == crate::state::ConnectionStatus::Connected && settings.proxy_mode == "tun";

    Ok(ProxyStateSerialized {
        status,
        http_port: settings.http_port,
        socks_port: settings.socks_port,
        mixed_port: settings.mixed_port,
        tun_enabled,
        uptime,
    })
}

#[tauri::command]
pub async fn get_traffic_stats(state: State<'_, crate::state::ProxyState>) -> Result<TrafficStats, String> {
    if !state.is_running() {
        return Ok(TrafficStats {
            upload_bytes: 0,
            download_bytes: 0,
            upload_speed: 0,
            download_speed: 0,
            active_connections: 0,
        });
    }

    Ok(TrafficStats {
        upload_bytes: *state.upload_bytes.lock().map_err(|e| e.to_string())?,
        download_bytes: *state.download_bytes.lock().map_err(|e| e.to_string())?,
        upload_speed: *state.upload_speed.lock().map_err(|e| e.to_string())?,
        download_speed: *state.download_speed.lock().map_err(|e| e.to_string())?,
        active_connections: *state.active_connections.lock().map_err(|e| e.to_string())?,
    })
}

pub fn prepare_and_patch_config(
    app: &tauri::AppHandle,
    state: &crate::state::ProxyState,
    selected_outbound_tag: Option<&str>,
    mode_override: Option<&str>,
) -> Result<(std::path::PathBuf, String, u16, String), String> {
    // Reload settings from disk and update cache
    let settings = state.reload_settings(app)?;

    let dns_address = crate::config::resolve_dns_address(if settings.dns_address.trim().is_empty() { None } else { Some(&settings.dns_address) });
    let wifi_sharing = settings.wifi_sharing;
    let api_port = settings.api_port;
    let api_secret = settings.api_secret.clone();
    let api_cors = settings.api_cors;
    let proxy_mode = match mode_override {
        Some(m) => m.to_string(),
        None => settings.proxy_mode.clone(),
    };

    let listen_address = if wifi_sharing { "0.0.0.0" } else { "127.0.0.1" };
    let config_path = crate::commands::config::get_active_config_path(app)?;

    // Write a default valid config if the file doesn't exist
    if !config_path.exists() {
        let mixed_port = settings.mixed_port;
        let default_config = format!(r#"{{
  "log": {{
    "level": "info",
    "timestamp": true
  }},
  "inbounds": [
    {{
      "type": "mixed",
      "tag": "mixed-in",
      "listen": "127.0.0.1",
      "listen_port": {}
    }}
  ],
  "outbounds": [
    {{ "type": "direct", "tag": "direct" }}
  ],
  "route": {{
    "rules": [
      {{ "ip_is_private": true, "outbound": "direct" }}
    ],
    "final": "direct",
    "auto_detect_interface": true
  }},
  "experimental": {{
    "clash_api": {{
      "external_controller": "127.0.0.1:9090"
    }}
  }}
}}"#, mixed_port);
        std::fs::write(&config_path, default_config)
            .map_err(|e| format!("Failed to write default config: {}", e))?;
    }

    // Read the active profile outbounds from active.json
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read active config: {}", e))?;
    let config_val: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse active config JSON: {}", e))?;

    let outbounds_val = config_val.get("outbounds")
        .and_then(|o| o.as_array())
        .ok_or_else(|| "No outbounds found in active config".to_string())?;

    let all_outbounds: Vec<crate::config::adapters::SingBoxOutbound> = serde_json::from_value(serde_json::Value::Array(outbounds_val.clone()))
        .map_err(|e| format!("Failed to deserialize outbounds: {}", e))?;

    // Filter to only include the actual node outbounds (excluding the selector, direct, and block outbounds)
    let node_outbounds: Vec<crate::config::adapters::SingBoxOutbound> = all_outbounds
        .into_iter()
        .filter(|o| o.outbound_type != "selector" && o.outbound_type != "direct" && o.outbound_type != "block")
        .collect();

    let tun_settings = load_tun_settings(app);
    let mixed_port = settings.mixed_port;

    // Generate a clean sing-box config from scratch
    let generated_config = crate::config::generator::generate_singbox_config(
        mixed_port,
        node_outbounds,
        &proxy_mode,
        &dns_address,
        listen_address,
        &tun_settings,
        selected_outbound_tag,
    )?;

    let mut final_config_val: serde_json::Value = serde_json::from_str(&generated_config)
        .map_err(|e| format!("Generated config is invalid JSON: {}", e))?;

    // Inject Clash API experimental settings
    let mut clash_api = serde_json::json!({
        "external_controller": format!("127.0.0.1:{}", api_port)
    });
    if !api_secret.is_empty() {
        clash_api["secret"] = serde_json::Value::String(api_secret.clone());
    }
    if api_cors {
        clash_api["access_control_allow_origin"] = serde_json::json!(["*"]);
    }
    final_config_val["experimental"] = serde_json::json!({
        "clash_api": clash_api
    });

    // Write the final config to active.json
    std::fs::write(&config_path, serde_json::to_string_pretty(&final_config_val).unwrap())
        .map_err(|e| format!("Failed to write active config: {}", e))?;

    Ok((config_path, proxy_mode, api_port, api_secret))
}


#[allow(dead_code)]
async fn hot_reload_via_api(
    config_path: &std::path::Path,
    api_port: u16,
    api_secret: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let path_str = config_path.to_string_lossy();
    let mut req = client
        .put(format!("http://127.0.0.1:{}/configs", api_port))
        .json(&serde_json::json!({ "path": path_str, "force": false }));
    if !api_secret.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_secret));
    }
    req.send()
        .await
        .map_err(|e| format!("Hot reload API call failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Hot reload rejected by sing-box: {}", e))?;
    Ok(())
}

async fn switch_selector_node_via_api(
    tag: &str,
    api_port: u16,
    api_secret: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let mut req = client
        .put(format!("http://127.0.0.1:{}/proxies/proxy", api_port))
        .json(&serde_json::json!({ "name": tag }));
    if !api_secret.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_secret));
    }
    req.send()
        .await
        .map_err(|e| format!("Selector switch failed: {}", e))?;
    Ok(())
}

/// Switch the active node via the Clash API without restarting sing-box.
/// Falls back to a full config reload if the API call fails or the API is disabled.
#[tauri::command]
pub async fn switch_node_hot(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::ProxyState>,
    tag: String,
) -> Result<(), String> {
    // Update the tracked selected tag
    if let Ok(mut lock) = state.selected_outbound_tag.lock() {
        *lock = Some(tag.clone());
    }

    let settings = state.get_settings();
    let api_port = settings.api_port;
    let api_secret = settings.api_secret.clone();
    let api_enabled = settings.api_enabled;

    if state.get_status() == crate::state::ConnectionStatus::Connected && api_enabled {
        let _ = prepare_and_patch_config(&app, &state, Some(&tag), None);
        match switch_selector_node_via_api(&tag, api_port, &api_secret).await {
            Ok(_) => {
                state.push_log(format!("[System] Hot-switched to node: {}", tag));
                return Ok(());
            }
            Err(e) => {
                state.push_log(format!("[System] Hot switch failed ({}), falling back to reload.", e));
            }
        }
    }

    try_reload_proxy_config(&app, &state, Some(tag)).await
}

pub fn try_reload_proxy_config<'a>(
    app: &'a tauri::AppHandle,
    _state: &'a crate::state::ProxyState,
    selected_outbound_tag: Option<String>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
    let app_handle = app.clone();
    let state_handle = app.state::<crate::state::ProxyState>();
    let tag = selected_outbound_tag;

    Box::pin(async move {
        let _ = prepare_and_patch_config(&app_handle, &state_handle, tag.as_deref(), None)?;

        // Stop the proxy
        let _ = toggle_proxy(app_handle.clone(), state_handle.clone(), false, None).await;

        // Give it a brief moment to shut down gracefully
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Start the proxy
        let res = toggle_proxy(app_handle, state_handle, true, tag).await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    })
}

#[tauri::command]
pub async fn reload_active_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::ProxyState>,
    selected_outbound_tag: Option<String>,
) -> Result<(), String> {
    try_reload_proxy_config(&app, &state, selected_outbound_tag).await
}

#[tauri::command]
pub async fn toggle_proxy(
    app: tauri::AppHandle,
    state: State<'_, crate::state::ProxyState>,
    start: bool,
    selected_outbound_tag: Option<String>,
) -> Result<String, String> {
    let _guard = get_toggle_lock().lock().await;

    // ── STOP PATH ────────────────────────────────────────────────────────────
    if !start {
        state.set_status(&app, crate::state::ConnectionStatus::Disconnected);
        {
            let mut process_lock = state.child_process.lock()
                .map_err(|e| format!("state_lock_poisoned: {}", e))?;
            if let Some(child) = process_lock.take() {
                let _ = child.kill();
            }
        }
        
        // Forcefully kill any sing-box processes system-wide to guarantee clean state
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/IM", "sing-box.exe", "/T"])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("pkill")
                .arg("-f")
                .arg("sing-box")
                .output();
        }

        // Clear system proxy
        let _ = crate::os::disable_system_proxy();
        
        // Clear global state
        if let Ok(mut session_lock) = state.active_session_id.lock() {
            *session_lock = None;
        }
        *CONNECTION_START_TIME.lock().unwrap() = None;

        // Reset bandwidth metrics
        if let Ok(mut up_s) = state.upload_speed.lock() { *up_s = 0; }
        if let Ok(mut down_s) = state.download_speed.lock() { *down_s = 0; }
        if let Ok(mut conn) = state.active_connections.lock() { *conn = 0; }

        // Sync native system tray
        crate::tray::update_tray(&app);
        
        return Ok("stopped".into());
    }

    // ── START PATH ───────────────────────────────────────────────────────────
    // If already connected, only a node-switch (different selected_outbound_tag)
    // warrants action -- otherwise this is a no-op.
    if start && state.get_status() == crate::state::ConnectionStatus::Connected {
        let current_tag = crate::commands::config::get_selected_outbound_tag(&app);
        if selected_outbound_tag.is_some() && selected_outbound_tag != current_tag {
            let result = try_reload_proxy_config(&app, &state, selected_outbound_tag.clone()).await;
            match result {
                Ok(_) => {
                    state.push_log("[System] Dynamic node switch succeeded.".to_string());
                    return Ok("started".into());
                }
                Err(e) => {
                    state.push_log(format!("[System] Dynamic node switch failed: {}. Falling back to clean restart...", e));
                }
            }
        } else {
            return Ok("started".into());
        }
    }

    state.set_status(&app, crate::state::ConnectionStatus::Connecting);
    crate::tray::update_tray(&app);
    // Forcefully kill any existing sing-box processes system-wide before starting
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "sing-box.exe", "/T"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
        // Brief delay to let sockets fully release
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("pkill")
            .arg("-f")
            .arg("sing-box")
            .output();
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    // Clean up any stale state in state variable
    let mut killed_stale = false;
    {
        let mut process_lock = state.child_process.lock()
            .map_err(|e| format!("state_lock_poisoned: {}", e))?;
        if let Some(child) = process_lock.take() {
            let _ = child.kill();
            killed_stale = true;
        }
    }

    if killed_stale {
        let _ = crate::os::disable_system_proxy();
        *CONNECTION_START_TIME.lock().unwrap() = None;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    let (config_path, resolved_proxy_mode, api_port, api_secret) = match prepare_and_patch_config(&app, &state, selected_outbound_tag.as_deref(), None) {
        Ok(res) => res,
        Err(e) => {
            state.set_status(&app, crate::state::ConnectionStatus::Disconnected);
            crate::tray::update_tray(&app);
            return Err(e);
        }
    };

    let mut current_mode = resolved_proxy_mode.clone();
    let mut attempt = 0;
    let mut last_error = String::new();
    let mut connected_successfully = false;

    // Helper helper to apply TUN compatibility in-place
    let apply_tun_compatibility_in_place = |path: &std::path::Path| -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read active config for TUN compatibility: {}", e))?;
        let mut config_val = serde_json::from_str::<serde_json::Value>(&content)
            .map_err(|e| format!("Failed to parse active config for TUN compatibility: {}", e))?;
        crate::config::apply_tun_compatibility_profile(&mut config_val);
        std::fs::write(path, serde_json::to_string_pretty(&config_val).unwrap())
            .map_err(|e| format!("Failed to write TUN compatibility fallback config: {}", e))?;
        Ok(())
    };

    while attempt < 5 {
        if attempt > 0 {
            // Apply fallback patch for current attempt
            if attempt <= 3 {
                if let Err(e) = patch_config_for_fallback(&config_path, attempt) {
                    state.push_log(format!("[System] Fallback patching failed: {}", e));
                }
            } else if attempt == 4 && current_mode == "tun" {
                state.push_log("[System] Retrying with Windows TUN compatibility profile...".to_string());
                if let Err(e) = apply_tun_compatibility_in_place(&config_path) {
                    state.push_log(format!("[System] TUN compatibility patching failed: {}", e));
                }
            }
        }

        let session_id = uuid::Uuid::new_v4().to_string();
        {
            if let Ok(mut session_lock) = state.active_session_id.lock() {
                *session_lock = Some(session_id.clone());
            }
        }

        #[cfg(target_os = "windows")]
        if current_mode == "tun" {
            crate::copy_wintun_dll_to_sidecar_dir(&app);
        }

        let mut term_rx = match spawn_singbox_sidecar(&app, &state, &config_path, &session_id) {
            Ok(rx) => rx,
            Err(e) => {
                last_error = e.clone();
                state.push_log(format!("[System] Attempt {} failed to spawn: {}", attempt + 1, e));
                attempt += 1;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }
        };

        if let Err(e) = wait_for_startup_or_exit(&mut term_rx).await {
            stop_startup_child(&app, &state);
            last_error = e.clone();
            state.push_log(format!("[System] Attempt {} process exited early: {}", attempt + 1, e));
            attempt += 1;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            continue;
        }

        let mixed_port = state.get_settings().mixed_port;
        match run_startup_health_checks(mixed_port, &current_mode).await {
            Ok(_) => {
                state.push_log("[System] Connection verified successfully.".to_string());
                connected_successfully = true;
                break;
            }
            Err(probe_err) => {
                stop_startup_child(&app, &state);
                last_error = probe_err.clone();
                state.push_log(format!("[System] Attempt {} probe failed: {}", attempt + 1, probe_err));

                attempt += 1;
                if attempt < 5 {
                    match attempt {
                        1 => state.push_log("[System] Retrying with Firefox TLS fingerprint fallback...".to_string()),
                        2 => state.push_log("[System] Retrying with Native TLS (uTLS disabled) fallback...".to_string()),
                        3 => state.push_log("[System] Retrying with Public DNS bootstrap fallback...".to_string()),
                        _ => {}
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    if !connected_successfully {
        if resolved_proxy_mode == "tun" {
            state.push_log("[System] TUN mode failed completely after all fallback attempts. Rolling back to System Proxy mode...".to_string());
            
            // Switch mode dynamically
            current_mode = "system".to_string();
            
            // Re-generate configuration for system proxy mode
            let (retry_config_path, _, _, _) = prepare_and_patch_config(&app, &state, selected_outbound_tag.as_deref(), Some("system"))?;
            
            let session_id = uuid::Uuid::new_v4().to_string();
            {
                if let Ok(mut session_lock) = state.active_session_id.lock() {
                    *session_lock = Some(session_id.clone());
                }
            }
            
            let mut term_rx = spawn_singbox_sidecar(&app, &state, &retry_config_path, &session_id)
                .map_err(|e| format!("System proxy fallback spawn failed: {}", e))?;
                
            wait_for_startup_or_exit(&mut term_rx).await
                .map_err(|e| format!("System proxy fallback process exited: {}", e))?;
                
            let mixed_port = state.get_settings().mixed_port;
            run_startup_health_checks(mixed_port, &current_mode).await
                .map_err(|e| format!("System proxy fallback health check failed: {}", e))?;
                
            state.push_log("[System] Connected successfully in System Proxy mode.".to_string());
        } else {
            state.set_status(&app, crate::state::ConnectionStatus::Disconnected);
            crate::tray::update_tray(&app);
            return Err(format!("Connection failed after 5 fallback attempts: {}", last_error));
        }
    }

    // Spawn background traffic monitoring task
    let app_handle = app.clone();
    let api_secret_clone = api_secret.clone();
    tokio::spawn(async move {
        let state_clone = app_handle.state::<crate::state::ProxyState>();
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/connections", api_port);
        let mut prev_upload = 0u64;
        let mut prev_download = 0u64;
        let mut consecutive_failures = 0;

        loop {
            if !state_clone.is_running() {
                break;
            }

            let mut req = client.get(&url);
            if !api_secret_clone.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", api_secret_clone));
            }

            match req.send().await {
                Ok(resp) => {
                    #[derive(serde::Deserialize)]
                    struct ConnectionsResponse {
                        #[serde(rename = "uploadTotal")]
                        upload_total: u64,
                        #[serde(rename = "downloadTotal")]
                        download_total: u64,
                        connections: Vec<serde_json::Value>,
                    }

                    if let Ok(data) = resp.json::<ConnectionsResponse>().await {
                        consecutive_failures = 0;
                        let mut up_speed = 0u64;
                        let mut down_speed = 0u64;

                        if prev_upload > 0 && data.upload_total >= prev_upload {
                            up_speed = data.upload_total - prev_upload;
                        }
                        if prev_download > 0 && data.download_total >= prev_download {
                            down_speed = data.download_total - prev_download;
                        }

                        prev_upload = data.upload_total;
                        prev_download = data.download_total;

                        if let Ok(mut up_b) = state_clone.upload_bytes.lock() {
                            *up_b = data.upload_total;
                        }
                        if let Ok(mut down_b) = state_clone.download_bytes.lock() {
                            *down_b = data.download_total;
                        }
                        if let Ok(mut up_s) = state_clone.upload_speed.lock() {
                            *up_s = up_speed;
                        }
                        if let Ok(mut down_s) = state_clone.download_speed.lock() {
                            *down_s = down_speed;
                        }
                        if let Ok(mut conn) = state_clone.active_connections.lock() {
                            *conn = data.connections.len() as u32;
                        }

                        // Update tray tooltip with live speed
                        crate::tray::update_tray_tooltip(&app_handle);
                    } else {
                        consecutive_failures += 1;
                    }
                }
                Err(_) => {
                    consecutive_failures += 1;
                }
            }

            if consecutive_failures > 5 {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        if let Ok(mut up_s) = state_clone.upload_speed.lock() { *up_s = 0; };
        if let Ok(mut down_s) = state_clone.download_speed.lock() { *down_s = 0; };
        if let Ok(mut conn) = state_clone.active_connections.lock() { *conn = 0; };

        // Reset tooltip on disconnect
        crate::tray::update_tray_tooltip(&app_handle);
    });

    // Enable system proxy if the final connection mode is 'system' (either by default or as a fallback)
    if current_mode == "system" {
        let mixed_port = state.get_settings().mixed_port;
        if let Err(e) = crate::os::enable_system_proxy("127.0.0.1", mixed_port) {
            crate::tray::perform_clean_cleanup(&app);
            state.set_status(&app, crate::state::ConnectionStatus::Disconnected);
            crate::tray::update_tray(&app);
            return Err(e);
        }
    }

    // Set connection global states
    state.set_status(&app, crate::state::ConnectionStatus::Connected);
    *CONNECTION_START_TIME.lock().unwrap() = Some(std::time::Instant::now());

    // Sync native system tray
    crate::tray::update_tray(&app);

    Ok("started".into())
}

#[tauri::command]
pub async fn get_active_connections(state: State<'_, crate::state::ProxyState>) -> Result<serde_json::Value, String> {
    if !state.is_running() {
        return Ok(serde_json::json!({ "connections": [] }));
    }
    let api_port = state.get_settings().api_port;
    let api_secret = state.get_settings().api_secret.clone();
    let url = format!("http://127.0.0.1:{}/connections", api_port);

    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if !api_secret.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_secret));
    }

    match req.send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                Ok(json)
            } else {
                Err("Failed to parse connections JSON".to_string())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn close_connection(id: String, state: State<'_, crate::state::ProxyState>) -> Result<(), String> {
    if !state.is_running() {
        return Ok(());
    }
    let api_port = state.get_settings().api_port;
    let api_secret = state.get_settings().api_secret.clone();
    let url = format!("http://127.0.0.1:{}/connections/{}", api_port, id);

    let client = reqwest::Client::new();
    let mut req = client.delete(&url);
    if !api_secret.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_secret));
    }

    match req.send().await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn close_all_connections(state: State<'_, crate::state::ProxyState>) -> Result<(), String> {
    if !state.is_running() {
        return Ok(());
    }
    let api_port = state.get_settings().api_port;
    let api_secret = state.get_settings().api_secret.clone();
    let url = format!("http://127.0.0.1:{}/connections", api_port);

    let client = reqwest::Client::new();
    let mut req = client.delete(&url);
    if !api_secret.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_secret));
    }

    match req.send().await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
