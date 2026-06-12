use std::collections::HashMap;
use serde_json::Value;
use super::adapters::SingBoxOutbound;
use super::build_route_exclude_addresses;
use super::resolve_server_ips;
use super::build_route_rules;
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
        final_outbounds.push(Value::Object(obj));
    }

    // Unconditionally push the standard direct outbound (required for dns detour)
    let mut direct = HashMap::new();
    direct.insert("type".to_string(), Value::String("direct".to_string()));
    direct.insert("tag".to_string(), Value::String("direct".to_string()));
    final_outbounds.push(Value::Object(direct.into_iter().collect()));

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
                "172.19.0.1/30"
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

    let route_rules = build_route_rules();

    // In TUN mode, DNS must go through the proxy tunnel to avoid the strict_route WFP deadlock.
    // Two-server strategy:
    //   - proxy-dns: uses a PUBLIC DNS (not the user's local router IP which is unreachable
    //     from the remote proxy server). Routes through the proxy tunnel.
    //   - local-dns: uses the system/local resolver via direct detour, only for resolving
    //     the proxy server's own hostname (triggered by the "outbound: any" rule).
    let dns_section = if proxy_mode == "tun" {
        serde_json::json!({
            "servers": [
                { "tag": "proxy-dns", "address": "tcp://1.1.1.1", "detour": "proxy" },
                { "tag": "local-dns", "address": resolved_dns_address, "detour": "direct" }
            ],
            "rules": [
                { "outbound": "any", "server": "local-dns" }
            ],
            "strategy": "ipv4_only"
        })
    } else {
        serde_json::json!({
            "servers": [
                { "tag": "local-dns", "address": resolved_dns_address, "detour": "direct" }
            ],
            "rules": [
                { "outbound": "any", "server": "local-dns" }
            ]
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
        "route": {
            "rules": route_rules,
            "final": "proxy",
            "auto_detect_interface": true
        }
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
        ).unwrap();
        let config_wifi: serde_json::Value = serde_json::from_str(&config_str_wifi).unwrap();
        let inbounds_wifi = config_wifi["inbounds"].as_array().unwrap();
        assert!(!inbounds_wifi.is_empty());
        let mixed_inbound_wifi = &inbounds_wifi[0];
        assert_eq!(mixed_inbound_wifi["listen"].as_str().unwrap(), "0.0.0.0");
        assert_eq!(mixed_inbound_wifi["listen_port"].as_u64().unwrap(), 7890);
    }
}
