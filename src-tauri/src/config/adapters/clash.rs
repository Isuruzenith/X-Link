use super::SingBoxOutbound;
use serde_json::Value;
use std::collections::HashMap;

fn parse_clash_tls(obj: &serde_json::Map<String, Value>, default_enabled: bool) -> Option<Value> {
    let tls_enabled = obj
        .get("tls")
        .and_then(|v| v.as_bool())
        .unwrap_or(default_enabled);
    let sni = obj
        .get("servername")
        .and_then(|v| v.as_str())
        .or_else(|| obj.get("sni").and_then(|v| v.as_str()));
    let reality_opts = obj.get("reality-opts").and_then(|v| v.as_object());

    if tls_enabled || sni.is_some() || reality_opts.is_some() {
        let mut tls = HashMap::new();
        tls.insert("enabled".to_string(), Value::Bool(true));

        if let Some(sni_val) = sni {
            tls.insert(
                "server_name".to_string(),
                Value::String(sni_val.to_string()),
            );
        }

        let skip_verify = obj
            .get("skip-cert-verify")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if skip_verify {
            tls.insert("insecure".to_string(), Value::Bool(true));
        }

        // Parse fingerprint if available
        if let Some(fp) = obj.get("fingerprint").and_then(|v| v.as_str()) {
            let mut utls = HashMap::new();
            utls.insert("enabled".to_string(), Value::Bool(true));
            utls.insert("fingerprint".to_string(), Value::String(fp.to_string()));
            tls.insert(
                "utls".to_string(),
                Value::Object(utls.into_iter().collect()),
            );
        }

        if let Some(ro) = reality_opts {
            let mut reality = HashMap::new();
            reality.insert("enabled".to_string(), Value::Bool(true));
            if let Some(pbk) = ro.get("public-key").and_then(|v| v.as_str()) {
                reality.insert("public_key".to_string(), Value::String(pbk.to_string()));
            }
            if let Some(sid) = ro.get("short-id").and_then(|v| v.as_str()) {
                reality.insert("short_id".to_string(), Value::String(sid.to_string()));
            }
            tls.insert(
                "reality".to_string(),
                Value::Object(reality.into_iter().collect()),
            );
        }

        return Some(Value::Object(tls.into_iter().collect()));
    }
    None
}

fn parse_clash_transport(obj: &serde_json::Map<String, Value>) -> Option<Value> {
    let network = obj.get("network").and_then(|v| v.as_str()).unwrap_or("");
    if network.is_empty() || network == "tcp" {
        return None;
    }

    let mut transport = HashMap::new();
    transport.insert("type".to_string(), Value::String(network.to_string()));

    match network {
        "ws" => {
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
                    transport.insert(
                        "headers".to_string(),
                        Value::Object(transport_headers.into_iter().collect()),
                    );
                }
            }
        }
        "grpc" => {
            if let Some(grpc_opts) = obj.get("grpc-opts").and_then(|o| o.as_object()) {
                if let Some(service_name) =
                    grpc_opts.get("grpc-service-name").and_then(|s| s.as_str())
                {
                    transport.insert(
                        "service_name".to_string(),
                        Value::String(service_name.to_string()),
                    );
                }
            }
        }
        "h2" | "http" => {
            if let Some(h2_opts) = obj.get("h2-opts").and_then(|o| o.as_object()) {
                if let Some(paths) = h2_opts.get("path").and_then(|p| p.as_array()) {
                    if let Some(first_path) = paths.first().and_then(|p| p.as_str()) {
                        transport.insert("path".to_string(), Value::String(first_path.to_string()));
                    }
                } else if let Some(path) = h2_opts.get("path").and_then(|p| p.as_str()) {
                    transport.insert("path".to_string(), Value::String(path.to_string()));
                }
                if let Some(host) = h2_opts.get("host").and_then(|h| h.as_array()) {
                    if let Some(first_host) = host.first().and_then(|h| h.as_str()) {
                        transport.insert(
                            "host".to_string(),
                            Value::Array(vec![Value::String(first_host.to_string())]),
                        );
                    }
                }
            }
        }
        _ => {}
    }

    Some(Value::Object(transport.into_iter().collect()))
}

pub fn adapt(raw: &[u8]) -> Result<Vec<SingBoxOutbound>, String> {
    // Parse YAML into serde_json::Value
    let doc: Value =
        serde_yaml::from_slice(raw).map_err(|e| format!("Failed to parse Clash YAML: {}", e))?;

    let proxies = doc
        .get("proxies")
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

        let server = obj
            .get("server")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
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

                if let Some(tls) = parse_clash_tls(obj, false) {
                    fields.insert("tls".to_string(), tls);
                }
                if let Some(transport) = parse_clash_transport(obj) {
                    fields.insert("transport".to_string(), transport);
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
                if let Some(tls) = parse_clash_tls(obj, false) {
                    fields.insert("tls".to_string(), tls);
                }
                if let Some(transport) = parse_clash_transport(obj) {
                    fields.insert("transport".to_string(), transport);
                }
                "vless".to_string()
            }
            "trojan" => {
                if let Some(password) = obj.get("password").and_then(|p| p.as_str()) {
                    fields.insert("password".to_string(), Value::String(password.to_string()));
                }
                if let Some(tls) = parse_clash_tls(obj, true) {
                    fields.insert("tls".to_string(), tls);
                }
                if let Some(transport) = parse_clash_transport(obj) {
                    fields.insert("transport".to_string(), transport);
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

        let server = outbound
            .fields
            .get("server")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(server, "1.2.3.4");

        let port = outbound
            .fields
            .get("server_port")
            .and_then(|v| v.as_u64())
            .unwrap();
        assert_eq!(port, 8388);

        let method = outbound
            .fields
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(method, "aes-256-gcm");

        let password = outbound
            .fields
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(password, "secretpassword");
    }

    #[test]
    fn test_adapt_clash_yaml_complex() {
        let yaml = r#"
proxies:
  - name: "Vless Reality Node"
    type: vless
    server: "4.5.6.7"
    port: 443
    uuid: "f47ac10b-58cc-4372-a567-0e02b2c3d479"
    flow: "xtls-rprx-vision"
    tls: true
    servername: "aka.ms"
    network: grpc
    grpc-opts:
      grpc-service-name: "grpc-test"
    reality-opts:
      public-key: "pbkkey"
      short-id: "shortid"
"#;
        let res = adapt(yaml.as_bytes()).unwrap();
        assert_eq!(res.len(), 1);
        let outbound = &res[0];
        assert_eq!(outbound.outbound_type, "vless");
        assert_eq!(outbound.tag, "Vless Reality Node");

        let tls = outbound.fields.get("tls").unwrap().as_object().unwrap();
        assert_eq!(tls.get("enabled").unwrap().as_bool().unwrap(), true);
        assert_eq!(tls.get("server_name").unwrap().as_str().unwrap(), "aka.ms");

        let reality = tls.get("reality").unwrap().as_object().unwrap();
        assert_eq!(reality.get("enabled").unwrap().as_bool().unwrap(), true);
        assert_eq!(
            reality.get("public_key").unwrap().as_str().unwrap(),
            "pbkkey"
        );
        assert_eq!(
            reality.get("short_id").unwrap().as_str().unwrap(),
            "shortid"
        );

        let transport = outbound
            .fields
            .get("transport")
            .unwrap()
            .as_object()
            .unwrap();
        assert_eq!(transport.get("type").unwrap().as_str().unwrap(), "grpc");
        assert_eq!(
            transport.get("service_name").unwrap().as_str().unwrap(),
            "grpc-test"
        );
    }
}
