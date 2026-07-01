use std::net::IpAddr;
use serde_json::Value;
#[cfg(target_os = "windows")]
use std::process::Command;

pub mod adapters;
pub mod generator;
pub mod validator;
pub mod rules;

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

pub fn build_route_rules(
    bypass_lan: bool,
    user_rules: &[crate::config::rules::RoutingRule],
    rule_sets: &[crate::config::rules::RuleSet],
) -> Vec<Value> {
    let mut rules = vec![
        serde_json::json!({ "protocol": "dns", "action": "hijack-dns" }),
    ];

    if bypass_lan {
        // Native, database-free private-network matching (no geoip/geosite
        // resource files required, unlike the deprecated geoip/geosite fields).
        rules.push(serde_json::json!({ "ip_is_private": true, "outbound": "direct" }));
        rules.push(serde_json::json!({
            "domain_suffix": [".lan", ".local", ".internal", ".home.arpa"],
            "outbound": "direct"
        }));
    }

    // User-defined rules evaluated top-to-bottom
    for rule in user_rules {
        if rule.enabled.unwrap_or(true) {
            if let Some(json_rule) = crate::config::rules::build_singbox_rule(rule) {
                rules.push(json_rule);
            }
        }
    }

    // Rule-set references (injected as individual rule entries)
    for rs in rule_sets {
        rules.push(serde_json::json!({
            "rule_set": [rs.tag],
            "outbound": "direct"
        }));
    }

    rules
}

pub fn apply_tun_compatibility_profile(config_val: &mut Value) -> bool {
    let mut changed = false;

    if let Some(inbounds) = config_val.get_mut("inbounds").and_then(|v| v.as_array_mut()) {
        for inbound in inbounds {
            if inbound.get("type").and_then(|v| v.as_str()) == Some("tun") {
                inbound["address"] = serde_json::json!(["172.19.0.1/30"]);
                inbound["auto_route"] = serde_json::json!(true);
                inbound["strict_route"] = serde_json::json!(true);
                inbound["stack"] = serde_json::json!("gvisor");
                inbound["sniff"] = serde_json::json!(true);
                inbound["sniff_override_destination"] = serde_json::json!(true);
                changed = true;
            }
        }
    }

    if has_vless_websocket_outbound(config_val) {
        let udp_reject = serde_json::json!({
            "network": "udp",
            "action": "reject",
            "method": "drop"
        });

        if let Some(route) = config_val.get_mut("route").and_then(|v| v.as_object_mut()) {
            let rules = route
                .entry("rules".to_string())
                .or_insert_with(|| serde_json::json!([]));
            if let Some(rule_list) = rules.as_array_mut() {
                let already_present = rule_list.iter().any(|rule| {
                    rule.get("network").and_then(|v| v.as_str()) == Some("udp")
                        && rule.get("action").and_then(|v| v.as_str()) == Some("reject")
                });

                if !already_present {
                    let insert_at = rule_list
                        .iter()
                        .position(|rule| rule.get("protocol").and_then(|v| v.as_str()) != Some("dns"))
                        .unwrap_or(rule_list.len());
                    rule_list.insert(insert_at, udp_reject);
                    changed = true;
                }
            }
        }
    }

    changed
}

fn has_vless_websocket_outbound(config_val: &Value) -> bool {
    config_val
        .get("outbounds")
        .and_then(|v| v.as_array())
        .map(|outbounds| {
            outbounds.iter().any(|outbound| {
                outbound.get("type").and_then(|v| v.as_str()) == Some("vless")
                    && outbound
                        .get("transport")
                        .and_then(|v| v.get("type"))
                        .and_then(|v| v.as_str())
                        .map(|transport| transport == "ws" || transport == "httpupgrade")
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false)
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
    use std::os::windows::process::CommandExt;
    let output = Command::new("ipconfig")
        .arg("/all")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_excludes_proxy_server_ips() {
        let addresses = build_route_exclude_addresses(
            "192.168.8.1",
            &["139.59.105.103/32".to_string(), "2001:db8::1/128".to_string()],
        );

        assert!(addresses.contains(&"192.168.8.1/32".to_string()));
        assert!(addresses.contains(&"139.59.105.103/32".to_string()));
        assert!(addresses.contains(&"2001:db8::1/128".to_string()));
    }

    #[test]
    fn test_tun_compatibility_profile_uses_ipv4_only_and_sniff_override() {
        let mut config = serde_json::json!({
            "inbounds": [{
                "type": "tun",
                "address": ["172.19.0.1/30", "fdfe:dcba:9876::1/126"],
                "auto_route": false,
                "strict_route": false,
                "stack": "system",
                "sniff": true,
                "sniff_override_destination": false
            }],
            "outbounds": [{
                "type": "vless",
                "tag": "node",
                "transport": { "type": "ws" }
            }],
            "route": {
                "rules": [{ "protocol": "dns", "action": "hijack-dns" }],
                "final": "proxy"
            }
        });

        assert!(apply_tun_compatibility_profile(&mut config));
        let tun = &config["inbounds"][0];
        assert_eq!(tun["address"], serde_json::json!(["172.19.0.1/30"]));
        assert_eq!(tun["auto_route"], serde_json::json!(true));
        assert_eq!(tun["strict_route"], serde_json::json!(true));
        assert_eq!(tun["stack"], serde_json::json!("gvisor"));
        assert_eq!(tun["sniff_override_destination"], serde_json::json!(true));
    }

    #[test]
    fn test_tun_compatibility_profile_rejects_udp_for_vless_ws() {
        let mut config = serde_json::json!({
            "inbounds": [{ "type": "tun" }],
            "outbounds": [{
                "type": "vless",
                "tag": "node",
                "transport": { "type": "ws" }
            }],
            "route": {
                "rules": [
                    { "protocol": "dns", "action": "hijack-dns" },
                    { "ip_is_private": true, "outbound": "direct" }
                ],
                "final": "proxy"
            }
        });

        apply_tun_compatibility_profile(&mut config);
        let rules = config["route"]["rules"].as_array().unwrap();
        assert_eq!(rules[0]["protocol"].as_str().unwrap(), "dns");
        assert_eq!(rules[1]["network"].as_str().unwrap(), "udp");
        assert_eq!(rules[1]["action"].as_str().unwrap(), "reject");
        assert_eq!(rules[1]["method"].as_str().unwrap(), "drop");
    }
}
