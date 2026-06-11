use tauri::State;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use tauri::Emitter;
use tokio::sync::oneshot;
use std::sync::Arc;

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
    pub status: String,
    pub active_profile_id: Option<String>,
    pub http_port: u16,
    pub socks_port: u16,
    pub mixed_port: u16,
    pub tun_enabled: bool,
    pub uptime: u64,
}

// Global variable to keep track of active profile and connection start time
static ACTIVE_PROFILE_ID: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
pub fn get_active_profile_id() -> Option<String> {
    ACTIVE_PROFILE_ID.lock().unwrap().clone()
}
static CONNECTION_START_TIME: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

#[tauri::command]
pub fn get_proxy_status(state: State<'_, crate::state::ProxyState>) -> Result<ProxyStateSerialized, String> {
    let is_running = state.is_running();
    let status = if is_running { "connected" } else { "idle" };
    let active_profile = ACTIVE_PROFILE_ID.lock().unwrap().clone();
    let uptime = if let Some(start) = *CONNECTION_START_TIME.lock().unwrap() {
        start.elapsed().as_secs()
    } else {
        0
    };

    Ok(ProxyStateSerialized {
        status: status.to_string(),
        active_profile_id: active_profile,
        http_port: *state.http_port.lock().unwrap(),
        socks_port: *state.socks_port.lock().unwrap(),
        mixed_port: *state.mixed_port.lock().unwrap(),
        tun_enabled: false,
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

#[tauri::command]
pub async fn toggle_proxy(
    app: tauri::AppHandle,
    state: State<'_, crate::state::ProxyState>,
    start: bool,
    profile_id: String,
) -> Result<String, String> {
    // ── STOP PATH ────────────────────────────────────────────────────────────
    if !start {
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
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/IM", "sing-box.exe", "/T"])
                .output();
        }

        // Clear system proxy
        let _ = crate::os::disable_system_proxy();
        
        // Clear global state
        *ACTIVE_PROFILE_ID.lock().unwrap() = None;
        *CONNECTION_START_TIME.lock().unwrap() = None;

        // Reset bandwidth metrics
        if let Ok(mut up_s) = state.upload_speed.lock() { *up_s = 0; }
        if let Ok(mut down_s) = state.download_speed.lock() { *down_s = 0; }
        if let Ok(mut conn) = state.active_connections.lock() { *conn = 0; }

        // Sync native system tray menu
        crate::tray::update_tray_menu(&app);
        
        return Ok("stopped".into());
    }

    // ── START PATH ───────────────────────────────────────────────────────────
    // Forcefully kill any existing sing-box processes system-wide before starting
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "sing-box.exe", "/T"])
            .output();
        // Brief delay to let sockets fully release
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
        *ACTIVE_PROFILE_ID.lock().unwrap() = None;
        *CONNECTION_START_TIME.lock().unwrap() = None;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    // Synchronize Rust state ports with user's settings.json configuration dynamically
    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("settings.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(p) = val.get("mixedPort").and_then(|v| v.as_u64()) {
                        *state.mixed_port.lock().unwrap() = p as u16;
                    }
                    if let Some(p) = val.get("httpPort").and_then(|v| v.as_u64()) {
                        *state.http_port.lock().unwrap() = p as u16;
                    }
                    if let Some(p) = val.get("socksPort").and_then(|v| v.as_u64()) {
                        *state.socks_port.lock().unwrap() = p as u16;
                    }
                }
            }
        }
    }

    let (dns_address, sni_host, wifi_sharing, api_port, api_secret, api_cors) = {
        let mut dns: Option<String> = None;
        let mut sni = "aka.ms".to_string();
        let mut wifi = false;
        let mut api_port = 9090u16;
        let mut api_secret = "".to_string();
        let mut api_cors = true;
        if let Ok(mut path) = app.path().app_data_dir() {
            path.push("settings.json");
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(d) = val.get("dnsAddress").and_then(|v| v.as_str()) {
                            if !d.trim().is_empty() {
                                dns = Some(d.to_string());
                            }
                        }
                        if let Some(s) = val.get("sniHost").and_then(|v| v.as_str()) {
                            sni = s.to_string();
                        }
                        if let Some(w) = val.get("wifiSharing").and_then(|v| v.as_bool()) {
                            wifi = w;
                        }
                        if let Some(p) = val.get("apiPort").and_then(|v| v.as_u64()) {
                            api_port = p as u16;
                        }
                        if let Some(s) = val.get("apiSecret").and_then(|v| v.as_str()) {
                            api_secret = s.to_string();
                        }
                        if let Some(c) = val.get("apiCors").and_then(|v| v.as_bool()) {
                            api_cors = c;
                        }
                    }
                }
            }
        }
        let resolved_dns = crate::config::resolve_dns_address(dns.as_deref());
        (resolved_dns, sni, wifi, api_port, api_secret, api_cors)
    };

    let listen_address = if wifi_sharing { "0.0.0.0" } else { "127.0.0.1" };
    let route_exclude_addresses = crate::config::build_route_exclude_addresses(&dns_address);

    let config_path = crate::config::get_config_path(&app, &profile_id)?;
    
    // Write a default valid config if the file doesn't exist
    if !config_path.exists() {
        let mixed_port = *state.mixed_port.lock().unwrap();
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
      {{ "geoip": "private", "action": "direct" }}
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

    // Dynamically patch inbounds array to match user's active settings.json (ports & proxyMode)
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(mut config_val) = serde_json::from_str::<serde_json::Value>(&content) {
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

                let mut inbounds = vec![
                    serde_json::json!({
                        "type": "mixed",
                        "tag": "mixed-in",
                        "listen": listen_address,
                        "listen_port": mixed_port
                    })
                ];

                if proxy_mode == "tun" {
                    inbounds.push(serde_json::json!({
                        "type": "tun",
                        "tag": "tun-in",
                        "interface_name": "X-Link",
                        "address": [
                            "172.19.0.1/30",
                            "fdfe:dcba:9876::1/126"
                        ],
                        "auto_route": true,
                        "strict_route": true,
                        "stack": "gvisor",
                        "route_exclude_address": route_exclude_addresses,
                        "sniff": true,
                        "sniff_override_destination": true
                    }));
                }

                config_val["inbounds"] = serde_json::to_value(inbounds).unwrap();

                // Patch DNS section with dynamic dnsAddress from settings
                config_val["dns"] = serde_json::json!({
                    "servers": [
                        { "tag": "local-dns", "address": dns_address, "detour": "direct" }
                    ],
                    "rules": [
                        { "outbound": "any", "server": "local-dns" }
                    ]
                });

                // Patch routing rules to keep public traffic on the proxy
                let route_rules = crate::config::build_route_rules();
                config_val["route"] = serde_json::json!({
                    "rules": route_rules,
                    "final": "proxy",
                    "auto_detect_interface": true
                });

                // Patch experimental clash_api configuration
                let mut clash_api = serde_json::json!({
                    "external_controller": format!("127.0.0.1:{}", api_port)
                });
                if !api_secret.is_empty() {
                    clash_api["secret"] = serde_json::Value::String(api_secret.clone());
                }
                if api_cors {
                    clash_api["access_control_allow_origin"] = serde_json::json!(["*"]);
                }
                config_val["experimental"] = serde_json::json!({
                    "clash_api": clash_api
                });

                // Patch outbounds with dynamic SNI host from settings
                if let Some(outbounds) = config_val.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
                    for outbound in outbounds.iter_mut() {
                        if let Some(outbound_obj) = outbound.as_object_mut() {
                            if !sni_host.is_empty() {
                                if let Some(tls_val) = outbound_obj.get_mut("tls") {
                                    if let Some(tls_obj) = tls_val.as_object_mut() {
                                        if tls_obj.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false) {
                                            tls_obj.insert("server_name".to_string(), serde_json::Value::String(sni_host.clone()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let _ = std::fs::write(&config_path, serde_json::to_string_pretty(&config_val).unwrap());
            }
        }
    }

    let (mut rx, child) = app
        .shell()
        .sidecar("sing-box")
        .map_err(|e| format!("Failed to initialize sidecar: {}", e))?
        .args(["run", "-c", config_path.to_str().unwrap()])
        .env("ENABLE_DEPRECATED_GEOSITE", "true")
        .spawn()
        .map_err(|e| format!("spawn_failed: {}", e))?;

    {
        let mut process_lock = state.child_process.lock()
            .map_err(|e| format!("state_lock_poisoned: {}", e))?;
        *process_lock = Some(child);
    }

    // ── Start async log pipe (writes to Rust buffer + emits to frontend) ────
    let app_for_logs = app.clone();
    let (term_tx, mut term_rx) = oneshot::channel::<Option<i32>>();
    let term_tx = Arc::new(tokio::sync::Mutex::new(Some(term_tx)));

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
                    
                    // Clear global state & restore OS system proxy
                    *ACTIVE_PROFILE_ID.lock().unwrap() = None;
                    *CONNECTION_START_TIME.lock().unwrap() = None;
                    crate::tray::perform_clean_cleanup(&app_for_logs);
                    crate::tray::update_tray_menu(&app_for_logs);
                    
                    break;
                }
                _ => {}
            }
        }
    });

    // ── Health-check gate: Sleep for 1500ms to allow boot-up, then check process vitality ──
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    // Check if the sidecar process exited prematurely
    match term_rx.try_recv() {
        Ok(code) => {
            let state = app.state::<crate::state::ProxyState>();
            let mut lock = state.child_process.lock().map_err(|e| e.to_string())?;
            lock.take();
            return Err(format!(
                "spawn_failed: exited early during startup with code {:?}", code
            ));
        }
        Err(oneshot::error::TryRecvError::Closed) => {
            let state = app.state::<crate::state::ProxyState>();
            let mut lock = state.child_process.lock().map_err(|e| e.to_string())?;
            lock.take();
            return Err("spawn_failed: child process terminated unexpectedly during startup".into());
        }
        Err(oneshot::error::TryRecvError::Empty) => {
            // Process is healthy and still running successfully!
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
    });

    // Enable system proxy ONLY if proxyMode is 'system'
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

    if proxy_mode == "system" {
        let mixed_port = *state.mixed_port.lock().unwrap();
        crate::os::enable_system_proxy("127.0.0.1", mixed_port)?;
    }

    // Set connection global states
    *ACTIVE_PROFILE_ID.lock().unwrap() = Some(profile_id);
    *CONNECTION_START_TIME.lock().unwrap() = Some(std::time::Instant::now());

    // Sync native system tray menu
    crate::tray::update_tray_menu(&app);

    Ok("started".into())
}
