use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_shell::process::CommandChild;

/// Network routing mode that determines DNS strategy, route rules, and safety constraints.
///
/// - `SniZeroData`: Sri Lanka 0-data ISP package mode. Forces remote DNS detour through proxy,
///   suppresses user `direct` rules and LAN-direct rules to prevent unbilled data leaks,
///   forces QUIC block, and forces `proxy` as the final outbound.
/// - `Rule`: Split tunneling with custom user rules (default).
/// - `Global`: All traffic through proxy.
/// - `Direct`: Full bypass mode for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoutingMode {
    SniZeroData,
    #[default]
    Rule,
    Global,
    Direct,
}

/// Maximum lines retained in the Rust-side log buffer and diagnostic ring buffers.
pub const LOG_BUFFER_RUST_MAX: usize = 500;

pub enum SidecarChild {
    Shell(CommandChild),
    #[allow(dead_code)]
    Tokio(Box<tokio::process::Child>),
}

impl SidecarChild {
    pub fn pid(&self) -> Option<u32> {
        match self {
            Self::Shell(child) => Some(child.pid()),
            Self::Tokio(child) => child.id(),
        }
    }

    pub fn kill(self) -> Result<(), String> {
        match self {
            Self::Shell(child) => child.kill().map_err(|e| e.to_string()),
            Self::Tokio(mut child) => {
                // Dropping stdin delivers EOF to the root/companion supervisor, which cleanly sends SIGTERM to sing-box
                if let Some(stdin) = child.stdin.take() {
                    drop(stdin);
                } else {
                    let _ = child.start_kill();
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DnsLogEntry {
    pub timestamp: u64,
    pub domain: String,
    pub qtype: String,
    pub answers: Vec<String>,
    pub elapsed_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TraceEntry {
    pub timestamp: u64,
    pub inbound: String,
    pub outbound: String,
    pub rule: Option<String>,
    pub destination: String,
}

#[derive(Debug, Clone)]
pub struct ConnectionByteSnapshot {
    pub upload: u64,
    pub download: u64,
    pub recorded_at: std::time::Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionMetadata {
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(rename = "sourceIP", alias = "sourceIp", default)]
    pub source_ip: String,
    #[serde(default)]
    pub source_port: String,
    #[serde(rename = "destinationIP", alias = "destinationIp", default)]
    pub destination_ip: String,
    #[serde(default)]
    pub destination_port: String,
    #[serde(default)]
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveConnection {
    pub id: String,
    #[serde(default)]
    pub metadata: ConnectionMetadata,
    #[serde(default)]
    pub upload: u64,
    #[serde(default)]
    pub download: u64,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub chains: Vec<String>,
    #[serde(default)]
    pub rule: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up_speed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down_speed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionsPayload {
    pub download_total: u64,
    pub upload_total: u64,
    pub total_connections: usize,
    pub connections: Vec<ActiveConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyGroupInfo {
    pub group: String,
    pub winning_tag: String,
    pub delay: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticTracesPayload {
    pub dns_queries: Vec<DnsLogEntry>,
    pub traces: Vec<TraceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarSession {
    pub pid: u32,
    pub session_id: String,
    pub proxy_mode: String,
    pub api_port: u16,
    pub started_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

/// Generates a cryptographically secure random token (32-character hexadecimal string, 128-bit CSPRNG entropy).
pub fn generate_secure_secret() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
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
    pub geo_lookup_enabled: bool,
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
    pub use_separate_ports: bool,
    pub multiplex: bool,
    /// Reject QUIC and UDP/443 early to allow TCP fallback on restricted networks.
    pub block_quic: bool,
    /// Network routing mode (zero-data, rule-based, global, or direct).
    pub routing_mode: RoutingMode,
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
            multiplex: false,
            block_quic: false,
            sniff_enabled: true,
            sniff_http: true,
            sniff_tls: true,
            sniff_quic: true,
            sniff_override_destination: false,
            api_enabled: false,
            api_port: 9090,
            api_secret: generate_secure_secret(),
            api_cors: false,
            geo_lookup_enabled: false,
            primary_dns: "https://1.1.1.1/dns-query".to_string(),
            fallback_dns: "https://8.8.8.8/dns-query".to_string(),
            direct_dns: "".to_string(),
            dns_strategy: "prefer_ipv4".to_string(),
            dns_mode: "fakeip".to_string(),
            fakeip_range: "198.18.0.0/15".to_string(),
            fakeip_filter: "geosite:private".to_string(),
            dns_leak_protection: true,
            dns_caching: true,
            final_outbound: "proxy".to_string(),
            bypass_lan: true,
            dns_address: "".to_string(),
            use_separate_ports: false,
            routing_mode: RoutingMode::Rule,
        }
    }
}

pub struct ProxyState {
    /// The live sing-box child process, if running
    pub child_process: Mutex<Option<SidecarChild>>,
    /// Active session ID to prevent race conditions during restart/hot-swaps
    pub active_session_id: Mutex<Option<String>>,
    /// Connection status
    pub connection_status: Mutex<ConnectionStatus>,
    /// Effective mode of the current session, which may differ from saved settings.
    pub effective_proxy_mode: Mutex<Option<String>>,
    /// Rust-side log ring buffer
    pub log_buffer: Mutex<VecDeque<String>>,
    /// Ring buffer for parsed DNS exchange records (capped at 500)
    pub dns_buffer: Mutex<VecDeque<DnsLogEntry>>,
    /// Ring buffer for parsed routing rule match traces (capped at 500)
    pub trace_buffer: Mutex<VecDeque<TraceEntry>>,
    /// Persistent HTTP client for Clash API queries with 800ms read timeout
    pub clash_client: reqwest::Client,
    /// Previous byte snapshots per connection ID for rate calculation
    pub prev_connection_bytes: Mutex<HashMap<String, ConnectionByteSnapshot>>,
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
        let clash_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(800))
            .pool_max_idle_per_host(4)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            child_process: Mutex::new(None),
            active_session_id: Mutex::new(None),
            connection_status: Mutex::new(ConnectionStatus::Disconnected),
            effective_proxy_mode: Mutex::new(None),
            log_buffer: Mutex::new(VecDeque::with_capacity(LOG_BUFFER_RUST_MAX)),
            dns_buffer: Mutex::new(VecDeque::with_capacity(LOG_BUFFER_RUST_MAX)),
            trace_buffer: Mutex::new(VecDeque::with_capacity(LOG_BUFFER_RUST_MAX)),
            clash_client,
            prev_connection_bytes: Mutex::new(HashMap::new()),
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
                        if let Some(v) = val.get("proxyMode").and_then(|v| v.as_str()) {
                            default_s.proxy_mode = v.to_string();
                        }
                        if let Some(v) = val.get("closeToTray").and_then(|v| v.as_bool()) {
                            default_s.close_to_tray = v;
                        }
                        if let Some(v) = val.get("autostart").and_then(|v| v.as_bool()) {
                            default_s.autostart = v;
                        }
                        if let Some(v) = val.get("httpPort").and_then(|v| v.as_u64()) {
                            default_s.http_port = v as u16;
                        }
                        if let Some(v) = val.get("socksPort").and_then(|v| v.as_u64()) {
                            default_s.socks_port = v as u16;
                        }
                        if let Some(v) = val.get("mixedPort").and_then(|v| v.as_u64()) {
                            default_s.mixed_port = v as u16;
                        }
                        if let Some(v) = val.get("wifiSharing").and_then(|v| v.as_bool()) {
                            default_s.wifi_sharing = v;
                        }
                        if let Some(v) = val.get("tunAutoRoute").and_then(|v| v.as_bool()) {
                            default_s.tun_auto_route = v;
                        }
                        if let Some(v) = val.get("tunAutoRedirect").and_then(|v| v.as_bool()) {
                            default_s.tun_auto_redirect = v;
                        }
                        if let Some(v) = val.get("tunStrictRoute").and_then(|v| v.as_bool()) {
                            default_s.tun_strict_route = v;
                        }
                        if let Some(v) = val.get("tunStack").and_then(|v| v.as_str()) {
                            default_s.tun_stack = v.to_string();
                        }
                        if let Some(v) = val.get("tunMtu").and_then(|v| v.as_u64()) {
                            default_s.tun_mtu = v as u32;
                        }
                        if let Some(v) = val
                            .get("tunEndpointIndependentNat")
                            .and_then(|v| v.as_bool())
                        {
                            default_s.tun_endpoint_independent_nat = v;
                        }
                        if let Some(v) = val.get("sniffEnabled").and_then(|v| v.as_bool()) {
                            default_s.sniff_enabled = v;
                        }
                        if let Some(v) = val.get("sniffHttp").and_then(|v| v.as_bool()) {
                            default_s.sniff_http = v;
                        }
                        if let Some(v) = val.get("sniffTls").and_then(|v| v.as_bool()) {
                            default_s.sniff_tls = v;
                        }
                        if let Some(v) = val.get("sniffQuic").and_then(|v| v.as_bool()) {
                            default_s.sniff_quic = v;
                        }
                        if let Some(v) = val
                            .get("sniffOverrideDestination")
                            .and_then(|v| v.as_bool())
                        {
                            default_s.sniff_override_destination = v;
                        }
                        if let Some(v) = val.get("apiEnabled").and_then(|v| v.as_bool()) {
                            default_s.api_enabled = v;
                        }
                        if let Some(v) = val.get("apiPort").and_then(|v| v.as_u64()) {
                            default_s.api_port = v as u16;
                        }
                        if let Some(v) = val.get("apiSecret").and_then(|v| v.as_str()) {
                            default_s.api_secret = v.to_string();
                        }
                        if let Some(v) = val.get("apiCors").and_then(|v| v.as_bool()) {
                            default_s.api_cors = v;
                        }
                        if let Some(v) = val.get("geoLookupEnabled").and_then(|v| v.as_bool()) {
                            default_s.geo_lookup_enabled = v;
                        }
                        if let Some(v) = val.get("primaryDns").and_then(|v| v.as_str()) {
                            default_s.primary_dns = v.to_string();
                        }
                        if let Some(v) = val.get("fallbackDns").and_then(|v| v.as_str()) {
                            default_s.fallback_dns = v.to_string();
                        }
                        if let Some(v) = val.get("directDns").and_then(|v| v.as_str()) {
                            default_s.direct_dns = v.to_string();
                        }
                        if let Some(v) = val.get("dnsStrategy").and_then(|v| v.as_str()) {
                            default_s.dns_strategy = v.to_string();
                        }
                        if let Some(v) = val.get("dnsMode").and_then(|v| v.as_str()) {
                            default_s.dns_mode = v.to_string();
                        }
                        if let Some(v) = val.get("fakeipRange").and_then(|v| v.as_str()) {
                            default_s.fakeip_range = v.to_string();
                        }
                        if let Some(v) = val.get("fakeipFilter").and_then(|v| v.as_str()) {
                            default_s.fakeip_filter = v.to_string();
                        }
                        if let Some(v) = val.get("dnsLeakProtection").and_then(|v| v.as_bool()) {
                            default_s.dns_leak_protection = v;
                        }
                        if let Some(v) = val.get("dnsCaching").and_then(|v| v.as_bool()) {
                            default_s.dns_caching = v;
                        }
                        if let Some(v) = val.get("finalOutbound").and_then(|v| v.as_str()) {
                            default_s.final_outbound = v.to_string();
                        }
                        if let Some(v) = val.get("bypassLan").and_then(|v| v.as_bool()) {
                            default_s.bypass_lan = v;
                        }
                        if let Some(v) = val.get("dnsAddress").and_then(|v| v.as_str()) {
                            default_s.dns_address = v.to_string();
                        }
                        if let Some(v) = val.get("useSeparatePorts").and_then(|v| v.as_bool()) {
                            default_s.use_separate_ports = v;
                        }
                        if let Some(v) = val.get("multiplex").and_then(|v| v.as_bool()) {
                            default_s.multiplex = v;
                        }
                        if let Some(v) = val.get("blockQuic").and_then(|v| v.as_bool()) {
                            default_s.block_quic = v;
                        }
                        if let Some(v) = val.get("routingMode").and_then(|v| v.as_str()) {
                            if let Ok(rm) = serde_json::from_value::<RoutingMode>(
                                serde_json::Value::String(v.to_string()),
                            ) {
                                default_s.routing_mode = rm;
                            }
                        }
                        settings = default_s;
                    }

                    // Safe migration: ensure existing stored settings with empty/missing secrets get a secure token
                    if settings.api_secret.trim().is_empty() {
                        settings.api_secret = generate_secure_secret();
                        if let Ok(mut json_val) =
                            serde_json::from_str::<serde_json::Value>(&content)
                        {
                            if let Some(obj) = json_val.as_object_mut() {
                                obj.insert(
                                    "apiSecret".to_string(),
                                    serde_json::Value::String(settings.api_secret.clone()),
                                );
                                let _ = std::fs::write(
                                    &path,
                                    serde_json::to_string_pretty(&json_val).unwrap_or_default(),
                                );
                            }
                        }
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
        if status == ConnectionStatus::Disconnected {
            if let Ok(mut mode) = self.effective_proxy_mode.lock() {
                *mode = None;
            }
        }
        let status_str = match status {
            ConnectionStatus::Disconnected => "disconnected",
            ConnectionStatus::Connecting => "connecting",
            ConnectionStatus::Connected => "connected",
        };
        let _ = app.emit("proxy-status-changed", status_str);
        let _ = app.emit("tunnel-status-changed", status_str);
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

    /// Return a non-destructive clone snapshot of up to `max_lines` recent log lines from the buffer.
    pub fn get_buffered_logs_snapshot(&self, max_lines: usize) -> Vec<String> {
        if let Ok(buf) = self.log_buffer.lock() {
            let start = if buf.len() > max_lines {
                buf.len() - max_lines
            } else {
                0
            };
            buf.iter().skip(start).cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Append a parsed DNS log entry to the ring buffer (capped at 500).
    pub fn push_dns_log(&self, entry: DnsLogEntry) {
        if let Ok(mut buf) = self.dns_buffer.lock() {
            if buf.len() >= LOG_BUFFER_RUST_MAX {
                buf.pop_front();
            }
            buf.push_back(entry);
        }
    }

    /// Append a parsed routing trace entry to the ring buffer (capped at 500).
    pub fn push_trace(&self, entry: TraceEntry) {
        if let Ok(mut buf) = self.trace_buffer.lock() {
            if buf.len() >= LOG_BUFFER_RUST_MAX {
                buf.pop_front();
            }
            buf.push_back(entry);
        }
    }

    /// Return a non-destructive snapshot of buffered DNS queries and routing traces.
    pub fn get_diagnostic_traces_snapshot(&self) -> DiagnosticTracesPayload {
        let dns_queries = self
            .dns_buffer
            .lock()
            .map(|b| b.iter().cloned().collect())
            .unwrap_or_default();
        let traces = self
            .trace_buffer
            .lock()
            .map(|b| b.iter().cloned().collect())
            .unwrap_or_default();
        DiagnosticTracesPayload {
            dns_queries,
            traces,
        }
    }
}

/// Metadata representation of a profile within the persisted store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreProfile {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_url: Option<String>,
    #[serde(default)]
    pub updated_at: u64,
    #[serde(default)]
    pub node_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_node_tag: Option<String>,
}

/// Strongly-typed single source of truth for persisted client state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedStore {
    pub profiles: Vec<StoreProfile>,
    pub active_profile_id: Option<String>,
    pub selected_node_id: Option<String>,
    pub settings: UserSettings,
}

impl PersistedStore {
    /// Constructs a clean fallback store containing a default manual profile.
    pub fn default_with_profile() -> Self {
        let default_profile_id = "default-profile".to_string();
        Self {
            profiles: vec![StoreProfile {
                id: default_profile_id.clone(),
                name: "Default Profile".into(),
                r#type: Some("manual".into()),
                subscription_url: None,
                updated_at: 0,
                node_count: 0,
                selected_node_tag: None,
            }],
            active_profile_id: Some(default_profile_id),
            selected_node_id: None,
            settings: UserSettings::default(),
        }
    }

    /// Atomically writes state to `<data_dir>/xlink_store.json.tmp` and renames to `<data_dir>/xlink_store.json`.
    #[allow(dead_code)]
    pub fn save_to_dir<P: AsRef<std::path::Path>>(&self, data_dir: P) -> Result<(), String> {
        let dir = data_dir.as_ref();
        let _ = std::fs::create_dir_all(dir);
        let tmp_path = dir.join("xlink_store.json.tmp");
        let final_path = dir.join("xlink_store.json");

        let serialized = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize store: {}", e))?;

        std::fs::write(&tmp_path, serialized)
            .map_err(|e| format!("Failed to write tmp store file: {}", e))?;

        #[cfg(windows)]
        if final_path.exists() {
            use windows::core::PCWSTR;
            use windows::Win32::Storage::FileSystem::ReplaceFileW;
            use windows::Win32::Storage::FileSystem::REPLACE_FILE_FLAGS;
            use std::os::windows::ffi::OsStrExt;

            let target: Vec<u16> = final_path.as_os_str().encode_wide().chain(Some(0)).collect();
            let replacement: Vec<u16> = tmp_path.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe {
                ReplaceFileW(
                    PCWSTR(target.as_ptr()),
                    PCWSTR(replacement.as_ptr()),
                    None,
                    REPLACE_FILE_FLAGS(0),
                    None,
                    None,
                )
            }.map_err(|e| format!("Failed to atomically replace store file: {}", e))?;
        } else {
            std::fs::rename(&tmp_path, &final_path)
                .map_err(|e| format!("Failed to commit store file: {}", e))?;
        }
        #[cfg(not(windows))]
        std::fs::rename(&tmp_path, &final_path)
            .map_err(|e| format!("Failed to commit store file: {}", e))?;

        Ok(())
    }

    /// Load a standalone store; reject legacy files that are still the application's live state.
    pub fn load_from_dir<P: AsRef<std::path::Path>>(data_dir: P) -> Result<Self, String> {
        let dir = data_dir.as_ref();
        let store_path = dir.join("xlink_store.json");
        // Legacy files are still live; an earlier xlink_store.json snapshot cannot supersede them.
        if dir.join("profiles.json").exists() || dir.join("settings.json").exists() {
            return Err("Legacy profiles.json or settings.json exists; migration is required before xlink_store.json can be used".to_string());
        }
        if store_path.exists() {
            let raw = std::fs::read_to_string(&store_path)
                .map_err(|e| format!("Failed to read xlink_store.json: {e}"))?;
            return serde_json::from_str::<PersistedStore>(&raw)
                .map_err(|e| format!("Invalid xlink_store.json: {e}"));
        }
        Ok(Self::default_with_profile())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persisted_store_default_clean_boot() {
        let store = PersistedStore::default_with_profile();
        assert_eq!(store.profiles.len(), 1);
        assert_eq!(store.profiles[0].id, "default-profile");
        assert_eq!(store.active_profile_id.as_deref(), Some("default-profile"));
        assert_eq!(store.settings.routing_mode, RoutingMode::Rule);
    }

    #[test]
    fn test_persisted_store_atomic_save_and_reload() {
        let temp_dir = std::env::temp_dir().join(format!("xlink-store-test-{}", uuid::Uuid::new_v4()));
        let mut store = PersistedStore::default_with_profile();
        store.profiles[0].name = "Test Profile Updated".to_string();
        store.settings.routing_mode = RoutingMode::SniZeroData;

        store.save_to_dir(&temp_dir).expect("save_to_dir must succeed");

        let final_file = temp_dir.join("xlink_store.json");
        let tmp_file = temp_dir.join("xlink_store.json.tmp");
        assert!(final_file.exists());
        assert!(!tmp_file.exists(), "temp file must be renamed away");

        let loaded = PersistedStore::load_from_dir(&temp_dir).expect("saved store should load");
        assert_eq!(loaded.profiles[0].name, "Test Profile Updated");
        assert_eq!(loaded.settings.routing_mode, RoutingMode::SniZeroData);

        store.profiles[0].name = "Saved Again".to_string();
        store.save_to_dir(&temp_dir).expect("second save must atomically replace existing file");
        assert_eq!(PersistedStore::load_from_dir(&temp_dir).expect("second save should load").profiles[0].name, "Saved Again");
        assert!(!tmp_file.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_persisted_store_corrupted_file_fails_closed() {
        let temp_dir = std::env::temp_dir().join(format!("xlink-corrupt-test-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let final_file = temp_dir.join("xlink_store.json");
        std::fs::write(&final_file, b"invalid corrupt json {{{").unwrap();

        let error = PersistedStore::load_from_dir(&temp_dir).expect_err("corrupt store must not be replaced by defaults");
        assert!(error.contains("Invalid xlink_store.json"));
        assert_eq!(std::fs::read(&final_file).unwrap(), b"invalid corrupt json {{{");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_persisted_store_refuses_to_fabricate_legacy_profiles() {
        let temp_dir = std::env::temp_dir().join(format!("xlink-legacy-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let legacy_path = temp_dir.join("profiles.json");
        std::fs::write(&legacy_path, br#"{"profiles":[]}"#).unwrap();

        let error = PersistedStore::load_from_dir(&temp_dir).expect_err("legacy profiles need migration");
        assert!(error.contains("migration is required"));
        assert!(!temp_dir.join("xlink_store.json").exists());
        assert_eq!(std::fs::read(&legacy_path).unwrap(), br#"{"profiles":[]}"#);

        let mut stale = PersistedStore::default_with_profile();
        stale.profiles[0].name = "Stale snapshot".to_string();
        stale.save_to_dir(&temp_dir).unwrap();
        let error = PersistedStore::load_from_dir(&temp_dir).expect_err("live legacy data must take precedence over a stale snapshot");
        assert!(error.contains("migration is required"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
