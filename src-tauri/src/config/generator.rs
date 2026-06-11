use std::collections::HashMap;
use serde_json::Value;
use super::adapters::SingBoxOutbound;
use super::build_route_exclude_addresses;
use super::build_route_rules;
use super::resolve_dns_address;

pub fn generate_singbox_config(
    mixed_port: u16,
    outbounds: Vec<SingBoxOutbound>,
    proxy_mode: &str,
    dns_address: &str,
    sni_host: &str,
    listen_address: &str,
) -> Result<String, String> {
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
        
        // If an SNI host is configured, inject/override the TLS server_name with the allowed SNI
        if !sni_host.is_empty() {
            if let Some(tls_val) = obj.get_mut("tls") {
                if let Some(tls_obj) = tls_val.as_object_mut() {
                    // Only override server_name if TLS is actually enabled
                    if tls_obj.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false) {
                        tls_obj.insert("server_name".to_string(), Value::String(sni_host.to_string()));
                    }
                }
            }
        }

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
    let route_exclude_addresses = build_route_exclude_addresses(&resolved_dns_address);
    let mut inbounds = vec![
        serde_json::json!({
            "type": "mixed",
            "tag": "mixed-in",
            "listen": listen_address,
            "listen_port": mixed_port
        })
    ];

    if proxy_mode == "tun" {
        inbounds.push(serde_json::json!({
            "type": "tun",
            "tag": "tun-in",
            "interface_name": "X-Link",
            "address": [
                "172.19.0.1/30",
                "fdfe:dcba:9876::1/126"
            ],
            "auto_route": true,
            "strict_route": true,
            "stack": "gvisor",
            "route_exclude_address": route_exclude_addresses,
            "sniff": true,
            "sniff_override_destination": true
        }));
    }

    let route_rules = build_route_rules();

    // Construct the complete sing-box configuration
    let config = serde_json::json!({
        "log": {
            "level": "info",
            "timestamp": true
        },
        "dns": {
            "servers": [
                { "tag": "local-dns", "address": resolved_dns_address, "detour": "direct" }
            ],
            "rules": [
                { "outbound": "any", "server": "local-dns" }
            ]
        },
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

        // Case 1: Wifi sharing is disabled -> binds to 127.0.0.1
        let config_str_local = generate_singbox_config(
            7890,
            outbounds.clone(),
            "system",
            "1.1.1.1",
            "",
            "127.0.0.1",
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
            "",
            "0.0.0.0",
        ).unwrap();
        let config_wifi: serde_json::Value = serde_json::from_str(&config_str_wifi).unwrap();
        let inbounds_wifi = config_wifi["inbounds"].as_array().unwrap();
        assert!(!inbounds_wifi.is_empty());
        let mixed_inbound_wifi = &inbounds_wifi[0];
        assert_eq!(mixed_inbound_wifi["listen"].as_str().unwrap(), "0.0.0.0");
        assert_eq!(mixed_inbound_wifi["listen_port"].as_u64().unwrap(), 7890);
    }
}
