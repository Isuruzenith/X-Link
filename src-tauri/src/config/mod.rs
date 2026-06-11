use std::net::IpAddr;
use serde_json::Value;
#[cfg(target_os = "windows")]
use std::process::Command;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub mod adapters;
pub mod generator;
pub mod validator;

pub fn get_config_path(app: &AppHandle, profile_id: &str) -> Result<PathBuf, String> {
    let mut path = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    path.push("configs");
    
    // Create directory if it doesn't exist
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create configs directory: {}", e))?;
    
    path.push(format!("{}.json", profile_id));
    Ok(path)
}

pub fn build_route_exclude_addresses(dns_address: &str, server_ips: &[String]) -> Vec<String> {
    let mut addresses = vec![
        "10.0.0.0/8".to_string(),
        "172.16.0.0/12".to_string(),
        "192.168.0.0/16".to_string(),
        "127.0.0.0/8".to_string(),
        "169.254.0.0/16".to_string(),
        "fc00::/7".to_string(),
        "fe80::/10".to_string(),
    ];

    if let Ok(ip) = dns_address.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(addr) => addresses.push(format!("{}/32", addr)),
            IpAddr::V6(addr) => addresses.push(format!("{}/128", addr)),
        }
    }

    for ip in server_ips {
        if !addresses.contains(ip) {
            addresses.push(ip.clone());
        }
    }

    addresses
}

pub fn resolve_server_ips(servers: &[String]) -> Vec<String> {
    use std::net::ToSocketAddrs;
    let mut ip_strings = Vec::new();
    for server in servers {
        let trimmed = server.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(ip) = trimmed.parse::<IpAddr>() {
            match ip {
                IpAddr::V4(addr) => ip_strings.push(format!("{}/32", addr)),
                IpAddr::V6(addr) => ip_strings.push(format!("{}/128", addr)),
            }
        } else {
            let host_port = format!("{}:443", trimmed);
            if let Ok(addrs) = host_port.to_socket_addrs() {
                for addr in addrs {
                    match addr.ip() {
                        IpAddr::V4(a) => {
                            let cidr = format!("{}/32", a);
                            if !ip_strings.contains(&cidr) {
                                ip_strings.push(cidr);
                            }
                        }
                        IpAddr::V6(a) => {
                            let cidr = format!("{}/128", a);
                            if !ip_strings.contains(&cidr) {
                                ip_strings.push(cidr);
                            }
                        }
                    }
                }
            }
        }
    }
    ip_strings
}

pub fn build_route_rules() -> Vec<Value> {
    vec![
        serde_json::json!({ "protocol": "dns", "action": "hijack-dns" }),
        serde_json::json!({ "geoip": "private", "action": "direct" }),
    ]
}

pub fn resolve_dns_address(configured: Option<&str>) -> String {
    if let Some(address) = configured {
        let trimmed = address.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Some(address) = get_system_dns_address() {
        return address;
    }

    "1.1.1.1".to_string()
}

fn extract_ip_from_line(line: &str) -> Option<String> {
    for token in line.split(|c: char| c.is_whitespace() || c == ':' || c == ',') {
        let cleaned = token.trim();
        if cleaned.is_empty() {
            continue;
        }
        let cleaned = cleaned.split('%').next().unwrap_or(cleaned);
        if let Ok(ip) = cleaned.parse::<IpAddr>() {
            return Some(ip.to_string());
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn get_system_dns_address() -> Option<String> {
    let output = Command::new("ipconfig")
        .arg("/all")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut in_dns_block = false;
    for line in text.lines() {
        if line.contains("DNS Servers") {
            in_dns_block = true;
        }

        if in_dns_block {
            if let Some(ip) = extract_ip_from_line(line) {
                // Ignore X-Link TUN virtual DNS address to prevent circular routing loops
                if ip != "172.19.0.2" && ip != "fdfe:dcba:9876::2" {
                    return Some(ip);
                }
            }

            if !line.starts_with(' ') && !line.starts_with('\t') && !line.contains("DNS Servers") {
                in_dns_block = false;
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn get_system_dns_address() -> Option<String> {
    let content = std::fs::read_to_string("/etc/resolv.conf").ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("nameserver") {
            if let Some(ip) = extract_ip_from_line(rest) {
                return Some(ip);
            }
        }
    }
    None
}
