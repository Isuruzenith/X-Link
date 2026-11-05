use crate::config::generator::TunSettings;
use crate::os::{PlatformNetworkManager, TunSupportStatus};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tauri::{path::BaseDirectory, AppHandle, Manager};

pub const DARWIN_HELPER_DIR: &str = "/Library/Application Support/X-Link/helpers";
pub const DARWIN_SUPERVISOR_PATH: &str =
    "/Library/Application Support/X-Link/helpers/xlink-darwin-supervisor";
pub const DARWIN_SINGBOX_PATH: &str = "/Library/Application Support/X-Link/helpers/sing-box";
pub const DARWIN_SUDOERS_PATH: &str = "/private/etc/sudoers.d/xlink-singbox";

pub struct DarwinNetworkManager;

impl DarwinNetworkManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DarwinNetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn is_darwin_helper_ready() -> bool {
    let helper_exists =
        Path::new(DARWIN_SUPERVISOR_PATH).exists() && Path::new(DARWIN_SINGBOX_PATH).exists();
    if !helper_exists {
        return false;
    }
    // Verify passwordless sudo authorization via non-interactive sudo -n -l
    std::process::Command::new("sudo")
        .args(["-n", "-l", DARWIN_SUPERVISOR_PATH])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn spawn_darwin_supervisor(
    config_path: &Path,
    app_data_dir: &Path,
) -> Result<tokio::process::Child, std::io::Error> {
    let config_str = config_path.to_str().unwrap_or_default();
    let data_str = app_data_dir.to_str().unwrap_or_default();
    let parent_pid = std::process::id().to_string();

    tokio::process::Command::new("sudo")
        .args([
            "-n",
            DARWIN_SUPERVISOR_PATH,
            "run",
            "-c",
            config_str,
            "-D",
            data_str,
            "--parent-pid",
            &parent_pid,
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
impl PlatformNetworkManager for DarwinNetworkManager {
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
        let ready = crate::os::is_elevated() || is_darwin_helper_ready();
        Ok(crate::os::resolve_darwin_tun_status(ready))
    }

    async fn provision_tun_interface(&self, _settings: &TunSettings) -> Result<String, String> {
        if !crate::os::is_elevated() && !is_darwin_helper_ready() {
            return Err(
                "macOS TUN helper is not authorized. Please grant administrator permissions."
                    .to_string(),
            );
        }
        // Dynamic utun allocation by Darwin kernel
        Ok("utun".to_string())
    }

    async fn request_platform_elevation(&self, app_handle: &AppHandle) -> Result<(), String> {
        let supervisor_src = resolve_source_binary(app_handle, "xlink-darwin-supervisor")?;
        let singbox_src = resolve_source_binary(app_handle, "sing-box")?;

        let sup_str = supervisor_src
            .to_str()
            .ok_or("Invalid supervisor path encoding")?
            .replace('\'', "'\\''");
        let sb_str = singbox_src
            .to_str()
            .ok_or("Invalid sing-box path encoding")?
            .replace('\'', "'\\''");

        let shell_cmd = format!(
            "set -e; \
             trap 'rm -f /private/etc/sudoers.d/.xlink-singbox.tmp' EXIT INT TERM; \
             mkdir -p -m 0755 '{helper_dir}'; \
             cp -f '{sup_src}' '{sup_dst}'; \
             cp -f '{sb_src}' '{sb_dst}'; \
             chown root:wheel '{helper_dir}' '{sup_dst}' '{sb_dst}'; \
             chmod 0755 '{sup_dst}' '{sb_dst}'; \
             mkdir -p -m 0755 /private/etc/sudoers.d; \
             printf '%s\\n' '%admin ALL=(root) NOPASSWD: /Library/Application\\ Support/X-Link/helpers/xlink-darwin-supervisor' > /private/etc/sudoers.d/.xlink-singbox.tmp; \
             chown root:wheel /private/etc/sudoers.d/.xlink-singbox.tmp; \
             chmod 0440 /private/etc/sudoers.d/.xlink-singbox.tmp; \
             visudo -c -f /private/etc/sudoers.d/.xlink-singbox.tmp; \
             mv -f /private/etc/sudoers.d/.xlink-singbox.tmp '{sudoers_dst}'",
            helper_dir = DARWIN_HELPER_DIR,
            sup_src = sup_str,
            sup_dst = DARWIN_SUPERVISOR_PATH,
            sb_src = sb_str,
            sb_dst = DARWIN_SINGBOX_PATH,
            sudoers_dst = DARWIN_SUDOERS_PATH,
        );

        let escaped = shell_cmd.replace('\\', "\\\\").replace('"', "\\\"");
        let applescript = format!(
            "do shell script \"{}\" with administrator privileges",
            escaped
        );

        let output = std::process::Command::new("osascript")
            .args(["-e", &applescript])
            .output()
            .map_err(|e| format!("Failed to execute osascript: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!("macOS elevation failed or was cancelled: {}", err));
        }

        Ok(())
    }

    async fn get_system_dns_servers(&self) -> Result<Vec<IpAddr>, String> {
        let mut servers = Vec::new();
        if let Ok(output) = std::process::Command::new("scutil").arg("--dns").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("nameserver[") {
                    if let Some((_, addr_part)) = rest.split_once(':') {
                        let cleaned = addr_part.trim().split('%').next().unwrap_or("").trim();
                        if let Ok(ip) = cleaned.parse::<IpAddr>() {
                            if !ip.is_loopback() && !ip.is_unspecified() && !servers.contains(&ip) {
                                servers.push(ip);
                            }
                        }
                    }
                }
            }
        }
        Ok(servers)
    }
}
