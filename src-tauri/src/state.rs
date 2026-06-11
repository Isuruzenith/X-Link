use std::sync::Mutex;
use std::collections::VecDeque;
use tauri_plugin_shell::process::CommandChild;

/// Maximum lines retained in the Rust-side log buffer.
const LOG_BUFFER_RUST_MAX: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

pub struct ProxyState {
    /// The live sing-box child process, if running
    pub child_process: Mutex<Option<CommandChild>>,
    /// Whether system proxy is currently set in the OS
    pub system_proxy_active: Mutex<bool>,
    /// Connection status
    pub connection_status: Mutex<ConnectionStatus>,
    /// Rust-side log ring buffer
    pub log_buffer: Mutex<VecDeque<String>>,
    /// Port configuration
    pub http_port: Mutex<u16>,
    pub socks_port: Mutex<u16>,
    pub mixed_port: Mutex<u16>,
    /// Live bandwidth statistics
    pub upload_bytes: Mutex<u64>,
    pub download_bytes: Mutex<u64>,
    pub upload_speed: Mutex<u64>,
    pub download_speed: Mutex<u64>,
    pub active_connections: Mutex<u32>,
}

impl ProxyState {
    pub fn new(http_port: u16, socks_port: u16, mixed_port: u16) -> Self {
        Self {
            child_process: Mutex::new(None),
            system_proxy_active: Mutex::new(false),
            connection_status: Mutex::new(ConnectionStatus::Disconnected),
            log_buffer: Mutex::new(VecDeque::with_capacity(LOG_BUFFER_RUST_MAX)),
            http_port: Mutex::new(http_port),
            socks_port: Mutex::new(socks_port),
            mixed_port: Mutex::new(mixed_port),
            upload_bytes: Mutex::new(0),
            download_bytes: Mutex::new(0),
            upload_speed: Mutex::new(0),
            download_speed: Mutex::new(0),
            active_connections: Mutex::new(0),
        }
    }

    pub fn get_status(&self) -> ConnectionStatus {
        if let Ok(g) = self.connection_status.lock() {
            *g
        } else {
            ConnectionStatus::Disconnected
        }
    }

    pub fn set_status(&self, status: ConnectionStatus) {
        if let Ok(mut g) = self.connection_status.lock() {
            *g = status;
        }
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
