use crate::config::generator::TunSettings;
use crate::os::{PlatformNetworkManager, TunSupportStatus};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tauri::{path::BaseDirectory, AppHandle, Manager};

pub struct LinuxNetworkManager;

impl LinuxNetworkManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxNetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn get_linux_helper_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("x-link")
        .join("helpers")
}

pub fn has_linux_cap_net_admin(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    if let Ok(output) = std::process::Command::new("getcap").arg(path).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        return stdout.contains("cap_net_admin");
    }
    false
}

pub fn is_linux_tun_capable() -> bool {
    if crate::os::is_elevated() {
        return true;
    }
    let helper_singbox = get_linux_helper_dir().join("sing-box");
    if has_linux_cap_net_admin(&helper_singbox) {
        return true;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with("sing-box") && !name.contains("supervisor") {
                        if has_linux_cap_net_admin(&entry.path()) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

pub fn spawn_linux_supervisor(
    config_path: &Path,
    app_data_dir: &Path,
) -> Result<tokio::process::Child, std::io::Error> {
    let helper_dir = get_linux_helper_dir();
    let supervisor_path = {
        let candidate = helper_dir.join("xlink-linux-supervisor");
        if candidate.exists() {
            candidate
        } else if let Ok(exe) = std::env::current_exe() {
            exe.parent()
                .map(|d| d.join("xlink-linux-supervisor"))
                .unwrap_or(candidate)
        } else {
            candidate
        }
    };

    tokio::process::Command::new(&supervisor_path)
        .args([
            "run",
            "-c",
            config_path.to_str().unwrap_or_default(),
            "-D",
            app_data_dir.to_str().unwrap_or_default(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

fn resolve_source_binary(app_handle: &AppHandle, prefix: &str) -> Result<PathBuf, String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let direct = dir.join(prefix);
            if direct.exists() {
                return Ok(direct);
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with(prefix) {
                        return Ok(entry.path());
                    }
                }
            }
        }
    }

    app_handle
        .path()
        .resolve(format!("resources/{}", prefix), BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve {} binary: {}", prefix, e))
}

#[async_trait::async_trait]
impl PlatformNetworkManager for LinuxNetworkManager {
    async fn configure_system_proxy(&self, port: u16, _bypass_local: bool) -> Result<(), String> {
        crate::os::enable_system_proxy_internal("127.0.0.1", port)
    }

    async fn restore_system_proxy(&self) -> Result<(), String> {
        self.restore_system_proxy_sync()
    }

    fn restore_system_proxy_sync(&self) -> Result<(), String> {
        crate::os::disable_system_proxy_internal()
    }

    async fn check_tun_support(&self) -> Result<TunSupportStatus, String> {
        Ok(crate::os::resolve_linux_tun_status(is_linux_tun_capable()))
    }

    async fn provision_tun_interface(&self, _settings: &TunSettings) -> Result<String, String> {
        if !is_linux_tun_capable() {
            return Err(
                "Linux TUN capabilities (cap_net_admin) are not granted on sing-box".to_string(),
            );
        }
        Ok("X-Link".to_string())
    }

    async fn request_platform_elevation(&self, app_handle: &AppHandle) -> Result<(), String> {
        let helper_dir = get_linux_helper_dir();
        std::fs::create_dir_all(&helper_dir)
            .map_err(|e| format!("Failed to create Linux helper directory: {}", e))?;

        let singbox_src = resolve_source_binary(app_handle, "sing-box")?;
        let singbox_dst = helper_dir.join("sing-box");
        std::fs::copy(&singbox_src, &singbox_dst)
            .map_err(|e| format!("Failed to stage sing-box helper binary: {}", e))?;

        if let Ok(sup_src) = resolve_source_binary(app_handle, "xlink-linux-supervisor") {
            let sup_dst = helper_dir.join("xlink-linux-supervisor");
            let _ = std::fs::copy(&sup_src, &sup_dst);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&sup_dst) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&sup_dst, perms);
                }
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&singbox_dst) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&singbox_dst, perms);
            }
        }

        let output = std::process::Command::new("pkexec")
            .args([
                "setcap",
                "cap_net_admin,cap_net_bind_service=+ep",
                singbox_dst.to_str().ok_or("Invalid helper path encoding")?,
            ])
            .output()
            .map_err(|e| format!("Failed to execute pkexec: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!("Failed to grant Linux TUN capabilities: {}", err));
        }

        Ok(())
    }

    async fn get_system_dns_servers(&self) -> Result<Vec<IpAddr>, String> {
        let mut servers = Vec::new();
        if let Ok(content) = std::fs::read_to_string("/etc/resolv.conf") {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some(rest) = trimmed.strip_prefix("nameserver") {
                    let token = rest
                        .trim()
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .split('%')
                        .next()
                        .unwrap_or("");
                    if let Ok(ip) = token.parse::<IpAddr>() {
                        if !ip.is_loopback() && !ip.is_unspecified() && !servers.contains(&ip) {
                            servers.push(ip);
                        }
                    }
                }
            }
        }
        Ok(servers)
    }
}
