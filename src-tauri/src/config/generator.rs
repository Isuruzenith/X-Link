use super::adapters::SingBoxOutbound;
use super::build_route_exclude_addresses;
use super::resolve_dns_address;
use super::resolve_server_ips;
use serde_json::Value;
use std::collections::HashMap;

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
}

impl Default for TunSettings {
    fn default() -> Self {
        Self {
            auto_route: true,
            auto_redirect: false,
            strict_route: true,
            stack: "gvisor".to_string(),
            mtu: 1400,
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
            direct_dns: "223.5.5.5".to_string(),
            fakeip_filter: "geosite:private".to_string(),
            dns_leak_protection: true,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn generate_singbox_config(
    mixed_port: u16,
    outbounds: Vec<SingBoxOutbound>,
    proxy_mode: &str,
    dns_address: &str,
    listen_address: &str,
    tun_settings: &TunSettings,
    default_outbound_tag: Option<&str>,
    user_rules: &[crate::config::rules::RoutingRule],
    rule_sets: &[crate::config::rules::RuleSet],
) -> Result<String, String> {
    let mut server_hosts = Vec::new();
    for outbound in &outbounds {
        if let Some(server_val) = outbound.fields.get("server") {
            if let Some(server_str) = server_val.as_str() {
                server_hosts.push(server_str.to_string());
            }
        }
    }
    let server_ips = resolve_server_ips(&server_hosts);

    let mut node_tags: Vec<String> = outbounds.iter().map(|n| n.tag.clone()).collect();

    // Create the outbounds array
    let mut final_outbounds = Vec::new();

    // Fallback if no outbounds imported yet
    if node_tags.is_empty() {
        node_tags.push("direct".to_string());
    }

    // Assemble selector outbound "proxy"
    let mut selector = HashMap::new();
    selector.insert("type".to_string(), Value::String("selector".to_string()));
    selector.insert("tag".to_string(), Value::String("proxy".to_string()));
    selector.insert(
        "outbounds".to_string(),
        Value::Array(node_tags.into_iter().map(Value::String).collect()),
    );
    if let Some(def_tag) = default_outbound_tag {
        selector.insert("default".to_string(), Value::String(def_tag.to_string()));
    }

    // First, push our custom selector outbound
    final_outbounds.push(Value::Object(selector.into_iter().collect()));

    // Push the parsed node outbounds
    for node in outbounds {
        let mut obj = serde_json::to_value(&node)
            .map_err(|e| format!("Failed to serialize node: {}", e))?
            .as_object()
            .unwrap()
            .clone();

        // Ensure type and tag are correct
        obj.insert("tag".to_string(), Value::String(node.tag.clone()));


        // If TLS is enabled, ensure a default ALPN is specified if missing (default to "http/1.1")
        if let Some(tls) = obj.get_mut("tls").and_then(|t| t.as_object_mut()) {
            let enabled = tls.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
            if enabled && tls.get("alpn").is_none() {
                tls.insert("alpn".to_string(), serde_json::json!(["http/1.1"]));
            }
        }

        // Self-heal & Prioritize: If transport type is "ws" (WebSocket), force ALPN to http/1.1.
        // WebSockets over HTTP/2 (RFC 8441) is unsupported by most standard VLESS/VMess proxy servers (Nginx/Xray),
        // causing immediate EOF/reset errors when sing-box attempts it.
        if let Some(transport) = obj.get("transport").and_then(|t| t.as_object()) {
            if transport.get("type").and_then(|t| t.as_str()) == Some("ws") {
                if let Some(tls) = obj.get_mut("tls").and_then(|t| t.as_object_mut()) {
                    tls.insert("alpn".to_string(), serde_json::json!(["http/1.1"]));
                }
            }
        }

        final_outbounds.push(Value::Object(obj));
    }

    // Unconditionally push the standard direct outbound (required for dns detour)
    let mut direct = HashMap::new();
    direct.insert("type".to_string(), Value::String("direct".to_string()));
    direct.insert("tag".to_string(), Value::String("direct".to_string()));
    final_outbounds.push(Value::Object(direct.into_iter().collect()));

    // Unconditionally push block outbound
    final_outbounds.push(serde_json::json!({ "type": "block", "tag": "block" }));

    let resolved_dns_address = resolve_dns_address(Some(dns_address));

    // Construct the dynamic inbounds array
    let route_exclude_addresses = build_route_exclude_addresses(&resolved_dns_address, &server_ips);
    
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
            "interface_name": "X-Link",
            "address": [
                "172.19.0.1/30",
                "fdfe:dcba:9876::1/126"
            ],
            "auto_route": tun_settings.auto_route,
            "strict_route": tun_settings.strict_route,
            "stack": tun_settings.stack,
            "route_exclude_address": route_exclude_addresses,
            "sniff": tun_settings.sniff_enabled,
            "sniff_override_destination": tun_settings.sniff_override_destination
        });
        if tun_settings.mtu != 0 {
            let safe_mtu = if tun_settings.mtu == 1500 { 1400 } else { tun_settings.mtu };
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
        user_rules,
        rule_sets,
    );

    let route_section = serde_json::json!({
        "rules": route_rules,
        "final": tun_settings.final_outbound,
        "auto_detect_interface": true
    });

    let mut dns_rules = vec![serde_json::json!({ "outbound": "direct", "server": "local-dns" })];

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
        resolved_dns_address
    } else {
        tun_settings.direct_dns.clone()
    };

