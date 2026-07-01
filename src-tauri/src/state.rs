use std::sync::Mutex;
use std::collections::VecDeque;
use tauri_plugin_shell::process::CommandChild;
use serde::{Serialize, Deserialize};
use tauri::Manager;
use tauri::Emitter;

/// Maximum lines retained in the Rust-side log buffer.
const LOG_BUFFER_RUST_MAX: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    pub proxy_mode: String,
    pub close_to_tray: bool,
    pub autostart: bool,
    pub http_port: u16,
    pub socks_port: u16,
    pub mixed_port: u16,
    pub wifi_sharing: bool,
    pub tun_auto_route: bool,
    pub tun_auto_redirect: bool,
    pub tun_strict_route: bool,
    pub tun_stack: String,
    pub tun_mtu: u32,
    pub tun_endpoint_independent_nat: bool,
    pub sniff_enabled: bool,
    pub sniff_http: bool,
    pub sniff_tls: bool,
    pub sniff_quic: bool,
    pub sniff_override_destination: bool,
    pub api_enabled: bool,
    pub api_port: u16,
    pub api_secret: String,
    pub api_cors: bool,
    pub primary_dns: String,
    pub fallback_dns: String,
    pub direct_dns: String,
    pub dns_strategy: String,
    pub dns_mode: String,
    pub fakeip_range: String,
    pub fakeip_filter: String,
    pub dns_leak_protection: bool,
    pub dns_caching: bool,
    pub final_outbound: String,
    pub bypass_lan: bool,
    pub dns_address: String,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            proxy_mode: "tun".to_string(),
            close_to_tray: true,
            autostart: false,
            http_port: 7890,
            socks_port: 7891,
            mixed_port: 7892,
            wifi_sharing: false,
            tun_auto_route: true,
            tun_auto_redirect: false,
            tun_strict_route: true,
            tun_stack: "gvisor".to_string(),
            tun_mtu: 1500,
            tun_endpoint_independent_nat: false,
            sniff_enabled: true,
            sniff_http: true,
            sniff_tls: true,
            sniff_quic: true,
            sniff_override_destination: false,
            api_enabled: false,
            api_port: 9090,
            api_secret: "".to_string(),
            api_cors: true,
            primary_dns: "https://1.1.1.1/dns-query".to_string(),
            fallback_dns: "https://8.8.8.8/dns-query".to_string(),
            direct_dns: "223.5.5.5".to_string(),
            dns_strategy: "prefer_ipv4".to_string(),
            dns_mode: "fakeip".to_string(),
            fakeip_range: "198.18.0.0/15".to_string(),
            fakeip_filter: "geosite:private".to_string(),
            dns_leak_protection: true,
            dns_caching: true,
            final_outbound: "proxy".to_string(),
            bypass_lan: true,
            dns_address: "".to_string(),
        }
    }
}

