use serde_json::Value;
use std::net::IpAddr;
#[cfg(target_os = "windows")]
use std::process::Command;

pub mod adapters;
pub mod generator;
pub mod rules;
pub mod validator;

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
    sniff_enabled: bool,
    user_rules: &[crate::config::rules::RoutingRule],
    _rule_sets: &[crate::config::rules::RuleSet],
    geosite_db_exists: bool,
    geoip_db_exists: bool,
) -> Vec<Value> {
    let mut rules = vec![serde_json::json!({ "port": 53, "action": "hijack-dns" })];

    if sniff_enabled {
        rules.push(serde_json::json!({
            "action": "sniff",
            "sniffer": ["http", "tls", "quic"]
        }));
    }

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
            if let Some(json_rule) =
                crate::config::rules::build_singbox_rule(rule, geosite_db_exists, geoip_db_exists)
            {
                rules.push(json_rule);
            }
        }
    }

    rules
}

pub fn apply_tun_compatibility_profile(config_val: &mut Value) -> bool {
    let mut changed = false;

    if let Some(inbounds) = config_val
        .get_mut("inbounds")
        .and_then(|v| v.as_array_mut())
    {
        for inbound in inbounds {
            if inbound.get("type").and_then(|v| v.as_str()) == Some("tun") {
                inbound["address"] = serde_json::json!(["172.19.0.1/30"]);
                inbound["auto_route"] = serde_json::json!(true);
                inbound["strict_route"] = serde_json::json!(true);
                if inbound.get("stack").is_none() {
                    inbound["stack"] = serde_json::json!("system");
                }
                // Remove legacy inbound fields
                if let Some(map) = inbound.as_object_mut() {
                    map.remove("sniff");
                    map.remove("sniff_override_destination");
                }
                changed = true;
            }
        }
    }

    if changed {
        // Ensure route rules have the sniff action
        if let Some(rules) = config_val
            .get_mut("route")
            .and_then(|r| r.get_mut("rules"))
            .and_then(|r| r.as_array_mut())
        {
            let has_sniff = rules
                .iter()
                .any(|r| r.get("action").and_then(|a| a.as_str()) == Some("sniff"));
            if !has_sniff {
                rules.insert(
                    0,
                    serde_json::json!({
                        "action": "sniff",
                        "sniffer": ["http", "tls", "quic"]
                    }),
                );
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
                        .position(|rule| {
                            let is_dns =
                                rule.get("action").and_then(|v| v.as_str()) == Some("hijack-dns");
                            let is_sniff =
                                rule.get("action").and_then(|v| v.as_str()) == Some("sniff");
                            !is_dns && !is_sniff
                        })
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

    struct AdapterInfo {
        _name: String,
        is_virtual: bool,
        dns_servers: Vec<String>,
        has_gateway: bool,
    }

    let mut adapters = Vec::new();
    let mut current_adapter: Option<AdapterInfo> = None;
    let mut in_dns_block = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // A line starting with no whitespace indicates the start of a new section/adapter
        if !line.starts_with(' ') && !line.starts_with('\t') {
            if let Some(adapter) = current_adapter.take() {
                adapters.push(adapter);
            }
            in_dns_block = false;
            let section_name = trimmed.trim_end_matches(':').to_string();
            current_adapter = Some(AdapterInfo {
                _name: section_name,
                is_virtual: false,
                dns_servers: Vec::new(),
                has_gateway: false,
            });
            continue;
        }

        if let Some(ref mut adapter) = current_adapter {
            if line.contains("Description") {
                if let Some(desc) = line.split(':').nth(1) {
                    let desc_trimmed = desc.trim().to_string();
                    let desc_lower = desc_trimmed.to_lowercase();
                    // Identify virtual/TAP adapters
                    if desc_lower.contains("tap")
                        || desc_lower.contains("virtual")
                        || desc_lower.contains("vpn")
                        || desc_lower.contains("vmware")
                        || desc_lower.contains("virtualbox")
                        || desc_lower.contains("hyper-v")
                        || desc_lower.contains("loopback")
                        || desc_lower.contains("pseudo")
                    {
                        adapter.is_virtual = true;
                    }
                }
            } else if line.contains("Default Gateway") {
                if let Some(gw) = line.split(':').nth(1) {
                    let gw_trimmed = gw.trim();
                    if !gw_trimmed.is_empty() && gw_trimmed != "0.0.0.0" {
                        adapter.has_gateway = true;
                    }
                }
            } else if line.contains("DNS Servers") {
                in_dns_block = true;
                if let Some(ip) = extract_ip_from_line(line) {
                    if ip != "172.19.0.2" && ip != "fdfe:dcba:9876::2" {
                        adapter.dns_servers.push(ip);
                    }
                }
            } else if in_dns_block {
                if let Some(ip) = extract_ip_from_line(line) {
                    if ip != "172.19.0.2" && ip != "fdfe:dcba:9876::2" {
                        adapter.dns_servers.push(ip);
                    }
                } else {
                    in_dns_block = false;
                }
            }
        }
    }

    if let Some(adapter) = current_adapter {
        adapters.push(adapter);
    }

    // Prioritize non-virtual adapters with default gateways
    for adapter in &adapters {
        if !adapter.is_virtual && adapter.has_gateway && !adapter.dns_servers.is_empty() {
            return Some(adapter.dns_servers[0].clone());
        }
    }

    // Prioritize non-virtual adapters with DNS servers
    for adapter in &adapters {
        if !adapter.is_virtual && !adapter.dns_servers.is_empty() {
            return Some(adapter.dns_servers[0].clone());
        }
    }

    // Prioritize virtual adapters with default gateways
    for adapter in &adapters {
        if adapter.is_virtual && adapter.has_gateway && !adapter.dns_servers.is_empty() {
            return Some(adapter.dns_servers[0].clone());
        }
    }

    // Fallback to any adapter with DNS servers
    for adapter in &adapters {
        if !adapter.dns_servers.is_empty() {
            return Some(adapter.dns_servers[0].clone());
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
            &[
                "139.59.105.103/32".to_string(),
                "2001:db8::1/128".to_string(),
            ],
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
                "rules": [{ "port": 53, "action": "hijack-dns" }],
                "final": "proxy"
            }
        });

        assert!(apply_tun_compatibility_profile(&mut config));
        let tun = &config["inbounds"][0];
        assert_eq!(tun["address"], serde_json::json!(["172.19.0.1/30"]));
        assert_eq!(tun["auto_route"], serde_json::json!(true));
        assert_eq!(tun["strict_route"], serde_json::json!(true));
        assert_eq!(tun["stack"], serde_json::json!("system"));
        assert!(tun.get("sniff").is_none());
        assert!(tun.get("sniff_override_destination").is_none());

        let rules = config["route"]["rules"].as_array().unwrap();
        assert_eq!(rules[0]["action"].as_str().unwrap(), "sniff");
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
                    { "port": 53, "action": "hijack-dns" },
                    { "ip_is_private": true, "outbound": "direct" }
                ],
                "final": "proxy"
            }
        });

        apply_tun_compatibility_profile(&mut config);
        let rules = config["route"]["rules"].as_array().unwrap();
        assert_eq!(rules[0]["action"].as_str().unwrap(), "sniff");
        assert_eq!(rules[1]["port"].as_u64().unwrap(), 53);
        assert_eq!(rules[2]["network"].as_str().unwrap(), "udp");
        assert_eq!(rules[2]["action"].as_str().unwrap(), "reject");
        assert_eq!(rules[2]["method"].as_str().unwrap(), "drop");
    }

    #[test]
    fn test_build_route_rules_respects_user_rules_and_does_not_auto_route_rule_sets() {
        let user_rules = vec![crate::config::rules::RoutingRule {
            id: "r1".to_string(),
            rule_type: "domain".to_string(),
            value: "custom-bypass.com".to_string(),
            outbound: "direct".to_string(),
            invert: false,
            notes: None,
            enabled: Some(true),
        }];

        let rule_sets = vec![crate::config::rules::RuleSet {
            id: "rs1".to_string(),
            tag: "geosite-youtube".to_string(),
            set_type: "remote".to_string(),
            format: "binary".to_string(),
            url: Some("https://example.com".to_string()),
            file_path: None,
            update_interval: "1d".to_string(),
        }];

        let rules = build_route_rules(true, true, &user_rules, &rule_sets, true, true);

        // rules should contain hijack-dns (port 53), sniff, bypass_lan (ip_is_private & domain_suffix),
        // and our user rule (custom-bypass.com).
        // It must NOT contain any auto-generated rule-set routing rules for "geosite-youtube"!

        let custom_bypass_rule = rules.iter().find(|r| {
            r.get("domain")
                .and_then(|d| d.as_array())
                .map(|arr| arr.iter().any(|v| v.as_str() == Some("custom-bypass.com")))
                .unwrap_or(false)
        });
        assert!(custom_bypass_rule.is_some());

        let auto_ruleset_rule = rules.iter().find(|r| {
            r.get("rule_set")
                .and_then(|rs| rs.as_array())
                .map(|arr| arr.iter().any(|v| v.as_str() == Some("geosite-youtube")))
                .unwrap_or(false)
        });
        assert!(auto_ruleset_rule.is_none());
    }
}
