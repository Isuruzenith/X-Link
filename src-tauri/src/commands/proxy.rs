use tauri::State;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use tauri::Emitter;
use tokio::sync::oneshot;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use std::sync::OnceLock;
use crate::config::generator::TunSettings;

fn get_toggle_lock() -> &'static AsyncMutex<()> {
    static LOCK: OnceLock<AsyncMutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| AsyncMutex::new(()))
}

/// Read TUN and sniffing settings from the user's settings.json file.
/// Falls back to safe defaults if the file is missing or any field is absent.
pub fn load_tun_settings(app: &tauri::AppHandle) -> TunSettings {
    let mut ts = TunSettings::default();
    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("settings.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(v) = val.get("tunAutoRoute").and_then(|v| v.as_bool()) { ts.auto_route = v; }
                    if let Some(v) = val.get("tunAutoRedirect").and_then(|v| v.as_bool()) { ts.auto_redirect = v; }
                    if let Some(v) = val.get("tunStrictRoute").and_then(|v| v.as_bool()) { ts.strict_route = v; }
                    if let Some(v) = val.get("tunStack").and_then(|v| v.as_str()) { ts.stack = v.to_string(); }
                    if let Some(v) = val.get("tunMtu").and_then(|v| v.as_u64()) { ts.mtu = v as u32; }
                    if let Some(v) = val.get("tunEndpointIndependentNat").and_then(|v| v.as_bool()) { ts.endpoint_independent_nat = v; }
                    if let Some(v) = val.get("sniffEnabled").and_then(|v| v.as_bool()) { ts.sniff_enabled = v; }
                    if let Some(v) = val.get("sniffHttp").and_then(|v| v.as_bool()) { ts.sniff_http = v; }
                    if let Some(v) = val.get("sniffTls").and_then(|v| v.as_bool()) { ts.sniff_tls = v; }
                    if let Some(v) = val.get("sniffQuic").and_then(|v| v.as_bool()) { ts.sniff_quic = v; }
                    if let Some(v) = val.get("sniffOverrideDestination").and_then(|v| v.as_bool()) { ts.sniff_override_destination = v; }
                }
            }
        }
    }
    ts
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
pub fn get_proxy_status(app: tauri::AppHandle, state: State<'_, crate::state::ProxyState>) -> Result<ProxyStateSerialized, String> {
    let status = state.get_status();
    let active_profile = ACTIVE_PROFILE_ID.lock().unwrap().clone();
    let uptime = if let Some(start) = *CONNECTION_START_TIME.lock().unwrap() {
        start.elapsed().as_secs()
    } else {
        0
    };

    // Derive tun_enabled dynamically: true when connected AND proxyMode is 'tun'
    let tun_enabled = if status == crate::state::ConnectionStatus::Connected {
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
        mode == "tun"
    } else {
        false
    };

    Ok(ProxyStateSerialized {
        status,
        active_profile_id: active_profile,
        http_port: *state.http_port.lock().unwrap(),
        socks_port: *state.socks_port.lock().unwrap(),
        mixed_port: *state.mixed_port.lock().unwrap(),
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
    profile_id: &str,
) -> Result<(std::path::PathBuf, String, u16, String), String> {
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

    let (dns_address, wifi_sharing, api_port, api_secret, api_cors, proxy_mode) = {
        let mut dns: Option<String> = None;
        let mut wifi = false;
        let mut api_port = 9090u16;
        let mut api_secret = "".to_string();
        let mut api_cors = true;
        let mut mode = "system".to_string();
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
                        if let Some(m) = val.get("proxyMode").and_then(|v| v.as_str()) {
                            mode = m.to_string();
                        }
                    }
                }
            }
        }
        let resolved_dns = crate::config::resolve_dns_address(dns.as_deref());
        (resolved_dns, wifi, api_port, api_secret, api_cors, mode)
    };

    let listen_address = if wifi_sharing { "0.0.0.0" } else { "127.0.0.1" };
    let config_path = crate::config::get_config_path(app, profile_id)?;

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
      {{ "geoip": "private", "outbound": "direct" }}
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
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let mut config_val = serde_json::from_str::<serde_json::Value>(&content)
        .map_err(|e| format!("Failed to parse config JSON: {}", e))?;

    // Self-heal: strip deprecated block/dns special outbounds from old config files
    if let Some(outbounds) = config_val.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        outbounds.retain(|o| {
            let t = o.get("type").and_then(|v| v.as_str()).unwrap_or("");
            t != "block" && t != "dns"
        });
    }

    let mut server_hosts = Vec::new();
    if let Some(outbounds) = config_val.get("outbounds").and_then(|o| o.as_array()) {
        for outbound in outbounds {
            if let Some(server) = outbound.get("server").and_then(|s| s.as_str()) {
                server_hosts.push(server.to_string());
            }
        }
    }
    let server_ips = crate::config::resolve_server_ips(&server_hosts);
    let route_exclude_addresses = crate::config::build_route_exclude_addresses(&dns_address, &server_ips);

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
        let tun_settings = load_tun_settings(app);
        let mut tun_inbound = serde_json::json!({
            "type": "tun",
            "tag": "tun-in",
            "interface_name": "X-Link",
            "address": [
                "172.19.0.1/30"
            ],
            "auto_route": tun_settings.auto_route,
            "strict_route": tun_settings.strict_route,
            "stack": tun_settings.stack,
            "route_exclude_address": route_exclude_addresses,
            "sniff": tun_settings.sniff_enabled,
            "sniff_override_destination": tun_settings.sniff_override_destination
        });
        if tun_settings.mtu != 0 {
            tun_inbound["mtu"] = serde_json::json!(tun_settings.mtu);
        }
        if tun_settings.auto_redirect {
            tun_inbound["auto_redirect"] = serde_json::json!(true);
        }
        if tun_settings.endpoint_independent_nat {
            tun_inbound["endpoint_independent_nat"] = serde_json::json!(true);
        }
        inbounds.push(tun_inbound);
    }

    config_val["inbounds"] = serde_json::to_value(inbounds).unwrap();

    let mut dns_rules = vec![
        serde_json::json!({ "outbound": "direct", "server": "local-dns" })
    ];

    let mut server_domains = Vec::new();
    for host in &server_hosts {
        if host.parse::<std::net::IpAddr>().is_err() {
            let trimmed = host.trim();
            if !trimmed.is_empty() {
                server_domains.push(trimmed.to_string());
            }
        }
    }

    if !server_domains.is_empty() {
        dns_rules.insert(0, serde_json::json!({
            "domain": server_domains,
            "server": "local-dns"
        }));
    }

    // Patch DNS section with dynamic dnsAddress from settings
    if proxy_mode == "tun" {
        config_val["dns"] = serde_json::json!({
            "servers": [
                { "tag": "proxy-dns", "address": "tcp://1.1.1.1", "detour": "proxy" },
                { "tag": "local-dns", "address": dns_address, "detour": "direct" }
            ],
            "rules": dns_rules,
            "strategy": "ipv4_only"
        });
    } else {
        config_val["dns"] = serde_json::json!({
            "servers": [
                { "tag": "local-dns", "address": dns_address, "detour": "direct" }
            ],
            "rules": dns_rules
        });
    }

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

    std::fs::write(&config_path, serde_json::to_string_pretty(&config_val).unwrap())
        .map_err(|e| format!("Failed to write patched config: {}", e))?;

    Ok((config_path, proxy_mode, api_port, api_secret))
}

