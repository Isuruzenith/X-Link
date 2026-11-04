use super::adapters::SingBoxOutbound;
use super::build_route_exclude_addresses;
use super::resolve_dns_address;
use super::resolve_server_ips;
use serde_json::Value;
use std::collections::HashMap;

fn split_host_port(addr: &str, default_port: u16) -> (String, u16) {
    let addr = addr.trim();
    // Bracketed IPv6: [2402:4000::2] or [2402:4000::2]:53
    if addr.starts_with('[') {
        if let Some(close_idx) = addr.find(']') {
            let host = &addr[1..close_idx];
            let rest = &addr[close_idx + 1..];
            let port = if let Some(colon_idx) = rest.find(':') {
                rest[colon_idx + 1..].parse::<u16>().unwrap_or(default_port)
            } else {
                default_port
            };
            return (host.to_string(), port);
        }
    }

    // Bare IP address (IPv4 or IPv6 e.g. "2402:4000::2" or "8.8.8.8")
    if let Ok(ip) = addr.parse::<std::net::IpAddr>() {
        return (ip.to_string(), default_port);
    }

    // Single colon means host:port (e.g. "8.8.8.8:5353" or "dns.google:53")
    if addr.matches(':').count() == 1 {
        if let Some(colon_idx) = addr.rfind(':') {
            let (h, p_str) = addr.split_at(colon_idx);
            if let Ok(p) = p_str[1..].parse::<u16>() {
                return (h.to_string(), p);
            }
        }
    }

    (addr.to_string(), default_port)
}

pub fn parse_dns_server(tag: &str, address: &str, detour: Option<&str>) -> serde_json::Value {
    let address = address.trim();
    let mut server_obj = serde_json::Map::new();
    server_obj.insert("tag".to_string(), serde_json::json!(tag));
    if let Some(d) = detour {
        if !d.is_empty() {
            server_obj.insert("detour".to_string(), serde_json::json!(d));
        }
    }

    if address == "local" || address.is_empty() {
        server_obj.insert("type".to_string(), serde_json::json!("local"));
    } else if address.starts_with("https://") {
        server_obj.insert("type".to_string(), serde_json::json!("https"));
        server_obj.insert("server".to_string(), serde_json::json!(address));
    } else if address.starts_with("tls://") {
        server_obj.insert("type".to_string(), serde_json::json!("tls"));
        let stripped = address.strip_prefix("tls://").unwrap();
        let (host, port) = split_host_port(stripped, 853);
        server_obj.insert("server".to_string(), serde_json::json!(host));
        server_obj.insert("server_port".to_string(), serde_json::json!(port));
    } else if address.starts_with("quic://") {
        server_obj.insert("type".to_string(), serde_json::json!("quic"));
        let stripped = address.strip_prefix("quic://").unwrap();
        let (host, port) = split_host_port(stripped, 853);
        server_obj.insert("server".to_string(), serde_json::json!(host));
        server_obj.insert("server_port".to_string(), serde_json::json!(port));
    } else if address.starts_with("tcp://") {
        server_obj.insert("type".to_string(), serde_json::json!("tcp"));
        let stripped = address.strip_prefix("tcp://").unwrap();
        let (host, port) = split_host_port(stripped, 53);
        server_obj.insert("server".to_string(), serde_json::json!(host));
        server_obj.insert("server_port".to_string(), serde_json::json!(port));
    } else {
        server_obj.insert("type".to_string(), serde_json::json!("udp"));
        let stripped = address.strip_prefix("udp://").unwrap_or(address);
        let (host, port) = split_host_port(stripped, 53);
        server_obj.insert("server".to_string(), serde_json::json!(host));
        server_obj.insert("server_port".to_string(), serde_json::json!(port));
    }

    serde_json::Value::Object(server_obj)
}

/// User-configurable TUN and sniffing settings read from settings.json.
#[derive(Debug, Clone)]
pub struct TunSettings {
    pub auto_route: bool,
    pub auto_redirect: bool,
    pub strict_route: bool,
    pub stack: String,
    pub mtu: u32,
    pub endpoint_independent_nat: bool,
    pub sniff_enabled: bool,
    pub sniff_http: bool,
    pub sniff_tls: bool,
    pub sniff_quic: bool,
    pub sniff_override_destination: bool,
    /// "normal" or "fakeip"
    pub dns_mode: String,
    pub fakeip_range: String,
    /// Remote DNS resolved through the proxy tunnel
    pub primary_dns: String,
    /// Secondary remote DNS, used only in "normal" DNS mode
    pub fallback_dns: String,
    /// Route RFC-1918 / .lan / .local traffic directly instead of through the proxy
    pub bypass_lan: bool,
    /// Outbound tag ("proxy" or "direct") used for unmatched traffic
    pub final_outbound: String,
    pub use_separate_ports: bool,
    pub http_port: u16,
    pub socks_port: u16,
    pub dns_caching: bool,
    pub dns_strategy: String,
    pub direct_dns: String,
    pub fakeip_filter: String,
    pub dns_leak_protection: bool,
    pub multiplex: bool,
}

impl Default for TunSettings {
    fn default() -> Self {
        Self {
            auto_route: true,
            auto_redirect: false,
            strict_route: true,
            stack: "gvisor".to_string(),
            mtu: 1500,
            endpoint_independent_nat: false,
            sniff_enabled: true,
            sniff_http: true,
            sniff_tls: true,
            sniff_quic: true,
            sniff_override_destination: false,
            dns_mode: "fakeip".to_string(),
            fakeip_range: "198.18.0.0/15".to_string(),
            primary_dns: "https://1.1.1.1/dns-query".to_string(),
            fallback_dns: "https://8.8.8.8/dns-query".to_string(),
            bypass_lan: true,
            final_outbound: "proxy".to_string(),
            use_separate_ports: false,
            http_port: 7890,
            socks_port: 7891,
            dns_caching: true,
            dns_strategy: "prefer_ipv4".to_string(),
            direct_dns: "8.8.8.8".to_string(),
            fakeip_filter: "geosite:private".to_string(),
            dns_leak_protection: true,
            multiplex: false,
        }
    }
}

