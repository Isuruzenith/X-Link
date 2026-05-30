use std::collections::HashMap;
use serde_json::Value;
use super::SingBoxOutbound;

pub fn adapt(raw: &[u8]) -> Result<Vec<SingBoxOutbound>, String> {
    // Parse YAML into serde_json::Value
    let doc: Value = serde_yaml::from_slice(raw)
        .map_err(|e| format!("Failed to parse Clash YAML: {}", e))?;

    let proxies = doc.get("proxies")
        .and_then(|p| p.as_array())
        .ok_or_else(|| "Clash subscription does not contain a 'proxies' array".to_string())?;

    let mut outbounds = Vec::new();

    for proxy in proxies {
        let obj = match proxy.as_object() {
            Some(o) => o,
            None => continue,
        };

        let tag = match obj.get("name").and_then(|n| n.as_str()) {
            Some(t) => t.to_string(),
            None => continue,
        };

        let clash_type = match obj.get("type").and_then(|t| t.as_str()) {
            Some(ct) => ct.to_lowercase(),
            None => continue,
        };

        let server = obj.get("server").and_then(|s| s.as_str()).unwrap_or("").to_string();
        let port = obj.get("port").and_then(|p| p.as_u64()).unwrap_or(0) as u16;

        let mut fields = HashMap::new();
        fields.insert("server".to_string(), Value::String(server));
        fields.insert("server_port".to_string(), Value::Number(port.into()));

        let outbound_type = match clash_type.as_str() {
            "ss" => {
                if let Some(cipher) = obj.get("cipher").and_then(|c| c.as_str()) {
                    fields.insert("method".to_string(), Value::String(cipher.to_string()));
                }
                if let Some(password) = obj.get("password").and_then(|p| p.as_str()) {
                    fields.insert("password".to_string(), Value::String(password.to_string()));
                }
                "shadowsocks".to_string()
            }
            "vmess" => {
                if let Some(uuid) = obj.get("uuid").and_then(|u| u.as_str()) {
                    fields.insert("uuid".to_string(), Value::String(uuid.to_string()));
                }
                let alter_id = obj.get("alterId").and_then(|a| a.as_u64()).unwrap_or(0);
                fields.insert("alter_id".to_string(), Value::Number(alter_id.into()));
                fields.insert("security".to_string(), Value::String("auto".to_string()));

                // WebSocket transport parsing
                if let Some(network) = obj.get("network").and_then(|n| n.as_str()) {
                    if network == "ws" {
                        let mut transport = HashMap::new();
                        transport.insert("type".to_string(), Value::String("ws".to_string()));
                        
                        if let Some(ws_opts) = obj.get("ws-opts").and_then(|o| o.as_object()) {
                            if let Some(path) = ws_opts.get("path").and_then(|p| p.as_str()) {
                                transport.insert("path".to_string(), Value::String(path.to_string()));
                            }
                            if let Some(headers) = ws_opts.get("headers").and_then(|h| h.as_object()) {
                                let mut transport_headers = HashMap::new();
                                for (k, v) in headers {
                                    if let Some(v_str) = v.as_str() {
                                        transport_headers.insert(k.clone(), Value::String(v_str.to_string()));
                                    }
                                }
                                transport.insert("headers".to_string(), Value::Object(transport_headers.into_iter().collect()));
                            }
                        }
                        
                        fields.insert("transport".to_string(), Value::Object(transport.into_iter().collect()));
                    }
                }
                "vmess".to_string()
            }
            "vless" => {
                if let Some(uuid) = obj.get("uuid").and_then(|u| u.as_str()) {
                    fields.insert("uuid".to_string(), Value::String(uuid.to_string()));
                }
                if let Some(flow) = obj.get("flow").and_then(|f| f.as_str()) {
                    fields.insert("flow".to_string(), Value::String(flow.to_string()));
                }
                "vless".to_string()
            }
            "trojan" => {
                if let Some(password) = obj.get("password").and_then(|p| p.as_str()) {
                    fields.insert("password".to_string(), Value::String(password.to_string()));
                }
                if let Some(sni) = obj.get("sni").and_then(|s| s.as_str()) {
                    let mut tls = HashMap::new();
                    tls.insert("enabled".to_string(), Value::Bool(true));
                    tls.insert("server_name".to_string(), Value::String(sni.to_string()));
                    fields.insert("tls".to_string(), Value::Object(tls.into_iter().collect()));
                }
                "trojan".to_string()
            }
            _ => clash_type, // Fallback to raw type if it matches
        };

        outbounds.push(SingBoxOutbound {
            outbound_type,
            tag,
            fields,
        });
    }

    Ok(outbounds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapt_clash_yaml() {
        let yaml = r#"
proxies:
  - name: "Shadowsocks Node"
    type: ss
    server: "1.2.3.4"
    port: 8388
    cipher: "aes-256-gcm"
    password: "secretpassword"
"#;
        let res = adapt(yaml.as_bytes()).unwrap();
        assert_eq!(res.len(), 1);
        
        let outbound = &res[0];
        assert_eq!(outbound.outbound_type, "shadowsocks");
        assert_eq!(outbound.tag, "Shadowsocks Node");
        
        let server = outbound.fields.get("server").and_then(|v| v.as_str()).unwrap();
        assert_eq!(server, "1.2.3.4");
        
        let port = outbound.fields.get("server_port").and_then(|v| v.as_u64()).unwrap();
        assert_eq!(port, 8388);
        
        let method = outbound.fields.get("method").and_then(|v| v.as_str()).unwrap();
        assert_eq!(method, "aes-256-gcm");
        
        let password = outbound.fields.get("password").and_then(|v| v.as_str()).unwrap();
        assert_eq!(password, "secretpassword");
    }
}