    let mut dns_servers = vec![
        serde_json::json!({ "tag": "local-dns", "address": local_dns_addr, "detour": "direct" }),
        serde_json::json!({ "tag": "remote-dns", "address": tun_settings.primary_dns, "detour": "proxy" }),
    ];
    if !is_fakeip && !tun_settings.fallback_dns.trim().is_empty() {
        dns_servers.push(serde_json::json!({
            "tag": "remote-dns-fallback", "address": tun_settings.fallback_dns, "detour": "proxy"
        }));
    }
    if is_fakeip {
        if !tun_settings.fakeip_filter.trim().is_empty() {
            let filters: Vec<String> = tun_settings.fakeip_filter
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !filters.is_empty() {
                let mut rule = serde_json::json!({
                    "server": "local-dns"
                });
                
                let mut geosite = Vec::new();
                let mut domain = Vec::new();
                let mut domain_suffix = Vec::new();
                let mut domain_keyword = Vec::new();
                
                for f in filters {
                    if f.starts_with("geosite:") {
                        geosite.push(f.strip_prefix("geosite:").unwrap().trim().to_string());
                    } else if f.starts_with("domain:") {
                        domain.push(f.strip_prefix("domain:").unwrap().trim().to_string());
                    } else if f.starts_with("keyword:") {
                        domain_keyword.push(f.strip_prefix("keyword:").unwrap().trim().to_string());
                    } else {
                        domain_suffix.push(f);
                    }
                }
                
                let mut has_matcher = false;
                if !geosite.is_empty() {
                    rule["geosite"] = serde_json::json!(geosite);
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

        dns_servers.push(serde_json::json!({ "tag": "fakeip-dns", "address": "fakeip" }));
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
        if is_fakeip {
            let fakeip_obj = serde_json::json!({
                "enabled": true,
                "inet4_range": tun_settings.fakeip_range,
                "inet6_range": "fc00::/18"
            });
            section["fakeip"] = fakeip_obj;
        }
        section
    } else {
        let mut section = serde_json::json!({
            "servers": [
                { "tag": "local-dns", "address": local_dns_addr, "detour": "direct" }
            ],
            "rules": dns_rules
        });
        section["disable_cache"] = serde_json::json!(!tun_settings.dns_caching);
        section
    };

    // Construct the complete sing-box configuration
    let config = serde_json::json!({
        "log": {
            "level": "info",
            "timestamp": true
        },
        "dns": dns_section,
        "inbounds": inbounds,
        "outbounds": final_outbounds,
        "route": route_section
    });

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
}