pub fn try_reload_proxy_config<'a>(
    app: &'a tauri::AppHandle,
    state: &'a crate::state::ProxyState,
    profile_id: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
    let app_handle = app.clone();
    let state_handle = app.state::<crate::state::ProxyState>();
    let pid = profile_id.to_string();

    Box::pin(async move {
        let _ = prepare_and_patch_config(&app_handle, &state_handle, &pid)?;
        
        // Stop the proxy
        let _ = toggle_proxy(app_handle.clone(), state_handle.clone(), false, "".to_string()).await;
        
        // Give it a brief moment to shut down gracefully
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        
        // Start the proxy
        let res = toggle_proxy(app_handle, state_handle, true, pid).await;
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    })
}

#[tauri::command]
pub async fn reload_active_profile(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::ProxyState>,
    profile_id: String,
) -> Result<(), String> {
    try_reload_proxy_config(&app, &state, &profile_id).await
}

#[tauri::command]
pub async fn toggle_proxy(
    app: tauri::AppHandle,
    state: State<'_, crate::state::ProxyState>,
    start: bool,
    profile_id: String,
) -> Result<String, String> {
    let _guard = get_toggle_lock().lock().await;

    // ── STOP PATH ────────────────────────────────────────────────────────────
    if !start {
        state.set_status(crate::state::ConnectionStatus::Disconnected);
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
        if let Ok(mut session_lock) = state.active_session_id.lock() {
            *session_lock = None;
        }
        *ACTIVE_PROFILE_ID.lock().unwrap() = None;
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
    if start && state.get_status() == crate::state::ConnectionStatus::Connected {
        if let Some(active_id) = get_active_profile_id() {
            if active_id != profile_id {
                let result = try_reload_proxy_config(&app, &state, &profile_id).await;
                match result {
                    Ok(_) => {
                        state.push_log(format!("[System] Dynamic config hot-reload to profile \"{}\" succeeded.", profile_id));
                        return Ok("started".into());
                    }
                    Err(e) => {
                        state.push_log(format!("[System] Dynamic config reload failed: {}. Falling back to clean restart...", e));
                    }
                }
            } else {
                return Ok("started".into());
            }
        }
    }

    state.set_status(crate::state::ConnectionStatus::Connecting);
    crate::tray::update_tray(&app);
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

    let (config_path, _proxy_mode, api_port, api_secret) = match prepare_and_patch_config(&app, &state, &profile_id) {
        Ok(res) => res,
        Err(e) => {
            state.set_status(crate::state::ConnectionStatus::Disconnected);
            crate::tray::update_tray(&app);
            return Err(e);
        }
    };

    let session_id = uuid::Uuid::new_v4().to_string();
    {
        if let Ok(mut session_lock) = state.active_session_id.lock() {
            *session_lock = Some(session_id.clone());
        }
    }

    let (mut rx, child) = match app
        .shell()
        .sidecar("sing-box")
        .map_err(|e| format!("Failed to initialize sidecar: {}", e))
        .and_then(|s| {
            s.args(["run", "-c", config_path.to_str().unwrap()])
                .env("ENABLE_DEPRECATED_GEOSITE", "true")
                .spawn()
                .map_err(|e| format!("spawn_failed: {}", e))
        })
    {
        Ok(res) => res,
        Err(e) => {
            state.set_status(crate::state::ConnectionStatus::Disconnected);
            if let Ok(mut session_lock) = state.active_session_id.lock() {
                *session_lock = None;
            }
            crate::tray::update_tray(&app);
            return Err(e);
        }
    };

    {
        let mut process_lock = state.child_process.lock()
            .map_err(|e| format!("state_lock_poisoned: {}", e))?;
        *process_lock = Some(child);
    }

    // ── Start async log pipe (writes to Rust buffer + emits to frontend) ────
    let app_for_logs = app.clone();
    let (term_tx, mut term_rx) = oneshot::channel::<Option<i32>>();
    let term_tx = Arc::new(tokio::sync::Mutex::new(Some(term_tx)));
    let session_id_clone = session_id.clone();

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
                    
                    // Verify if this process termination corresponds to the active session
                    let is_active = if let Ok(active_id_guard) = state_for_logs.active_session_id.lock() {
                        active_id_guard.as_ref() == Some(&session_id_clone)
                    } else {
                        false
                    };

                    if is_active {
                        // Clear global state & restore OS system proxy
                        state_for_logs.set_status(crate::state::ConnectionStatus::Disconnected);
                        *ACTIVE_PROFILE_ID.lock().unwrap() = None;
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

    // ── Health-check gate: Sleep for 1500ms to allow boot-up, then check process vitality ──
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    // Check if the sidecar process exited prematurely
    match term_rx.try_recv() {
        Ok(code) => {
            state.set_status(crate::state::ConnectionStatus::Disconnected);
            if let Ok(mut session_lock) = state.active_session_id.lock() {
                *session_lock = None;
            }
            crate::tray::update_tray(&app);
            let mut lock = state.child_process.lock().map_err(|e| e.to_string())?;
            lock.take();
            return Err(format!(
                "spawn_failed: exited early during startup with code {:?}", code
            ));
        }
        Err(oneshot::error::TryRecvError::Closed) => {
            state.set_status(crate::state::ConnectionStatus::Disconnected);
            if let Ok(mut session_lock) = state.active_session_id.lock() {
                *session_lock = None;
            }
            crate::tray::update_tray(&app);
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
        if let Err(e) = crate::os::enable_system_proxy("127.0.0.1", mixed_port) {
            crate::tray::perform_clean_cleanup(&app);
            state.set_status(crate::state::ConnectionStatus::Disconnected);
            crate::tray::update_tray(&app);
            return Err(e);
        }
    }

    // Set connection global states
    state.set_status(crate::state::ConnectionStatus::Connected);
    *ACTIVE_PROFILE_ID.lock().unwrap() = Some(profile_id);
    *CONNECTION_START_TIME.lock().unwrap() = Some(std::time::Instant::now());

    // Sync native system tray
    crate::tray::update_tray(&app);

    Ok("started".into())
}
