use serde_json::Value;
use std::net::IpAddr;

pub mod adapters;
pub mod generator;
pub mod rules;
pub mod subscription;
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

#[cfg(not(target_os = "windows"))]
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

/// Internal adapter representation used to select an operational physical/default DNS adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedAdapter {
    pub name: String,
    pub description: String,
    pub is_physical: bool,
    pub is_operational: bool,
    pub has_ipv4_gateway: bool,
    pub has_ipv6_gateway: bool,
    pub dns_servers: Vec<IpAddr>,
}

/// Selects the best DNS server address from a list of detected network adapters.
///
/// Priority:
/// 1. Operational, physical adapter with default gateway (IPv4 or IPv6) and valid non-excluded DNS.
/// 2. Operational, physical adapter with valid non-excluded DNS.
/// 3. Any operational adapter with default gateway and valid non-excluded DNS.
/// 4. Any operational adapter with valid non-excluded DNS.
/// 5. None (fallback).
pub fn select_best_dns_server(adapters: &[DetectedAdapter]) -> Option<IpAddr> {
    fn is_tun_or_xlink_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(v4) => {
                // Sing-box TUN default FakeIP / internal DNS addresses
                *v4 == std::net::Ipv4Addr::new(172, 19, 0, 2)
            }
            IpAddr::V6(v6) => {
                // Sing-box TUN IPv6 internal DNS address
                *v6 == "fdfe:dcba:9876::2".parse::<std::net::Ipv6Addr>().unwrap()
            }
        }
    }

    fn is_valid_dns(ip: &IpAddr) -> bool {
        !ip.is_loopback() && !ip.is_unspecified() && !is_tun_or_xlink_ip(ip)
    }

    fn is_xlink_or_tun_adapter(adapter: &DetectedAdapter) -> bool {
        let name_lower = adapter.name.to_lowercase();
        let desc_lower = adapter.description.to_lowercase();
        name_lower.contains("x-link")
            || name_lower.contains("sing-box")
            || desc_lower.contains("wintun")
            || desc_lower.contains("tap-windows")
    }

    let operational: Vec<&DetectedAdapter> = adapters
        .iter()
        .filter(|a| a.is_operational && !is_xlink_or_tun_adapter(a))
        .collect();

    // 1. Physical adapter with default gateway
    for a in &operational {
        if a.is_physical && (a.has_ipv4_gateway || a.has_ipv6_gateway) {
            if let Some(dns) = a.dns_servers.iter().find(|ip| is_valid_dns(ip)) {
                return Some(*dns);
            }
        }
    }

    // 2. Physical adapter without gateway
    for a in &operational {
        if a.is_physical {
            if let Some(dns) = a.dns_servers.iter().find(|ip| is_valid_dns(ip)) {
                return Some(*dns);
            }
        }
    }

    // 3. Any operational adapter with gateway
    for a in &operational {
        if a.has_ipv4_gateway || a.has_ipv6_gateway {
            if let Some(dns) = a.dns_servers.iter().find(|ip| is_valid_dns(ip)) {
                return Some(*dns);
            }
        }
    }

    // 4. Any operational adapter with DNS
    for a in &operational {
        if let Some(dns) = a.dns_servers.iter().find(|ip| is_valid_dns(ip)) {
            return Some(*dns);
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn get_system_dns_address() -> Option<String> {
    use std::net::{Ipv4Addr, Ipv6Addr};
    use windows::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST,
        GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6,
    };

    unsafe {
        let flags = GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST;
        let mut buf_len: u32 = 16384;
        let mut buffer: Vec<u8> = vec![0u8; buf_len as usize];

        let mut ret = GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            flags,
            None,
            Some(buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
            &mut buf_len,
        );

        if ret == ERROR_BUFFER_OVERFLOW.0 {
            buffer.resize(buf_len as usize, 0);
            ret = GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                flags,
                None,
                Some(buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut buf_len,
            );
        }

        if ret != 0 {
            return None;
        }

        let mut adapters = Vec::new();
        let mut curr_ptr = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;

        while !curr_ptr.is_null() {
            let entry = &*curr_ptr;

            let name = if !entry.AdapterName.0.is_null() {
                windows::core::PCSTR(entry.AdapterName.0)
                    .to_string()
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let description = if !entry.Description.0.is_null() {
                entry.Description.to_string().unwrap_or_default()
            } else {
                String::new()
            };

            let if_type = entry.IfType;
            // IF_TYPE_ETHERNET_CSMACD = 6, IF_TYPE_IEEE80211 = 71
            let is_physical = if_type == 6 || if_type == 71;

            // IfOperStatusUp = 1
            let is_operational = entry.OperStatus.0 == 1;

            let mut has_ipv4_gateway = false;
            let mut has_ipv6_gateway = false;
            let mut gw_ptr = entry.FirstGatewayAddress;
            while !gw_ptr.is_null() {
                let gw_entry = &*gw_ptr;
                let sock_ptr = gw_entry.Address.lpSockaddr;
                if !sock_ptr.is_null() {
                    let family = (*sock_ptr).sa_family;
                    if family == AF_INET {
                        has_ipv4_gateway = true;
                    } else if family == AF_INET6 {
                        has_ipv6_gateway = true;
                    }
                }
                gw_ptr = gw_entry.Next;
            }

            let mut dns_servers = Vec::new();
            let mut dns_ptr = entry.FirstDnsServerAddress;
            while !dns_ptr.is_null() {
                let dns_entry = &*dns_ptr;
                let sock_ptr = dns_entry.Address.lpSockaddr;
                if !sock_ptr.is_null() {
                    let family = (*sock_ptr).sa_family;
                    if family == AF_INET {
                        let sin = &*(sock_ptr as *const SOCKADDR_IN);
                        let ip_bytes = sin.sin_addr.S_un.S_addr.to_ne_bytes();
                        dns_servers.push(IpAddr::V4(Ipv4Addr::from(ip_bytes)));
                    } else if family == AF_INET6 {
                        let sin6 = &*(sock_ptr as *const SOCKADDR_IN6);
                        let ip_bytes = sin6.sin6_addr.u.Byte;
                        dns_servers.push(IpAddr::V6(Ipv6Addr::from(ip_bytes)));
                    }
                }
                dns_ptr = dns_entry.Next;
            }

            adapters.push(DetectedAdapter {
                name,
                description,
                is_physical,
                is_operational,
                has_ipv4_gateway,
                has_ipv6_gateway,
                dns_servers,
            });

            curr_ptr = entry.Next;
        }

        select_best_dns_server(&adapters).map(|ip| ip.to_string())
    }
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

    #[test]
    fn test_select_best_dns_server_physical_with_gateway_preferred() {
        use std::net::Ipv4Addr;

        let adapters = vec![
            DetectedAdapter {
                name: "VirtualBox Host-Only Ethernet Adapter".to_string(),
                description: "VirtualBox Host-Only Ethernet Adapter".to_string(),
                is_physical: false,
                is_operational: true,
                has_ipv4_gateway: false,
                has_ipv6_gateway: false,
                dns_servers: vec![IpAddr::V4(Ipv4Addr::new(192, 168, 56, 1))],
            },
            DetectedAdapter {
                name: "Wi-Fi".to_string(),
                description: "Intel(R) Wi-Fi 6 AX200 160MHz".to_string(),
                is_physical: true,
                is_operational: true,
                has_ipv4_gateway: true,
                has_ipv6_gateway: false,
                dns_servers: vec![
                    IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                    IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
                ],
            },
        ];

        let selected = select_best_dns_server(&adapters);
        assert_eq!(selected, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
    }

    #[test]
    fn test_select_best_dns_server_skips_disconnected_and_tun() {
        use std::net::{Ipv4Addr, Ipv6Addr};

        let adapters = vec![
            // Operational TUN adapter (wintun / X-Link) with sing-box internal DNS
            DetectedAdapter {
                name: "x-link-tun".to_string(),
                description: "Wintun Userspace Tunnel".to_string(),
                is_physical: false,
                is_operational: true,
                has_ipv4_gateway: true,
                has_ipv6_gateway: false,
                dns_servers: vec![
                    IpAddr::V4(Ipv4Addr::new(172, 19, 0, 2)),
                    IpAddr::V6("fdfe:dcba:9876::2".parse::<Ipv6Addr>().unwrap()),
                ],
            },
            // Physical but disconnected (not operational)
            DetectedAdapter {
                name: "Ethernet".to_string(),
                description: "Realtek PCIe GbE Family Controller".to_string(),
                is_physical: true,
                is_operational: false,
                has_ipv4_gateway: true,
                has_ipv6_gateway: false,
                dns_servers: vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))],
            },
            // Operational physical Wi-Fi with valid DNS
            DetectedAdapter {
                name: "Wi-Fi".to_string(),
                description: "Qualcomm Wi-Fi Adapter".to_string(),
                is_physical: true,
                is_operational: true,
                has_ipv4_gateway: true,
                has_ipv6_gateway: false,
                dns_servers: vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))],
            },
        ];

        let selected = select_best_dns_server(&adapters);
        assert_eq!(selected, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
    }

    #[test]
    fn test_select_best_dns_server_ipv6_dns() {
        use std::net::Ipv6Addr;

        let adapters = vec![DetectedAdapter {
            name: "Ethernet".to_string(),
            description: "Intel Gigabit Ethernet".to_string(),
            is_physical: true,
            is_operational: true,
            has_ipv4_gateway: false,
            has_ipv6_gateway: true,
            dns_servers: vec![IpAddr::V6(
                "2001:4860:4860::8888".parse::<Ipv6Addr>().unwrap(),
            )],
        }];

        let selected = select_best_dns_server(&adapters);
        assert_eq!(
            selected,
            Some(IpAddr::V6(
                "2001:4860:4860::8888".parse::<Ipv6Addr>().unwrap()
            ))
        );
    }

    #[test]
    fn test_select_best_dns_server_no_usable_dns() {
        use std::net::Ipv4Addr;

        let adapters = vec![DetectedAdapter {
            name: "Loopback".to_string(),
            description: "Loopback Adapter".to_string(),
            is_physical: false,
            is_operational: true,
            has_ipv4_gateway: false,
            has_ipv6_gateway: false,
            dns_servers: vec![
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            ],
        }];

        assert_eq!(select_best_dns_server(&adapters), None);
    }

    #[test]
    fn test_resolve_dns_address_preserves_explicit_preference() {
        assert_eq!(resolve_dns_address(Some("8.8.4.4")), "8.8.4.4");
        assert_eq!(resolve_dns_address(Some("  1.1.1.2  ")), "1.1.1.2");
    }

    #[test]
    fn test_build_route_exclude_addresses_includes_local_dns() {
        let excludes = build_route_exclude_addresses("192.168.1.1", &[]);
        assert!(
            excludes.contains(&"192.168.1.1/32".to_string()),
            "Excludes must contain detected /32 IPv4 DNS"
        );

        let excludes_v6 = build_route_exclude_addresses("2001:db8::53", &[]);
        assert!(
            excludes_v6.contains(&"2001:db8::53/128".to_string()),
            "Excludes must contain detected /128 IPv6 DNS"
        );
    }
}