pub fn should_include_tun_interface_name(target_os: &str) -> bool {
    target_os != "macos"
}

#[allow(clippy::too_many_arguments)]
pub fn generate_singbox_config(
    mixed_port: u16,
    mut outbounds: Vec<SingBoxOutbound>,
    proxy_mode: &str,
    dns_address: &str,
    listen_address: &str,
    tun_settings: &TunSettings,
    default_outbound_tag: Option<&str>,
    user_rules: &[crate::config::rules::RoutingRule],
    rule_sets: &[crate::config::rules::RuleSet],
    app_data_dir: &std::path::Path,
    outbound_groups: &[crate::config::rules::OutboundGroup],
    host_dns_servers: &[std::net::IpAddr],
) -> Result<String, String> {
    let check_and_cleanup_db = |path: &std::path::Path| -> bool {
        if path.exists() {
            if let Ok(metadata) = path.metadata() {
                if metadata.len() == 0 {
                    let _ = std::fs::remove_file(path);
                    return false;
                }
            }
            true
        } else {
            false
        }
    };

    let geosite_db_exists = check_and_cleanup_db(&app_data_dir.join("geosite.db"))
        || check_and_cleanup_db(std::path::Path::new("geosite.db"))
        || check_and_cleanup_db(std::path::Path::new("src-tauri/geosite.db"));
    let geoip_db_exists = check_and_cleanup_db(&app_data_dir.join("geoip.db"))
        || check_and_cleanup_db(std::path::Path::new("geoip.db"))
        || check_and_cleanup_db(std::path::Path::new("src-tauri/geoip.db"));

    // Strip dangling detour references gracefully before compilation
    crate::config::validator::normalize_detour_references(&mut outbounds, outbound_groups);

    let mut server_hosts = Vec::new();
    for outbound in &outbounds {
        if let Some(server_val) = outbound.fields.get("server") {
            if let Some(server_str) = server_val.as_str() {
                server_hosts.push(server_str.to_string());
            }
        }
    }
    let server_ips = resolve_server_ips(&server_hosts);

    let dialable_node_tags: Vec<String> = outbounds
        .iter()
        .filter(|n| {
            !matches!(
                n.outbound_type.as_str(),
                "direct" | "block" | "dns" | "selector" | "urltest" | "fallback"
            )
        })
        .map(|n| n.tag.clone())
        .collect();

    let all_node_tags: Vec<String> = outbounds.iter().map(|n| n.tag.clone()).collect();
    let valid_node_tag_set: std::collections::HashSet<&str> =
        all_node_tags.iter().map(|s| s.as_str()).collect();

    // Compile OutboundGroups
    let mut compiled_groups = Vec::new();
    let mut group_tags = Vec::new();

    for group in outbound_groups {
        let tag_trimmed = group.tag.trim();
        if tag_trimmed.is_empty()
            || crate::config::validator::RESERVED_OUTBOUND_TAGS
                .iter()
                .any(|r| r.eq_ignore_ascii_case(tag_trimmed))
            || valid_node_tag_set.contains(tag_trimmed)
            || group_tags.contains(&group.tag)
        {
            continue;
        }

        let mut valid_members: Vec<String> = group
            .nodes
            .iter()
            .filter(|n| valid_node_tag_set.contains(n.as_str()))
            .cloned()
            .collect();

        if matches!(
            group.r#type,
            crate::config::rules::OutboundGroupType::Urltest
                | crate::config::rules::OutboundGroupType::Fallback
        ) && valid_members.len() > crate::config::validator::MAX_AUTOMATED_GROUP_NODES
        {
            valid_members.truncate(crate::config::validator::MAX_AUTOMATED_GROUP_NODES);
        }

        group_tags.push(group.tag.clone());

        if valid_members.is_empty() {
            if dialable_node_tags.is_empty() {
                // No dialable nodes exist: emit as selector pointing to ["direct"], never ["proxy"]
                compiled_groups.push(serde_json::json!({
                    "type": "selector",
                    "tag": group.tag,
                    "outbounds": ["direct"]
                }));
                continue;
            } else {
                valid_members = match group.r#type {
                    crate::config::rules::OutboundGroupType::Urltest
                    | crate::config::rules::OutboundGroupType::Fallback => dialable_node_tags
                        .iter()
                        .take(crate::config::validator::MAX_AUTOMATED_GROUP_NODES)
                        .cloned()
                        .collect(),
                    crate::config::rules::OutboundGroupType::Selector => dialable_node_tags.clone(),
                };
            }
        }

        match group.r#type {
            crate::config::rules::OutboundGroupType::Urltest
            | crate::config::rules::OutboundGroupType::Fallback => {
                let url = group
                    .url
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("https://cp.cloudflare.com/generate_204");
                let interval = group
                    .interval
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("600s");
                let tolerance = group.tolerance.unwrap_or(50);

                compiled_groups.push(serde_json::json!({
                    "type": "urltest",
                    "tag": group.tag,
                    "outbounds": valid_members,
                    "url": url,
                    "interval": interval,
                    "tolerance": tolerance
                }));
            }
            crate::config::rules::OutboundGroupType::Selector => {
                compiled_groups.push(serde_json::json!({
                    "type": "selector",
                    "tag": group.tag,
                    "outbounds": valid_members
                }));
            }
        }
    }

    let mut selector_tags: Vec<String> = Vec::new();
    selector_tags.extend(group_tags);
    for nt in all_node_tags {
        if !selector_tags.contains(&nt) {
            selector_tags.push(nt);
        }
    }

    // Create the outbounds array
    let mut final_outbounds = Vec::new();
    let mut endpoints = Vec::new();

    // Fallback if no outbounds imported yet
    if selector_tags.is_empty() {
        selector_tags.push("direct".to_string());
    }

    // Assemble master selector outbound "proxy"
    let mut selector = HashMap::new();
    selector.insert("type".to_string(), Value::String("selector".to_string()));
    selector.insert("tag".to_string(), Value::String("proxy".to_string()));
    selector.insert(
        "outbounds".to_string(),
        Value::Array(selector_tags.iter().cloned().map(Value::String).collect()),
    );
    if let Some(def_tag) = default_outbound_tag {
        if selector_tags.iter().any(|t| t == def_tag) {
            selector.insert("default".to_string(), Value::String(def_tag.to_string()));
        }
    }

    // First, push our root selector outbound
    final_outbounds.push(Value::Object(selector.into_iter().collect()));

    // Next, push user-defined OutboundGroup outbounds
    final_outbounds.extend(compiled_groups);

    // Push the parsed node outbounds
    for node in outbounds {
        if node.outbound_type == "wireguard" {
            let address = if let Some(addr) = node
                .fields
                .get("address")
                .or_else(|| node.fields.get("local_address"))
            {
                if addr.is_array() {
                    addr.clone()
                } else if let Some(s) = addr.as_str() {
                    serde_json::json!([s])
                } else {
                    serde_json::json!(["10.0.0.2/32"])
                }
            } else {
                serde_json::json!(["10.0.0.2/32"])
            };

            let private_key = node
                .fields
                .get("private_key")
                .or_else(|| node.fields.get("secret_key"))
                .and_then(|k| k.as_str())
                .unwrap_or("")
                .to_string();

            let server_address = node
                .fields
                .get("server")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();

            let server_port = node
                .fields
                .get("server_port")
                .and_then(|p| p.as_u64())
                .unwrap_or(51820) as u16;

            let peer_public_key = node
                .fields
                .get("peer_public_key")
                .or_else(|| node.fields.get("public_key"))
                .and_then(|k| k.as_str())
                .unwrap_or("")
                .to_string();

            let allowed_ips = if let Some(ips) = node.fields.get("allowed_ips") {
                if ips.is_array() {
                    ips.clone()
                } else if let Some(s) = ips.as_str() {
                    serde_json::json!([s])
                } else {
                    serde_json::json!(["0.0.0.0/0", "::/0"])
                }
            } else {
                serde_json::json!(["0.0.0.0/0", "::/0"])
            };

            let mut peer_obj = serde_json::json!({
                "address": server_address,
                "port": server_port,
                "public_key": peer_public_key,
                "allowed_ips": allowed_ips
            });

            if let Some(reserved) = node.fields.get("reserved") {
                peer_obj["reserved"] = reserved.clone();
            }

            let mut endpoint_obj = serde_json::json!({
                "type": "wireguard",
                "tag": node.tag.clone(),
                "address": address,
                "private_key": private_key,
                "peers": [peer_obj]
            });

            if let Some(mtu) = node.fields.get("mtu").and_then(|m| m.as_u64()) {
                endpoint_obj["mtu"] = serde_json::json!(mtu);
            }

            endpoints.push(endpoint_obj);
            continue;
        }

        let mut obj = serde_json::to_value(&node)
            .map_err(|e| format!("Failed to serialize node: {}", e))?
            .as_object()
            .unwrap()
            .clone();

        // Ensure type and tag are correct
        obj.insert("tag".to_string(), Value::String(node.tag.clone()));

        if tun_settings.multiplex {
            let outbound_type = node.outbound_type.as_str();
            if outbound_type == "vless" || outbound_type == "vmess" || outbound_type == "trojan" {
                obj.insert(
                    "multiplex".to_string(),
                    serde_json::json!({
                        "enabled": true,
                        "protocol": "smux",
                        "max_connections": 4,
                        "min_streams": 4,
                        "max_streams": 32,
                        "padding": true
                    }),
                );
            }
        }

        // If TLS is enabled, ensure a default ALPN is specified if missing (default to "http/1.1")
        if let Some(tls) = obj.get_mut("tls").and_then(|t| t.as_object_mut()) {
            let enabled = tls
                .get("enabled")
                .and_then(|e| e.as_bool())
                .unwrap_or(false);
            if enabled && tls.get("alpn").is_none() {
                tls.insert("alpn".to_string(), serde_json::json!(["http/1.1"]));
            }
        }

        // WebSocket and HTTPUpgrade handshakes require HTTP/1.1; normalize imported
        // HTTPUpgrade Host headers so existing profiles generate the correct request host.
        let transport_type = obj
            .get("transport")
            .and_then(|t| t.get("type"))
            .and_then(|t| t.as_str())
            .map(str::to_string);
        if matches!(transport_type.as_deref(), Some("ws" | "httpupgrade")) {
            if let Some(tls) = obj.get_mut("tls").and_then(|t| t.as_object_mut()) {
                tls.insert("alpn".to_string(), serde_json::json!(["http/1.1"]));
            }
        }
        if transport_type.as_deref() == Some("httpupgrade") {
            if let Some(transport) = obj.get_mut("transport").and_then(|t| t.as_object_mut()) {
                let host = transport
                    .get_mut("headers")
                    .and_then(|headers| headers.as_object_mut())
                    .and_then(|headers| {
                        let host = headers.remove("Host").or_else(|| headers.remove("host"));
                        headers.remove("host");
                        host
                    });
                if let Some(host) = host {
                    transport.entry("host".to_string()).or_insert(host);
                }
                if transport
                    .get("headers")
                    .and_then(|h| h.as_object())
                    .is_some_and(|h| h.is_empty())
                {
                    transport.remove("headers");
                }
            }
        }

        // Inject TCP Keep-Alive settings at the root level of dialing outbounds (as supported in sing-box v1.13.14)
        let outbound_type = node.outbound_type.as_str();
        if outbound_type != "selector" && outbound_type != "block" && outbound_type != "dns" {
            obj.insert("tcp_keep_alive".to_string(), serde_json::json!("30s"));
            obj.insert(
                "tcp_keep_alive_interval".to_string(),
                serde_json::json!("15s"),
            );
        }

        final_outbounds.push(Value::Object(obj));
    }

    // Unconditionally push the standard direct outbound (required for dns detour)
    let direct = serde_json::json!({
        "type": "direct",
        "tag": "direct",
        "tcp_keep_alive": "30s",
        "tcp_keep_alive_interval": "15s"
    });
    final_outbounds.push(direct);

    let resolved_dns_address = resolve_dns_address(Some(dns_address));

    // Construct the dynamic inbounds array, including host_dns_servers in route_exclude_addresses
    let mut route_exclude_addresses =
        build_route_exclude_addresses(&resolved_dns_address, &server_ips);
    for dns_ip in host_dns_servers {
        let cidr = match dns_ip {
            std::net::IpAddr::V4(v4) => format!("{}/32", v4),
            std::net::IpAddr::V6(v6) => format!("{}/128", v6),
        };
        if !route_exclude_addresses.contains(&cidr) {
            route_exclude_addresses.push(cidr);
        }
    }

    let mut inbounds = Vec::new();
    if tun_settings.use_separate_ports {
        inbounds.push(serde_json::json!({
            "type": "http",
            "tag": "http-in",
            "listen": listen_address,
            "listen_port": tun_settings.http_port
        }));
        inbounds.push(serde_json::json!({
            "type": "socks",
            "tag": "socks-in",
            "listen": listen_address,
            "listen_port": tun_settings.socks_port
        }));
    } else {
        inbounds.push(serde_json::json!({
            "type": "mixed",
            "tag": "mixed-in",
            "listen": listen_address,
            "listen_port": mixed_port
        }));
    }

    if proxy_mode == "tun" {
        let mut tun_inbound = serde_json::json!({
            "type": "tun",
            "tag": "tun-in",
            "address": [
                "172.19.0.1/30",
                "fdfe:dcba:9876::1/126"
            ],
            "auto_route": tun_settings.auto_route,
            "strict_route": tun_settings.strict_route,
            "stack": tun_settings.stack,
            "route_exclude_address": route_exclude_addresses,
        });
        if should_include_tun_interface_name(std::env::consts::OS) {
            tun_inbound["interface_name"] = serde_json::json!("X-Link");
        }
        if tun_settings.mtu != 0 {
            let safe_mtu = if tun_settings.mtu == 1500 {
                1400
            } else {
                tun_settings.mtu
            };
            tun_inbound["mtu"] = serde_json::json!(safe_mtu);
        }
        if tun_settings.auto_redirect {
            tun_inbound["auto_redirect"] = serde_json::json!(true);
        }
        if tun_settings.endpoint_independent_nat {
            tun_inbound["endpoint_independent_nat"] = serde_json::json!(true);
        }
        inbounds.push(tun_inbound);
    }

    // Build route rules incorporating user routing rules from routing.json
    let route_rules = crate::config::build_route_rules(
        tun_settings.bypass_lan,
        tun_settings.sniff_enabled,
        user_rules,
        rule_sets,
        geosite_db_exists,
        geoip_db_exists,
        outbound_groups,
    );

    let mut route_section = serde_json::json!({
        "rules": route_rules,
        "final": tun_settings.final_outbound,
        "auto_detect_interface": true,
        "default_domain_resolver": "local-dns"
    });

    if !rule_sets.is_empty() {
        let mut singbox_rule_sets = Vec::new();
        for rs in rule_sets {
            let mut set_val = serde_json::json!({
                "tag": rs.tag,
                "type": rs.set_type,
                "format": rs.format,
            });
            if rs.set_type == "remote" {
                if let Some(ref url) = rs.url {
                    set_val["url"] = serde_json::json!(url);
                }
                if !rs.update_interval.trim().is_empty() {
                    set_val["update_interval"] = serde_json::json!(rs.update_interval);
                }
                // Detour remote rule-set downloads through the proxy outbound to support zero-package carriers
                set_val["download_detour"] = serde_json::json!("proxy");
            } else if rs.set_type == "local" {
                if let Some(ref path) = rs.file_path {
                    set_val["path"] = serde_json::json!(path);
                }
            }
            singbox_rule_sets.push(set_val);
        }
        route_section["rule_set"] = serde_json::json!(singbox_rule_sets);
    }

    let mut dns_rules = Vec::new();

    let server_domains: Vec<String> = server_hosts
        .iter()
        .filter(|h| h.trim().parse::<std::net::IpAddr>().is_err())
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .collect();
    if !server_domains.is_empty() {
        dns_rules.insert(
            0,
            serde_json::json!({
                "domain": server_domains,
                "server": "local-dns"
            }),
        );
    }

    let is_fakeip = tun_settings.dns_mode == "fakeip";
    let local_dns_addr = if tun_settings.direct_dns.trim().is_empty() {
        "local".to_string()
    } else {
        tun_settings.direct_dns.clone()
    };

    let mut dns_servers = vec![
        parse_dns_server("local-dns", &local_dns_addr, Some("direct")),
        parse_dns_server("remote-dns", &tun_settings.primary_dns, Some("proxy")),
    ];
    if !is_fakeip && !tun_settings.fallback_dns.trim().is_empty() {
        dns_servers.push(parse_dns_server(
            "remote-dns-fallback",
            &tun_settings.fallback_dns,
            Some("proxy"),
        ));
    }
    if is_fakeip {
        if !tun_settings.fakeip_filter.trim().is_empty() {
            let filters: Vec<String> = tun_settings
                .fakeip_filter
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !filters.is_empty() {
                let mut rule = serde_json::json!({
                    "server": "local-dns"
                });

                let mut rule_sets = Vec::new();
                let mut domain = Vec::new();
                let mut domain_suffix = Vec::new();
                let mut domain_keyword = Vec::new();

                for f in filters {
                    if f.starts_with("geosite:") {
                        let category = f.strip_prefix("geosite:").unwrap().trim().to_string();
                        if category == "private" {
                            for suffix in
                                &[".lan", ".local", ".internal", ".home.arpa", "localhost"]
                            {
                                domain_suffix.push(suffix.to_string());
                            }
                        } else {
                            rule_sets.push(format!("geosite-{}", category));
                        }
                    } else if f.starts_with("domain:") {
                        domain.push(f.strip_prefix("domain:").unwrap().trim().to_string());
                    } else if f.starts_with("keyword:") {
                        domain_keyword.push(f.strip_prefix("keyword:").unwrap().trim().to_string());
                    } else {
                        domain_suffix.push(f);
                    }
                }

                let mut has_matcher = false;
                if !rule_sets.is_empty() {
                    rule["rule_set"] = serde_json::json!(rule_sets);
                    has_matcher = true;
                }
                if !domain.is_empty() {
                    rule["domain"] = serde_json::json!(domain);
                    has_matcher = true;
                }
                if !domain_suffix.is_empty() {
                    rule["domain_suffix"] = serde_json::json!(domain_suffix);
                    has_matcher = true;
                }
                if !domain_keyword.is_empty() {
                    rule["domain_keyword"] = serde_json::json!(domain_keyword);
                    has_matcher = true;
                }

                if has_matcher {
                    dns_rules.push(rule);
                }
            }
        }

        // Add the fakeip DNS server in type-based format (sing-box 1.12.0+ standard)
        dns_servers.push(serde_json::json!({
            "tag": "fakeip-dns",
            "type": "fakeip",
            "inet4_range": tun_settings.fakeip_range,
            "inet6_range": "fc00::/18"
        }));
        dns_rules.push(serde_json::json!({
            "query_type": ["A", "AAAA"],
            "server": "fakeip-dns"
        }));
    }

    let dns_section = if proxy_mode == "tun" {
        let mut section = serde_json::json!({
            "servers": dns_servers,
            "rules": dns_rules,
            "strategy": tun_settings.dns_strategy,
            "final": "remote-dns"
        });
        section["disable_cache"] = serde_json::json!(!tun_settings.dns_caching);
        section
    } else {
        let mut section = serde_json::json!({
            "servers": [
                parse_dns_server("local-dns", &local_dns_addr, Some("direct"))
            ],
            "rules": dns_rules
        });
        section["disable_cache"] = serde_json::json!(!tun_settings.dns_caching);
        section
    };

    // Construct the complete sing-box configuration
    let mut config = serde_json::json!({
        "log": {
            "level": "debug",
            "timestamp": true
        },
        "dns": dns_section,
        "inbounds": inbounds,
        "outbounds": final_outbounds,
        "route": route_section
    });

    if !endpoints.is_empty() {
        config["endpoints"] = serde_json::Value::Array(endpoints);
    }

    serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to generate pretty JSON config: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_singbox_config_listen_addresses() {
        let outbounds = vec![super::super::adapters::SingBoxOutbound {
            outbound_type: "direct".to_string(),
            tag: "direct".to_string(),
            fields: std::collections::HashMap::new(),
        }];

        let tun = TunSettings::default();

        // Case 1: Wifi sharing is disabled -> binds to 127.0.0.1
        let config_str_local = generate_singbox_config(
            7890,
            outbounds.clone(),
            "system",
            "1.1.1.1",
            "127.0.0.1",
            &tun,
            None,
            &[],
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .unwrap();
        let config_local: serde_json::Value = serde_json::from_str(&config_str_local).unwrap();
        let inbounds_local = config_local["inbounds"].as_array().unwrap();
        assert!(!inbounds_local.is_empty());
        let mixed_inbound_local = &inbounds_local[0];
        assert_eq!(mixed_inbound_local["listen"].as_str().unwrap(), "127.0.0.1");
        assert_eq!(mixed_inbound_local["listen_port"].as_u64().unwrap(), 7890);

        // Case 2: Wifi sharing is enabled -> binds to 0.0.0.0
        let config_str_wifi = generate_singbox_config(
            7890,
            outbounds.clone(),
            "system",
            "1.1.1.1",
            "0.0.0.0",
            &tun,
            None,
            &[],
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .unwrap();
        let config_wifi: serde_json::Value = serde_json::from_str(&config_str_wifi).unwrap();
        let inbounds_wifi = config_wifi["inbounds"].as_array().unwrap();
        assert!(!inbounds_wifi.is_empty());
        let mixed_inbound_wifi = &inbounds_wifi[0];
        assert_eq!(mixed_inbound_wifi["listen"].as_str().unwrap(), "0.0.0.0");
        assert_eq!(mixed_inbound_wifi["listen_port"].as_u64().unwrap(), 7890);
    }

    #[test]
    fn test_generate_singbox_config_alpn_self_healing() {
        let mut fields = std::collections::HashMap::new();
        fields.insert(
            "server".to_string(),
            serde_json::json!("azure.ezgateway.net"),
        );
        fields.insert("server_port".to_string(), serde_json::json!(443));
        fields.insert(
            "uuid".to_string(),
            serde_json::json!("88dacb71-7530-475b-9ed6-a431caef6b3f"),
        );
        fields.insert(
            "tls".to_string(),
            serde_json::json!({
                "enabled": true,
                "insecure": true,
                "server_name": "aka.ms",
                "alpn": ["h2", "http/1.1"]
            }),
        );
        fields.insert(
            "transport".to_string(),
            serde_json::json!({
                "type": "ws",
                "path": "/azure",
                "headers": {
                    "Host": "azure.ezgateway.net"
                }
            }),
        );

        let outbounds = vec![super::super::adapters::SingBoxOutbound {
            outbound_type: "vless".to_string(),
            tag: "Zoom-SG-Kavishka-300GB".to_string(),
            fields,
        }];

        let tun = TunSettings::default();
        let config_str = generate_singbox_config(
            7892,
            outbounds,
            "tun",
            "172.20.10.1",
            "127.0.0.1",
            &tun,
            None,
            &[],
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .unwrap();

        let config: serde_json::Value = serde_json::from_str(&config_str).unwrap();
        let outbounds_arr = config["outbounds"].as_array().unwrap();

        // Find our node outbound
        let node_outbound = outbounds_arr
            .iter()
            .find(|o| o["tag"].as_str() == Some("Zoom-SG-Kavishka-300GB"))
            .expect("Should find our node outbound");

        let alpn = node_outbound["tls"]["alpn"].as_array().unwrap();
        // Check that "h2" has been stripped
        assert!(alpn.iter().all(|v| v.as_str() != Some("h2")));
        assert!(alpn.iter().any(|v| v.as_str() == Some("http/1.1")));
    }

    #[test]
    fn test_generate_singbox_config_httpupgrade_host_and_alpn() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("server".to_string(), serde_json::json!("proxy.example"));
        fields.insert("server_port".to_string(), serde_json::json!(443));
        fields.insert(
            "uuid".to_string(),
            serde_json::json!("00000000-0000-0000-0000-000000000000"),
        );
        fields.insert(
            "tls".to_string(),
            serde_json::json!({
                "enabled": true,
                "server_name": "tls.example",
                "alpn": ["h2", "http/1.1"]
            }),
        );
        fields.insert(
            "transport".to_string(),
            serde_json::json!({
                "type": "httpupgrade",
                "path": "/azure",
                "headers": { "Host": "cdn.example", "X-Custom": "value" }
            }),
        );

        let outbounds = vec![super::super::adapters::SingBoxOutbound {
            outbound_type: "vless".to_string(),
            tag: "HTTPUpgrade Node".to_string(),
            fields,
        }];
        let config_str = generate_singbox_config(
            7892,
            outbounds,
            "system",
            "1.1.1.1",
            "127.0.0.1",
            &TunSettings::default(),
            None,
            &[],
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .unwrap();
        let config: serde_json::Value = serde_json::from_str(&config_str).unwrap();
        let node = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["tag"] == "HTTPUpgrade Node")
            .unwrap();
        assert_eq!(node["tls"]["alpn"], serde_json::json!(["http/1.1"]));
        assert!(node["tls"].get("insecure").is_none());
        assert_eq!(node["transport"]["host"], "cdn.example");
        assert_eq!(node["transport"]["headers"]["X-Custom"], "value");
        assert!(node["transport"]["headers"].get("Host").is_none());
    }

    #[test]
    fn test_generate_singbox_config_httpupgrade_preserves_explicit_host() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("server".to_string(), serde_json::json!("proxy.example"));
        fields.insert("server_port".to_string(), serde_json::json!(443));
        fields.insert("tls".to_string(), serde_json::json!({ "enabled": true }));
        fields.insert(
            "transport".to_string(),
            serde_json::json!({
                "type": "httpupgrade",
                "host": "explicit.example",
                "headers": { "Host": "legacy.example" }
            }),
        );
        let outbounds = vec![super::super::adapters::SingBoxOutbound {
            outbound_type: "vless".to_string(),
            tag: "HTTPUpgrade Node".to_string(),
            fields,
        }];
        let config_str = generate_singbox_config(
            7892,
            outbounds,
            "system",
            "1.1.1.1",
            "127.0.0.1",
            &TunSettings::default(),
            None,
            &[],
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .unwrap();
        let config: serde_json::Value = serde_json::from_str(&config_str).unwrap();
        let node = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["tag"] == "HTTPUpgrade Node")
            .unwrap();
        assert_eq!(node["transport"]["host"], "explicit.example");
        assert!(node["transport"].get("headers").is_none());
        assert_eq!(node["tls"]["alpn"], serde_json::json!(["http/1.1"]));
    }

    #[test]
    fn test_generate_singbox_config_with_rule_sets() {
        let outbounds = vec![super::super::adapters::SingBoxOutbound {
            outbound_type: "direct".to_string(),
            tag: "direct".to_string(),
            fields: std::collections::HashMap::new(),
        }];

        let tun = TunSettings::default();
        let rule_sets = vec![
            crate::config::rules::RuleSet {
                id: "rs1".into(),
                tag: "geosite-youtube".into(),
                set_type: "remote".into(),
                format: "binary".into(),
                url: Some("https://example.com/youtube.srs".into()),
                file_path: None,
                update_interval: "1d".into(),
            },
            crate::config::rules::RuleSet {
                id: "rs2".into(),
                tag: "local-blocklist".into(),
                set_type: "local".into(),
                format: "source".into(),
                url: None,
                file_path: Some("C:/rules.json".into()),
                update_interval: "".into(),
            },
        ];

        let config_str = generate_singbox_config(
            7890,
            outbounds,
            "system",
            "1.1.1.1",
            "127.0.0.1",
            &tun,
            None,
            &[],
            &rule_sets,
            std::path::Path::new(""),
            &[],
            &[],
        )
        .unwrap();

        let config: serde_json::Value = serde_json::from_str(&config_str).unwrap();
        let rule_sets_arr = config["route"]["rule_set"].as_array().unwrap();
        assert_eq!(rule_sets_arr.len(), 2);

        let rs1 = &rule_sets_arr[0];
        assert_eq!(rs1["tag"].as_str().unwrap(), "geosite-youtube");
        assert_eq!(rs1["type"].as_str().unwrap(), "remote");
        assert_eq!(rs1["format"].as_str().unwrap(), "binary");
        assert_eq!(
            rs1["url"].as_str().unwrap(),
            "https://example.com/youtube.srs"
        );
        assert_eq!(rs1["update_interval"].as_str().unwrap(), "1d");
        assert_eq!(rs1["download_detour"].as_str().unwrap(), "proxy");

        let rs2 = &rule_sets_arr[1];
        assert_eq!(rs2["tag"].as_str().unwrap(), "local-blocklist");
        assert_eq!(rs2["type"].as_str().unwrap(), "local");
        assert_eq!(rs2["format"].as_str().unwrap(), "source");
        assert_eq!(rs2["path"].as_str().unwrap(), "C:/rules.json");
    }

    #[test]
    fn test_generate_singbox_config_multiplex() {
        let outbounds = vec![super::super::adapters::SingBoxOutbound {
            outbound_type: "vless".to_string(),
            tag: "Zoom-SG".to_string(),
            fields: std::collections::HashMap::new(),
        }];

        let tun = TunSettings {
            multiplex: true,
            ..TunSettings::default()
        };

        let config_str = generate_singbox_config(
            7890,
            outbounds,
            "system",
            "1.1.1.1",
            "127.0.0.1",
            &tun,
            None,
            &[],
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .unwrap();

        let config: serde_json::Value = serde_json::from_str(&config_str).unwrap();
        let outbounds_arr = config["outbounds"].as_array().unwrap();

        let vless_outbound = outbounds_arr
            .iter()
            .find(|o| o["tag"].as_str() == Some("Zoom-SG"))
            .expect("Should find VLESS outbound");

        let multiplex = &vless_outbound["multiplex"];
        assert!(multiplex["enabled"].as_bool().unwrap());
        assert_eq!(multiplex["protocol"].as_str().unwrap(), "smux");
        assert_eq!(multiplex["max_connections"].as_u64().unwrap(), 4);

        assert_eq!(vless_outbound["tcp_keep_alive"].as_str().unwrap(), "30s");
        assert_eq!(
            vless_outbound["tcp_keep_alive_interval"].as_str().unwrap(),
            "15s"
        );
    }

    #[test]
    fn test_generate_singbox_config_wireguard_endpoint_and_ssh() {
        let mut wg_fields = std::collections::HashMap::new();
        wg_fields.insert("server".to_string(), serde_json::json!("1.2.3.4"));
        wg_fields.insert("server_port".to_string(), serde_json::json!(51820));
        wg_fields.insert("private_key".to_string(), serde_json::json!("privkey=="));
        wg_fields.insert("peer_public_key".to_string(), serde_json::json!("pubkey=="));
        wg_fields.insert("address".to_string(), serde_json::json!(["10.0.0.2/32"]));

        let mut ssh_fields = std::collections::HashMap::new();
        ssh_fields.insert("server".to_string(), serde_json::json!("5.6.7.8"));
        ssh_fields.insert("server_port".to_string(), serde_json::json!(22));
        ssh_fields.insert("user".to_string(), serde_json::json!("root"));
        ssh_fields.insert("password".to_string(), serde_json::json!("pass123"));

        let outbounds = vec![
            super::super::adapters::SingBoxOutbound {
                outbound_type: "wireguard".to_string(),
                tag: "wg-node".to_string(),
                fields: wg_fields,
            },
            super::super::adapters::SingBoxOutbound {
                outbound_type: "ssh".to_string(),
                tag: "ssh-node".to_string(),
                fields: ssh_fields,
            },
        ];

        let tun = TunSettings::default();
        let config_str = generate_singbox_config(
            7890,
            outbounds,
            "system",
            "1.1.1.1",
            "127.0.0.1",
            &tun,
            None,
            &[],
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .unwrap();

        let config: serde_json::Value = serde_json::from_str(&config_str).unwrap();

        // 1. Selector outbound must include both wg-node and ssh-node
        let selector = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["type"].as_str() == Some("selector"))
            .unwrap();
        let selector_outbounds = selector["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(selector_outbounds.contains(&"wg-node"));
        assert!(selector_outbounds.contains(&"ssh-node"));

        // 2. WireGuard must NOT be in outbounds
        let wg_in_outbounds = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["type"].as_str() == Some("wireguard"));
        assert!(!wg_in_outbounds);

        // 3. WireGuard must be in endpoints
        let endpoints = config["endpoints"]
            .as_array()
            .expect("Endpoints must exist");
        assert_eq!(endpoints.len(), 1);
        let ep = &endpoints[0];
        assert_eq!(ep["type"].as_str().unwrap(), "wireguard");
        assert_eq!(ep["tag"].as_str().unwrap(), "wg-node");
        assert_eq!(ep["address"][0].as_str().unwrap(), "10.0.0.2/32");
        assert_eq!(ep["peers"][0]["address"].as_str().unwrap(), "1.2.3.4");

        // 4. SSH must be in outbounds
        let ssh_in_outbounds = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["tag"].as_str() == Some("ssh-node"))
            .expect("SSH outbound must exist");
        assert_eq!(ssh_in_outbounds["type"].as_str().unwrap(), "ssh");
        assert_eq!(ssh_in_outbounds["server"].as_str().unwrap(), "5.6.7.8");
    }

    #[test]
    fn test_generate_singbox_config_empty_outbounds_creates_valid_fallback() {
        let tun = TunSettings::default();
        let config_str = generate_singbox_config(
            7890,
            Vec::new(),
            "system",
            "1.1.1.1",
            "127.0.0.1",
            &tun,
            None,
            &[],
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .unwrap();
        let config: serde_json::Value = serde_json::from_str(&config_str).unwrap();
        let outbounds = config["outbounds"].as_array().unwrap();
        let selector = outbounds.iter().find(|o| o["type"] == "selector").unwrap();
        assert_eq!(
            selector["outbounds"].as_array().unwrap(),
            &vec![serde_json::json!("direct")]
        );
    }

    #[test]
    fn test_outbound_groups_compilation_and_pruning_fallback() {
        use crate::config::rules::{OutboundGroup, OutboundGroupType};

        let outbounds: Vec<super::super::adapters::SingBoxOutbound> = (1..=12)
            .map(|i| super::super::adapters::SingBoxOutbound {
                outbound_type: "vless".to_string(),
                tag: format!("node-{}", i),
                fields: std::collections::HashMap::new(),
            })
            .collect();

        let groups = vec![
            OutboundGroup {
                id: "g1".into(),
                tag: "Auto-Fastest".into(),
                r#type: OutboundGroupType::Urltest,
                nodes: vec!["node-1".into(), "node-2".into(), "deleted-node".into()],
                url: None,
                interval: None,
                tolerance: Some(50),
            },
            OutboundGroup {
                id: "g2".into(),
                tag: "Pruned-Pool".into(),
                r#type: OutboundGroupType::Fallback,
                nodes: vec!["non-existent".into()],
                url: None,
                interval: Some("300s".into()),
                tolerance: None,
            },
        ];

        let config_str = generate_singbox_config(
            7890,
            outbounds,
            "tun",
            "1.1.1.1",
            "127.0.0.1",
            &TunSettings::default(),
            Some("Auto-Fastest"),
            &[],
            &[],
            std::path::Path::new(""),
            &groups,
            &["9.9.9.9".parse().unwrap()],
        )
        .unwrap();

        let config: serde_json::Value = serde_json::from_str(&config_str).unwrap();
        assert_eq!(config["log"]["level"], "debug");

        let outbounds_arr = config["outbounds"].as_array().unwrap();
        let root_proxy = outbounds_arr.iter().find(|o| o["tag"] == "proxy").unwrap();
        assert_eq!(root_proxy["default"], "Auto-Fastest");
        let root_members = root_proxy["outbounds"].as_array().unwrap();
        assert_eq!(root_members[0], "Auto-Fastest");
        assert_eq!(root_members[1], "Pruned-Pool");

        let auto_group = outbounds_arr
            .iter()
            .find(|o| o["tag"] == "Auto-Fastest")
            .unwrap();
        assert_eq!(auto_group["type"], "urltest");
        assert_eq!(auto_group["interval"], "600s");
        assert_eq!(
            auto_group["outbounds"],
            serde_json::json!(["node-1", "node-2"])
        );

        // Pruned-Pool had all members pruned, so it falls back to the first 10 dialable nodes
        let pruned_group = outbounds_arr
            .iter()
            .find(|o| o["tag"] == "Pruned-Pool")
            .unwrap();
        assert_eq!(pruned_group["type"], "urltest");
        assert_eq!(pruned_group["outbounds"].as_array().unwrap().len(), 10);

        // Verify host DNS server was added to TUN route_exclude_address
        let tun_in = config["inbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["type"] == "tun")
            .unwrap();
        let excludes = tun_in["route_exclude_address"].as_array().unwrap();
        assert!(excludes.contains(&serde_json::json!("9.9.9.9/32")));
    }

    #[test]
    fn test_macos_dynamic_utun_interface_omission() {
        assert!(!should_include_tun_interface_name("macos"));
        assert!(should_include_tun_interface_name("windows"));
        assert!(should_include_tun_interface_name("linux"));
    }

    #[test]
    fn test_parse_dns_server_ipv6_and_bracketed() {
        // Bare IPv6 address must not be truncated at the first colon
        let parsed_bare_ipv6 = parse_dns_server("local-dns", "2402:4000::2", Some("direct"));
        assert_eq!(parsed_bare_ipv6["type"], "udp");
        assert_eq!(parsed_bare_ipv6["server"], "2402:4000::2");
        assert_eq!(parsed_bare_ipv6["server_port"], 53);
        assert_eq!(parsed_bare_ipv6["detour"], "direct");

        // Bracketed IPv6 with custom port
        let parsed_bracketed =
            parse_dns_server("custom-dns", "[2402:4000::2]:5353", Some("direct"));
        assert_eq!(parsed_bracketed["type"], "udp");
        assert_eq!(parsed_bracketed["server"], "2402:4000::2");
        assert_eq!(parsed_bracketed["server_port"], 5353);

        // Bracketed IPv6 without port
        let parsed_bracketed_no_port =
            parse_dns_server("custom-dns", "[2001:4860:4860::8888]", None);
        assert_eq!(parsed_bracketed_no_port["type"], "udp");
        assert_eq!(parsed_bracketed_no_port["server"], "2001:4860:4860::8888");
        assert_eq!(parsed_bracketed_no_port["server_port"], 53);
    }

    #[test]
    fn test_parse_dns_server_local_and_schemes() {
        let parsed_local = parse_dns_server("local-dns", "local", Some("direct"));
        assert_eq!(parsed_local["type"], "local");
        assert_eq!(parsed_local["tag"], "local-dns");
        assert_eq!(parsed_local["detour"], "direct");
        assert!(parsed_local.get("server").is_none());

        let parsed_empty = parse_dns_server("local-dns", "", None);
        assert_eq!(parsed_empty["type"], "local");

        let parsed_doh = parse_dns_server("doh", "https://1.1.1.1/dns-query", Some("proxy"));
        assert_eq!(parsed_doh["type"], "https");
        assert_eq!(parsed_doh["server"], "https://1.1.1.1/dns-query");

        let parsed_dot = parse_dns_server("dot", "tls://1.1.1.1:853", Some("proxy"));
        assert_eq!(parsed_dot["type"], "tls");
        assert_eq!(parsed_dot["server"], "1.1.1.1");
        assert_eq!(parsed_dot["server_port"], 853);

        let parsed_udp_prefix = parse_dns_server("udp", "udp://1.1.1.1:53", Some("direct"));
        assert_eq!(parsed_udp_prefix["type"], "udp");
        assert_eq!(parsed_udp_prefix["server"], "1.1.1.1");
        assert_eq!(parsed_udp_prefix["server_port"], 53);
    }
}
