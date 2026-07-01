use std::collections::HashMap;
use serde_json::Value;
use super::adapters::SingBoxOutbound;
use super::build_route_exclude_addresses;
use super::resolve_server_ips;
use super::resolve_dns_address;

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
        }
    }
}

pub fn generate_singbox_config(
    mixed_port: u16,
    outbounds: Vec<SingBoxOutbound>,
    proxy_mode: &str,
    dns_address: &str,
    listen_address: &str,
    tun_settings: &TunSettings,
    default_outbound_tag: Option<&str>,
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
    selector.insert("outbounds".to_string(), Value::Array(node_tags.into_iter().map(Value::String).collect()));
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

        // Self-heal: If transport type is "ws" (WebSocket), strip "h2" from ALPN.
        // WebSockets over HTTP/2 (RFC 8441) is unsupported by most standard VLESS/VMess proxy servers (Nginx/Xray),
        // causing immediate EOF/reset errors when sing-box attempts it.
        if let Some(transport) = obj.get("transport").and_then(|t| t.as_object()) {
            if transport.get("type").and_then(|t| t.as_str()) == Some("ws") {
                if let Some(tls) = obj.get_mut("tls").and_then(|t| t.as_object_mut()) {
                    if let Some(alpn) = tls.get_mut("alpn").and_then(|a| a.as_array_mut()) {
                        alpn.retain(|v| v.as_str() != Some("h2"));
                    }
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
    let mut inbounds = vec![
        serde_json::json!({
            "type": "mixed",
            "tag": "mixed-in",
            "listen": listen_address,
            "listen_port": mixed_port
        })
    ];

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
            tun_inbound["mtu"] = serde_json::json!(tun_settings.mtu);
        }
        if tun_settings.auto_redirect {
            tun_inbound["auto_redirect"] = serde_json::json!(true);
        }
        if tun_settings.endpoint_independent_nat {
            tun_inbound["endpoint_independent_nat"] = serde_json::json!(true);
        }
        inbounds.push(tun_inbound);
    }

    let mut route_rules = vec![
        serde_json::json!({ "protocol": "dns", "action": "hijack-dns" }),
    ];

    if tun_settings.bypass_lan {
        route_rules.push(serde_json::json!({ "ip_is_private": true, "outbound": "direct" }));
        route_rules.push(serde_json::json!({
            "domain_suffix": [".lan", ".local", ".internal", ".home.arpa"],
            "outbound": "direct"
        }));
    }

    let route_section = serde_json::json!({
        "rules": route_rules,
        "final": tun_settings.final_outbound,
        "auto_detect_interface": true
    });

    let mut dns_rules = vec![
        serde_json::json!({ "outbound": "direct", "server": "local-dns" })
    ];

    let server_domains: Vec<String> = server_hosts
        .iter()
        .filter(|h| h.trim().parse::<std::net::IpAddr>().is_err())
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .collect();
    if !server_domains.is_empty() {
        dns_rules.insert(0, serde_json::json!({
            "domain": server_domains,
            "server": "local-dns"
        }));
    }

    let is_fakeip = tun_settings.dns_mode == "fakeip";

    let mut dns_servers = vec![
        serde_json::json!({ "tag": "local-dns", "address": resolved_dns_address, "detour": "direct" }),
        serde_json::json!({ "tag": "remote-dns", "address": tun_settings.primary_dns, "detour": "proxy" }),
    ];
    if !is_fakeip && !tun_settings.fallback_dns.trim().is_empty() {
        dns_servers.push(serde_json::json!({
            "tag": "remote-dns-fallback", "address": tun_settings.fallback_dns, "detour": "proxy"
        }));
    }
    if is_fakeip {
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
            "strategy": "ipv4_only",
            "final": "remote-dns"
        });
        if is_fakeip {
            section["fakeip"] = serde_json::json!({
                "enabled": true,
                "inet4_range": tun_settings.fakeip_range,
                "inet6_range": "fc00::/18"
            });
        }
        section
    } else {
        serde_json::json!({
            "servers": [
                { "tag": "local-dns", "address": resolved_dns_address, "detour": "direct" }
            ],
            "rules": dns_rules
        })
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
        ).unwrap();
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
        ).unwrap();
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
        fields.insert("server".to_string(), serde_json::json!("azure.ezgateway.net"));
        fields.insert("server_port".to_string(), serde_json::json!(443));
        fields.insert("uuid".to_string(), serde_json::json!("88dacb71-7530-475b-9ed6-a431caef6b3f"));
        fields.insert("tls".to_string(), serde_json::json!({
            "enabled": true,
            "insecure": true,
            "server_name": "aka.ms",
            "alpn": ["h2", "http/1.1"]
        }));
        fields.insert("transport".to_string(), serde_json::json!({
            "type": "ws",
            "path": "/azure",
            "headers": {
                "Host": "azure.ezgateway.net"
            }
        }));

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
        ).unwrap();

        let config: serde_json::Value = serde_json::from_str(&config_str).unwrap();
        let outbounds_arr = config["outbounds"].as_array().unwrap();

        // Find our node outbound
        let node_outbound = outbounds_arr.iter()
            .find(|o| o["tag"].as_str() == Some("Zoom-SG-Kavishka-300GB"))
            .expect("Should find our node outbound");

        let alpn = node_outbound["tls"]["alpn"].as_array().unwrap();
        // Check that "h2" has been stripped
        assert!(alpn.iter().all(|v| v.as_str() != Some("h2")));
        assert!(alpn.iter().any(|v| v.as_str() == Some("http/1.1")));
    }
}