pub struct ProxyState {
    /// The live sing-box child process, if running
    pub child_process: Mutex<Option<CommandChild>>,
    /// Active session ID to prevent race conditions during restart/hot-swaps
    pub active_session_id: Mutex<Option<String>>,
    /// Connection status
    pub connection_status: Mutex<ConnectionStatus>,
    /// Rust-side log ring buffer
    pub log_buffer: Mutex<VecDeque<String>>,
    /// Cached User Settings
    pub settings: Mutex<UserSettings>,
    /// Live bandwidth statistics
    pub upload_bytes: Mutex<u64>,
    pub download_bytes: Mutex<u64>,
    pub upload_speed: Mutex<u64>,
    pub download_speed: Mutex<u64>,
    pub active_connections: Mutex<u32>,
    /// Cached sing-box version string (pre-fetched at startup)
    pub singbox_version: Mutex<Option<String>>,
    /// Currently selected outbound node tag
    pub selected_outbound_tag: Mutex<Option<String>>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            child_process: Mutex::new(None),
            active_session_id: Mutex::new(None),
            connection_status: Mutex::new(ConnectionStatus::Disconnected),
            log_buffer: Mutex::new(VecDeque::with_capacity(LOG_BUFFER_RUST_MAX)),
            settings: Mutex::new(UserSettings::default()),
            upload_bytes: Mutex::new(0),
            download_bytes: Mutex::new(0),
            upload_speed: Mutex::new(0),
            download_speed: Mutex::new(0),
            active_connections: Mutex::new(0),
            singbox_version: Mutex::new(None),
            selected_outbound_tag: Mutex::new(None),
        }
    }

    /// Load or reload settings from the settings.json file.
    pub fn reload_settings(&self, app: &tauri::AppHandle) -> Result<UserSettings, String> {
        let mut settings = UserSettings::default();
        if let Ok(mut path) = app.path().app_data_dir() {
            path.push("settings.json");
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(val) = serde_json::from_str::<UserSettings>(&content) {
                        settings = val;
                    } else if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        // Fallback: manually parse fields if partial settings json exists (e.g. from javascript store plugin)
                        let mut default_s = UserSettings::default();
                        if let Some(v) = val.get("proxyMode").and_then(|v| v.as_str()) { default_s.proxy_mode = v.to_string(); }
                        if let Some(v) = val.get("closeToTray").and_then(|v| v.as_bool()) { default_s.close_to_tray = v; }
                        if let Some(v) = val.get("autostart").and_then(|v| v.as_bool()) { default_s.autostart = v; }
                        if let Some(v) = val.get("httpPort").and_then(|v| v.as_u64()) { default_s.http_port = v as u16; }
                        if let Some(v) = val.get("socksPort").and_then(|v| v.as_u64()) { default_s.socks_port = v as u16; }
                        if let Some(v) = val.get("mixedPort").and_then(|v| v.as_u64()) { default_s.mixed_port = v as u16; }
                        if let Some(v) = val.get("wifiSharing").and_then(|v| v.as_bool()) { default_s.wifi_sharing = v; }
                        if let Some(v) = val.get("tunAutoRoute").and_then(|v| v.as_bool()) { default_s.tun_auto_route = v; }
                        if let Some(v) = val.get("tunAutoRedirect").and_then(|v| v.as_bool()) { default_s.tun_auto_redirect = v; }
                        if let Some(v) = val.get("tunStrictRoute").and_then(|v| v.as_bool()) { default_s.tun_strict_route = v; }
                        if let Some(v) = val.get("tunStack").and_then(|v| v.as_str()) { default_s.tun_stack = v.to_string(); }
                        if let Some(v) = val.get("tunMtu").and_then(|v| v.as_u64()) { default_s.tun_mtu = v as u32; }
                        if let Some(v) = val.get("tunEndpointIndependentNat").and_then(|v| v.as_bool()) { default_s.tun_endpoint_independent_nat = v; }
                        if let Some(v) = val.get("sniffEnabled").and_then(|v| v.as_bool()) { default_s.sniff_enabled = v; }
                        if let Some(v) = val.get("sniffHttp").and_then(|v| v.as_bool()) { default_s.sniff_http = v; }
                        if let Some(v) = val.get("sniffTls").and_then(|v| v.as_bool()) { default_s.sniff_tls = v; }
                        if let Some(v) = val.get("sniffQuic").and_then(|v| v.as_bool()) { default_s.sniff_quic = v; }
                        if let Some(v) = val.get("sniffOverrideDestination").and_then(|v| v.as_bool()) { default_s.sniff_override_destination = v; }
                        if let Some(v) = val.get("apiEnabled").and_then(|v| v.as_bool()) { default_s.api_enabled = v; }
                        if let Some(v) = val.get("apiPort").and_then(|v| v.as_u64()) { default_s.api_port = v as u16; }
                        if let Some(v) = val.get("apiSecret").and_then(|v| v.as_str()) { default_s.api_secret = v.to_string(); }
                        if let Some(v) = val.get("apiCors").and_then(|v| v.as_bool()) { default_s.api_cors = v; }
                        if let Some(v) = val.get("primaryDns").and_then(|v| v.as_str()) { default_s.primary_dns = v.to_string(); }
                        if let Some(v) = val.get("fallbackDns").and_then(|v| v.as_str()) { default_s.fallback_dns = v.to_string(); }
                        if let Some(v) = val.get("directDns").and_then(|v| v.as_str()) { default_s.direct_dns = v.to_string(); }
                        if let Some(v) = val.get("dnsStrategy").and_then(|v| v.as_str()) { default_s.dns_strategy = v.to_string(); }
                        if let Some(v) = val.get("dnsMode").and_then(|v| v.as_str()) { default_s.dns_mode = v.to_string(); }
                        if let Some(v) = val.get("fakeipRange").and_then(|v| v.as_str()) { default_s.fakeip_range = v.to_string(); }
                        if let Some(v) = val.get("fakeipFilter").and_then(|v| v.as_str()) { default_s.fakeip_filter = v.to_string(); }
                        if let Some(v) = val.get("dnsLeakProtection").and_then(|v| v.as_bool()) { default_s.dns_leak_protection = v; }
                        if let Some(v) = val.get("dnsCaching").and_then(|v| v.as_bool()) { default_s.dns_caching = v; }
                        if let Some(v) = val.get("finalOutbound").and_then(|v| v.as_str()) { default_s.final_outbound = v.to_string(); }
                        if let Some(v) = val.get("bypassLan").and_then(|v| v.as_bool()) { default_s.bypass_lan = v; }
                        if let Some(v) = val.get("dnsAddress").and_then(|v| v.as_str()) { default_s.dns_address = v.to_string(); }
                        settings = default_s;
                    }
                }
            }
        }
        if let Ok(mut guard) = self.settings.lock() {
            *guard = settings.clone();
        }
        Ok(settings)
    }

    pub fn get_settings(&self) -> UserSettings {
        if let Ok(guard) = self.settings.lock() {
            guard.clone()
        } else {
            UserSettings::default()
        }
    }

    pub fn get_status(&self) -> ConnectionStatus {
        if let Ok(g) = self.connection_status.lock() {
            *g
        } else {
            ConnectionStatus::Disconnected
        }
    }

    pub fn set_status(&self, app: &tauri::AppHandle, status: ConnectionStatus) {
        if let Ok(mut g) = self.connection_status.lock() {
            *g = status;
        }
        let status_str = match status {
            ConnectionStatus::Disconnected => "disconnected",
            ConnectionStatus::Connecting => "connecting",
            ConnectionStatus::Connected => "connected",
        };
        let _ = app.emit("proxy-status-changed", status_str);
    }

    pub fn is_running(&self) -> bool {
        if let Ok(g) = self.child_process.lock() {
            g.is_some()
        } else {
            false
        }
    }

    /// Append a log line to the Rust ring buffer, evicting oldest if at capacity.
    pub fn push_log(&self, line: String) {
        if let Ok(mut buf) = self.log_buffer.lock() {
            if buf.len() >= LOG_BUFFER_RUST_MAX {
                buf.pop_front();
            }
            buf.push_back(line);
        }
    }

    /// Drain and return all buffered log lines (clears the buffer).
    pub fn drain_logs(&self) -> Vec<String> {
        if let Ok(mut buf) = self.log_buffer.lock() {
            buf.drain(..).collect()
        } else {
            Vec::new()
        }
    }
}
