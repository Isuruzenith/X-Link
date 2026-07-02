use super::SingBoxOutbound;
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

fn decode_base64_padded(s: &str) -> Result<Vec<u8>, String> {
    let mut s = s.trim().to_string();
    while !s.len().is_multiple_of(4) {
        s.push('=');
    }
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &s)
        .or_else(|_| base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE, &s))
        .map_err(|e| format!("Base64 decode failed: {}", e))
}

fn parse_tls_settings(query_params: &HashMap<String, String>) -> Option<Value> {
    let security = query_params
        .get("security")
        .map(|s| s.as_str())
        .unwrap_or("");
    let sni = query_params.get("sni").or_else(|| query_params.get("peer"));

    if security == "tls" || security == "reality" || sni.is_some() {
        let mut tls = HashMap::new();
        tls.insert("enabled".to_string(), Value::Bool(true));

        if let Some(sni_val) = sni {
            tls.insert(
                "server_name".to_string(),
                Value::String(sni_val.to_string()),
            );
        }

        if let Some(fp) = query_params.get("fp") {
            let mut utls = HashMap::new();
            utls.insert("enabled".to_string(), Value::Bool(true));
            utls.insert("fingerprint".to_string(), Value::String(fp.to_string()));
            tls.insert(
                "utls".to_string(),
                Value::Object(utls.into_iter().collect()),
            );
        }

        let is_insecure = query_params
            .get("allowInsecure")
            .map(|s| s == "1" || s == "true")
            .or_else(|| {
                query_params
                    .get("insecure")
                    .map(|s| s == "1" || s == "true")
            })
            .unwrap_or(false);

        if is_insecure {
            tls.insert("insecure".to_string(), Value::Bool(true));
        }

        if security == "reality" {
            let mut reality = HashMap::new();
            reality.insert("enabled".to_string(), Value::Bool(true));
            if let Some(pbk) = query_params.get("pbk") {
                reality.insert("public_key".to_string(), Value::String(pbk.to_string()));
            }
            if let Some(sid) = query_params.get("sid") {
                reality.insert("short_id".to_string(), Value::String(sid.to_string()));
            }
            tls.insert(
                "reality".to_string(),
                Value::Object(reality.into_iter().collect()),
            );
        }

        if let Some(alpn_str) = query_params.get("alpn") {
            let alpn_list: Vec<Value> = alpn_str
                .split(',')
                .map(|s| Value::String(s.to_string()))
                .collect();
            tls.insert("alpn".to_string(), Value::Array(alpn_list));
        }

        return Some(Value::Object(tls.into_iter().collect()));
    }

    None
}

fn parse_transport_settings(query_params: &HashMap<String, String>) -> Option<Value> {
    let transport_type = query_params.get("type").map(|t| t.as_str()).unwrap_or("");
    if transport_type.is_empty() || transport_type == "tcp" {
        return None;
    }

    let mut transport = HashMap::new();
    transport.insert(
        "type".to_string(),
        Value::String(transport_type.to_string()),
    );

    match transport_type {
        "ws" | "httpupgrade" => {
            if let Some(path) = query_params.get("path") {
                transport.insert("path".to_string(), Value::String(path.to_string()));
            }
            if let Some(host) = query_params.get("host") {
                let mut headers = HashMap::new();
                headers.insert("Host".to_string(), Value::String(host.to_string()));
                transport.insert(
                    "headers".to_string(),
                    Value::Object(headers.into_iter().collect()),
                );
            }
        }
        "grpc" => {
            if let Some(service_name) = query_params
                .get("serviceName")
                .or_else(|| query_params.get("servicename"))
            {
                transport.insert(
                    "service_name".to_string(),
                    Value::String(service_name.to_string()),
                );
            }
        }
        _ => {}
    }

    Some(Value::Object(transport.into_iter().collect()))
}

fn parse_shadowsocks_uri(trimmed: &str) -> Option<(String, String, String, String, u16)> {
    if !trimmed.starts_with("ss://") {
        return None;
    }
    let s = &trimmed[5..];

    // Split fragment (#tag)
    let parts: Vec<&str> = s.split('#').collect();
    let main_with_query = parts[0];
    let tag = parts
        .get(1)
        .map(|f| {
            percent_encoding::percent_decode_str(f)
                .decode_utf8_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "Shadowsocks Proxy".to_string());

    // Split query (?plugin=...)
    let main = main_with_query.split('?').next().unwrap_or(main_with_query);

    if main.contains('@') {
        // SIP002 Format
        let at_parts: Vec<&str> = main.splitn(2, '@').collect();
        let userinfo_part = at_parts[0];
        let server_part = at_parts[1];

        let (method, password) = if let Ok(decoded_userinfo) = decode_base64_padded(userinfo_part) {
            let userinfo_str = String::from_utf8_lossy(&decoded_userinfo).into_owned();
            let colon_parts: Vec<&str> = userinfo_str.splitn(2, ':').collect();
            if colon_parts.len() == 2 {
                (colon_parts[0].to_string(), colon_parts[1].to_string())
            } else {
                return None;
            }
        } else {
            // Plain userinfo
            let userinfo_dec = percent_encoding::percent_decode_str(userinfo_part)
                .decode_utf8_lossy()
                .into_owned();
            let colon_parts: Vec<&str> = userinfo_dec.splitn(2, ':').collect();
            if colon_parts.len() == 2 {
                (colon_parts[0].to_string(), colon_parts[1].to_string())
            } else {
                return None;
            }
        };

        // Parse host:port
        let server_clean = server_part.trim_end_matches('/');
        let colon_idx = server_clean.rfind(':')?;
        let host = server_clean[..colon_idx].to_string();
        let port_str = &server_clean[colon_idx + 1..];
        let port = port_str.parse::<u16>().ok()?;

        Some((tag, method, password, host, port))
    } else {
        // Legacy Format (entire main is base64 encoded)
        let decoded_bytes = decode_base64_padded(main).ok()?;
        let decoded_str = String::from_utf8_lossy(&decoded_bytes).into_owned();
        if !decoded_str.contains('@') {
            return None;
        }
        let at_parts: Vec<&str> = decoded_str.splitn(2, '@').collect();
        let userinfo_part = at_parts[0];
        let server_part = at_parts[1];

        let colon_parts: Vec<&str> = userinfo_part.splitn(2, ':').collect();
        if colon_parts.len() != 2 {
            return None;
        }
        let method = colon_parts[0].to_string();
        let password = colon_parts[1].to_string();

        // Parse host:port
        let server_clean = server_part.trim_end_matches('/');
        let colon_idx = server_clean.rfind(':')?;
        let host = server_clean[..colon_idx].to_string();
        let port_str = &server_clean[colon_idx + 1..];
        let port = port_str.parse::<u16>().ok()?;

        Some((tag, method, password, host, port))
    }
}

pub fn adapt(raw: &[u8]) -> Result<Vec<SingBoxOutbound>, String> {
    let raw_str = String::from_utf8_lossy(raw);
    let mut outbounds = Vec::new();

    for line in raw_str.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        // ── VMESS PARSING ────────────────────────────────────────────────────
        if let Some(b64_part) = trimmed.strip_prefix("vmess://") {
            let parts: Vec<&str> = b64_part.split('#').collect();
            let b64_clean = parts[0];
            let fragment_tag = parts.get(1).map(|t| {
                percent_encoding::percent_decode_str(t)
                    .decode_utf8_lossy()
                    .into_owned()
            });

            let decoded_b64 = percent_encoding::percent_decode_str(b64_clean).decode_utf8_lossy();
            // Decode base64 JSON
            let decoded = match decode_base64_padded(&decoded_b64) {
                Ok(bytes) => bytes,
                Err(_) => continue, // skip invalid vmess URIs
            };

            let json: Value = match serde_json::from_slice(&decoded) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let tag = json
                .get("ps")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
                .or(fragment_tag)
                .unwrap_or_else(|| "VMess Proxy".to_string());
            let server = json
                .get("add")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();

            let port = match json.get("port") {
                Some(Value::Number(n)) => n.as_u64().unwrap_or(0) as u16,
                Some(Value::String(s)) => s.parse::<u16>().unwrap_or(0),
                _ => 0,
            };

            let uuid = json
                .get("id")
                .and_then(|u| u.as_str())
                .unwrap_or("")
                .to_string();
            let alter_id = json.get("aid").and_then(|a| a.as_u64()).unwrap_or(0);

            let mut fields = HashMap::new();
            fields.insert("server".to_string(), Value::String(server));
            fields.insert("server_port".to_string(), Value::Number(port.into()));
            fields.insert("uuid".to_string(), Value::String(uuid));
            fields.insert("alter_id".to_string(), Value::Number(alter_id.into()));
            fields.insert("security".to_string(), Value::String("auto".to_string()));

            let tls_enabled = match json.get("tls") {
                Some(Value::String(s)) => s == "tls",
                Some(Value::Bool(b)) => *b,
                Some(Value::Number(n)) => n.as_u64() == Some(1),
                _ => false,
            };

            if tls_enabled {
                let mut tls = HashMap::new();
                tls.insert("enabled".to_string(), Value::Bool(true));

                let host = json.get("host").and_then(|h| h.as_str());
                let sni = json.get("sni").and_then(|s| s.as_str()).or(host);
                if let Some(sni_val) = sni {
                    tls.insert(
                        "server_name".to_string(),
                        Value::String(sni_val.to_string()),
                    );
                }

                let is_insecure = match json.get("verify_cert") {
                    Some(Value::Bool(b)) => !*b,
                    _ => match json.get("allowInsecure") {
                        Some(Value::Bool(b)) => *b,
                        Some(Value::Number(n)) => n.as_u64() == Some(1),
                        Some(Value::String(s)) => s == "1" || s == "true",
                        _ => false,
                    },
                };
                if is_insecure {
                    tls.insert("insecure".to_string(), Value::Bool(true));
                }
                fields.insert("tls".to_string(), Value::Object(tls.into_iter().collect()));
            }

            // Transport WS check
            if let Some(net) = json.get("net").and_then(|n| n.as_str()) {
                if net == "ws" {
                    let mut transport = HashMap::new();
                    transport.insert("type".to_string(), Value::String("ws".to_string()));
                    if let Some(path) = json.get("path").and_then(|p| p.as_str()) {
                        transport.insert("path".to_string(), Value::String(path.to_string()));
                    }
                    if let Some(host) = json.get("host").and_then(|h| h.as_str()) {
                        let mut headers = HashMap::new();
                        headers.insert("Host".to_string(), Value::String(host.to_string()));
                        transport.insert(
                            "headers".to_string(),
                            Value::Object(headers.into_iter().collect()),
                        );
                    }
                    fields.insert(
                        "transport".to_string(),
                        Value::Object(transport.into_iter().collect()),
                    );
                }
            }

            outbounds.push(SingBoxOutbound {
                outbound_type: "vmess".to_string(),
                tag,
                fields,
            });
            continue;
        }

        // ── SHADOWSOCKS PARSING ──────────────────────────────────────────────
        if trimmed.starts_with("ss://") {
            if let Some((tag, method, password, host, port)) = parse_shadowsocks_uri(trimmed) {
                let mut fields = HashMap::new();
                fields.insert("server".to_string(), Value::String(host));
                fields.insert("server_port".to_string(), Value::Number(port.into()));
                fields.insert("method".to_string(), Value::String(method));
                fields.insert("password".to_string(), Value::String(password));
                outbounds.push(SingBoxOutbound {
                    outbound_type: "shadowsocks".to_string(),
                    tag,
                    fields,
                });
            }
            continue;
        }

        // ── OTHER URIs (VLESS, TROJAN) ───────────────────────────────────────
        let url = match Url::parse(trimmed) {
            Ok(u) => u,
            Err(_) => continue,
        };

        let tag = url
            .fragment()
            .map(|f| {
                percent_encoding::percent_decode_str(f)
                    .decode_utf8_lossy()
                    .into_owned()
            })
            .unwrap_or_else(|| "Proxy Node".to_string());

        let server = url.host_str().unwrap_or("").to_string();
        let port = url.port().unwrap_or(0);

        let mut fields = HashMap::new();
        fields.insert("server".to_string(), Value::String(server));
        fields.insert("server_port".to_string(), Value::Number(port.into()));

        match url.scheme() {
            "vless" => {
                // vless://[uuid]@host:port?query#tag
                let uuid = url.username().to_string();
                fields.insert("uuid".to_string(), Value::String(uuid));

                let query_params: HashMap<String, String> =
                    url.query_pairs().into_owned().collect();

                let mut flow_val = query_params.get("flow").map(|s| s.to_string());
                if flow_val.is_none() {
                    let security = query_params
                        .get("security")
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let net_type = query_params
                        .get("type")
                        .map(|s| s.as_str())
                        .unwrap_or("tcp");
                    if security == "reality" && net_type == "tcp" {
                        flow_val = Some("xtls-rprx-vision".to_string());
                    }
                }

                if let Some(flow) = flow_val {
                    fields.insert("flow".to_string(), Value::String(flow));
                }

                if let Some(pe) = query_params.get("packetEncoding") {
                    fields.insert("packet_encoding".to_string(), Value::String(pe.to_string()));
                }

                if let Some(tls) = parse_tls_settings(&query_params) {
                    fields.insert("tls".to_string(), tls);
                }

                if let Some(transport) = parse_transport_settings(&query_params) {
                    fields.insert("transport".to_string(), transport);
                }

                outbounds.push(SingBoxOutbound {
                    outbound_type: "vless".to_string(),
                    tag,
                    fields,
                });
            }
            "trojan" => {
                // trojan://[password]@host:port?query#tag
                let password = url.username().to_string();
                fields.insert("password".to_string(), Value::String(password));

                let query_params: HashMap<String, String> =
                    url.query_pairs().into_owned().collect();
                if let Some(tls) = parse_tls_settings(&query_params) {
                    fields.insert("tls".to_string(), tls);
                }

                if let Some(transport) = parse_transport_settings(&query_params) {
                    fields.insert("transport".to_string(), transport);
                }

                outbounds.push(SingBoxOutbound {
                    outbound_type: "trojan".to_string(),
                    tag,
                    fields,
                });
            }
            _ => {}
        }
    }

    Ok(outbounds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapt_raw_uris() {
        let uris = "ss://YWVzLTI1Ni1nY206c2VjcmV0cGFzc3dvcmQ@1.2.3.4:8388#Shadowsocks%20Node\nvless://f47ac10b-58cc-4372-a567-0e02b2c3d479@4.5.6.7:443?flow=xtls-rprx-vision#Vless%20Node";
        let res = adapt(uris.as_bytes()).unwrap();
        assert_eq!(res.len(), 2);

        // Shadowsocks node check
        let ss_node = &res[0];
        assert_eq!(ss_node.outbound_type, "shadowsocks");
        assert_eq!(ss_node.tag, "Shadowsocks Node");
        assert_eq!(
            ss_node
                .fields
                .get("server")
                .and_then(|v| v.as_str())
                .unwrap(),
            "1.2.3.4"
        );
        assert_eq!(
            ss_node
                .fields
                .get("server_port")
                .and_then(|v| v.as_u64())
                .unwrap(),
            8388
        );
        assert_eq!(
            ss_node
                .fields
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap(),
            "aes-256-gcm"
        );
        assert_eq!(
            ss_node
                .fields
                .get("password")
                .and_then(|v| v.as_str())
                .unwrap(),
            "secretpassword"
        );

        // Vless node check
        let vless_node = &res[1];
        assert_eq!(vless_node.outbound_type, "vless");
        assert_eq!(vless_node.tag, "Vless Node");
        assert_eq!(
            vless_node
                .fields
                .get("server")
                .and_then(|v| v.as_str())
                .unwrap(),
            "4.5.6.7"
        );
        assert_eq!(
            vless_node
                .fields
                .get("server_port")
                .and_then(|v| v.as_u64())
                .unwrap(),
            443
        );
        assert_eq!(
            vless_node
                .fields
                .get("uuid")
                .and_then(|v| v.as_str())
                .unwrap(),
            "f47ac10b-58cc-4372-a567-0e02b2c3d479"
        );
        assert_eq!(
            vless_node
                .fields
                .get("flow")
                .and_then(|v| v.as_str())
                .unwrap(),
            "xtls-rprx-vision"
        );
    }

    #[test]
    fn test_adapt_raw_uris_transport() {
        let uris = "vless://f47ac10b-58cc-4372-a567-0e02b2c3d479@4.5.6.7:443?type=ws&path=%2Fchat&host=cloudflare.com#Vless%20WS\ntrojan://mypass@4.5.6.7:443?type=grpc&serviceName=grpc-test#Trojan%20gRPC";
        let res = adapt(uris.as_bytes()).unwrap();
        assert_eq!(res.len(), 2);

        // VLESS WS check
        let vless_ws = &res[0];
        assert_eq!(vless_ws.outbound_type, "vless");
        assert_eq!(vless_ws.tag, "Vless WS");
        let transport = vless_ws
            .fields
            .get("transport")
            .unwrap()
            .as_object()
            .unwrap();
        assert_eq!(transport.get("type").unwrap().as_str().unwrap(), "ws");
        assert_eq!(transport.get("path").unwrap().as_str().unwrap(), "/chat");
        let headers = transport.get("headers").unwrap().as_object().unwrap();
        assert_eq!(
            headers.get("Host").unwrap().as_str().unwrap(),
            "cloudflare.com"
        );

        // Trojan gRPC check
        let trojan_grpc = &res[1];
        assert_eq!(trojan_grpc.outbound_type, "trojan");
        assert_eq!(trojan_grpc.tag, "Trojan gRPC");
        let transport_grpc = trojan_grpc
            .fields
            .get("transport")
            .unwrap()
            .as_object()
            .unwrap();
        assert_eq!(
            transport_grpc.get("type").unwrap().as_str().unwrap(),
            "grpc"
        );
        assert_eq!(
            transport_grpc
                .get("service_name")
                .unwrap()
                .as_str()
                .unwrap(),
            "grpc-test"
        );
    }

    #[test]
    fn test_adapt_zoom_vless_ws_tls_uri() {
        let uri = "vless://88dacb71-7530-475b-9ed6-a431caef6b3f@azure.ezgateway.net:443?alpn=h2%2Chttp%2F1.1&encryption=none&fp=chrome&host=azure.ezgateway.net&path=%2Fazure&security=tls&sni=aka.ms&allowInsecure=1&type=ws#Zoom-SG-Kavishka-300GB";
        let res = adapt(uri.as_bytes()).unwrap();
        assert_eq!(res.len(), 1);

        let node = &res[0];
        assert_eq!(node.outbound_type, "vless");
        assert_eq!(node.tag, "Zoom-SG-Kavishka-300GB");
        assert_eq!(
            node.fields.get("server").and_then(|v| v.as_str()).unwrap(),
            "azure.ezgateway.net"
        );
        assert_eq!(
            node.fields
                .get("server_port")
                .and_then(|v| v.as_u64())
                .unwrap(),
            443
        );
        assert_eq!(
            node.fields.get("uuid").and_then(|v| v.as_str()).unwrap(),
            "88dacb71-7530-475b-9ed6-a431caef6b3f"
        );

        let tls = node.fields.get("tls").unwrap().as_object().unwrap();
        assert_eq!(tls.get("enabled").unwrap().as_bool().unwrap(), true);
        assert_eq!(tls.get("server_name").unwrap().as_str().unwrap(), "aka.ms");
        assert_eq!(tls.get("insecure").unwrap().as_bool().unwrap(), true);
        assert_eq!(
            tls.get("alpn").unwrap().as_array().unwrap()[0]
                .as_str()
                .unwrap(),
            "h2"
        );
        assert_eq!(
            tls.get("alpn").unwrap().as_array().unwrap()[1]
                .as_str()
                .unwrap(),
            "http/1.1"
        );
        let utls = tls.get("utls").unwrap().as_object().unwrap();
        assert_eq!(utls.get("fingerprint").unwrap().as_str().unwrap(), "chrome");

        let transport = node.fields.get("transport").unwrap().as_object().unwrap();
        assert_eq!(transport.get("type").unwrap().as_str().unwrap(), "ws");
        assert_eq!(transport.get("path").unwrap().as_str().unwrap(), "/azure");
        let headers = transport.get("headers").unwrap().as_object().unwrap();
        assert_eq!(
            headers.get("Host").unwrap().as_str().unwrap(),
            "azure.ezgateway.net"
        );
    }

    #[test]
    fn test_adapt_raw_vmess() {
        // vmess JSON: {"ps":"VMess Test Node","add":"1.2.3.4","port":443,"id":"f47ac10b-58cc-4372-a567-0e02b2c3d479","aid":0,"tls":"tls","net":"ws","path":"/path","host":"sni.host"}
        let uri = "vmess://eyJwcyI6IlZNZXNzIFRlc3QgTm9kZSIsImFkZCI6IjEuMi4zLjQiLCJwb3J0Ijo0NDMsImlkIjoiZjQ3YWMxMGItNThjYy00MzcyLWE1NjctMGUwMmIyYzNkNDc5IiwiYWlkIjowLCJ0bHMiOiJ0bHMiLCJuZXQiOiJ3cyIsInBhdGgiOiIvcGF0aCIsImhvc3QiOiJzbmkuaG9zdCJ9";
        let res = adapt(uri.as_bytes()).unwrap();
        assert_eq!(res.len(), 1);
        let vmess = &res[0];
        assert_eq!(vmess.outbound_type, "vmess");
        assert_eq!(vmess.tag, "VMess Test Node");
        assert_eq!(
            vmess.fields.get("server").unwrap().as_str().unwrap(),
            "1.2.3.4"
        );
        assert_eq!(
            vmess.fields.get("server_port").unwrap().as_u64().unwrap(),
            443
        );
        assert_eq!(
            vmess.fields.get("uuid").unwrap().as_str().unwrap(),
            "f47ac10b-58cc-4372-a567-0e02b2c3d479"
        );

        let tls = vmess.fields.get("tls").unwrap().as_object().unwrap();
        assert_eq!(tls.get("enabled").unwrap().as_bool().unwrap(), true);
        assert_eq!(
            tls.get("server_name").unwrap().as_str().unwrap(),
            "sni.host"
        );

        let transport = vmess.fields.get("transport").unwrap().as_object().unwrap();
        assert_eq!(transport.get("type").unwrap().as_str().unwrap(), "ws");
        assert_eq!(transport.get("path").unwrap().as_str().unwrap(), "/path");
        let headers = transport.get("headers").unwrap().as_object().unwrap();
        assert_eq!(headers.get("Host").unwrap().as_str().unwrap(), "sni.host");

        // Test with percent encoded padding: eyJwcyI6IkFCIiwiYWRkIjoiMS4yLjMuNCIsInBvcnQiOjQ0MywiaWQiOiJ1dWlkIiwiYWlkIjowfQ%3D%3D
        let uri_percent = "vmess://eyJwcyI6IkFCIiwiYWRkIjoiMS4yLjMuNCIsInBvcnQiOjQ0MywiaWQiOiJ1dWlkIiwiYWlkIjowfQ%3D%3D";
        let res_percent = adapt(uri_percent.as_bytes()).unwrap();
        assert_eq!(res_percent.len(), 1);
        assert_eq!(res_percent[0].tag, "AB");
    }

    #[test]
    fn test_adapt_raw_vmess_with_fragment() {
        // vmess with trailing fragment: #MyVmessNode
        let uri = "vmess://eyJwcyI6IlZNZXNzIFRlc3QiLCJhZGQiOiIxLjIuMy40IiwicG9ydCI6NDQzLCJpZCI6InV1aWQiLCJhaWQiOjB9#MyVmessNode";
        let res = adapt(uri.as_bytes()).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].tag, "VMess Test"); // ps from JSON takes precedence

        // vmess with trailing fragment and missing ps
        let uri_no_ps = "vmess://eyJhZGQiOiIxLjIuMy40IiwicG9ydCI6NDQzLCJpZCI6InV1aWQiLCJhaWQiOjB9#My%20Custom%20Node";
        let res_no_ps = adapt(uri_no_ps.as_bytes()).unwrap();
        assert_eq!(res_no_ps.len(), 1);
        assert_eq!(res_no_ps[0].tag, "My Custom Node"); // falls back to percent-decoded fragment
    }

    #[test]
    fn test_adapt_legacy_shadowsocks() {
        // ss://aes-128-cfb:password@1.2.3.4:8388 in legacy base64: ss://YWVzLTEyOC1jZmI6cGFzc3dvcmRAMS4yLjMuNDo4Mzg4#Legacy%20Node
        let uri = "ss://YWVzLTEyOC1jZmI6cGFzc3dvcmRAMS4yLjMuNDo4Mzg4#Legacy%20Node";
        let res = adapt(uri.as_bytes()).unwrap();
        assert_eq!(res.len(), 1);
        let node = &res[0];
        assert_eq!(node.outbound_type, "shadowsocks");
        assert_eq!(node.tag, "Legacy Node");
        assert_eq!(
            node.fields.get("server").unwrap().as_str().unwrap(),
            "1.2.3.4"
        );
        assert_eq!(
            node.fields.get("server_port").unwrap().as_u64().unwrap(),
            8388
        );
        assert_eq!(
            node.fields.get("method").unwrap().as_str().unwrap(),
            "aes-128-cfb"
        );
        assert_eq!(
            node.fields.get("password").unwrap().as_str().unwrap(),
            "password"
        );
    }

    #[test]
    fn test_adapt_plain_userinfo_shadowsocks() {
        // ss://2022-blake3-aes-128-gcm:password_string@192.168.1.100:8888#Plain%20Node
        let uri = "ss://2022-blake3-aes-128-gcm:password_string@192.168.1.100:8888#Plain%20Node";
        let res = adapt(uri.as_bytes()).unwrap();
        assert_eq!(res.len(), 1);
        let node = &res[0];
        assert_eq!(node.outbound_type, "shadowsocks");
        assert_eq!(node.tag, "Plain Node");
        assert_eq!(
            node.fields.get("server").unwrap().as_str().unwrap(),
            "192.168.1.100"
        );
        assert_eq!(
            node.fields.get("server_port").unwrap().as_u64().unwrap(),
            8888
        );
        assert_eq!(
            node.fields.get("method").unwrap().as_str().unwrap(),
            "2022-blake3-aes-128-gcm"
        );
        assert_eq!(
            node.fields.get("password").unwrap().as_str().unwrap(),
            "password_string"
        );
    }

    #[test]
    fn test_adapt_user_provided_uris() {
        // URI 1: VLESS TCP TLS Reality/SNI
        let uri1 = "vless://1444a1d0-74ea-48eb-8e39-296c87d5f4c2@178.128.16.26:443?security=tls&alpn=h2%2Chttp%2F1.1&encryption=none&headerType=none&type=tcp&flow=xtls-rprx-vision&sni=aka.ms#Danuwa-v2ray-dialog-zoom";
        let res1 = adapt(uri1.as_bytes()).unwrap();
        assert_eq!(res1.len(), 1);
        let node1 = &res1[0];
        assert_eq!(node1.outbound_type, "vless");
        assert_eq!(node1.tag, "Danuwa-v2ray-dialog-zoom");
        assert_eq!(
            node1.fields.get("server").unwrap().as_str().unwrap(),
            "178.128.16.26"
        );
        assert_eq!(
            node1.fields.get("server_port").unwrap().as_u64().unwrap(),
            443
        );
        assert_eq!(
            node1.fields.get("flow").unwrap().as_str().unwrap(),
            "xtls-rprx-vision"
        );
        assert_eq!(
            node1.fields.get("tls").unwrap()["server_name"]
                .as_str()
                .unwrap(),
            "aka.ms"
        );

        // URI 2: Trojan WS TLS
        let uri2 = "trojan://fNeTGTI4KN@172.237.90.117:35287?path=%2FFREE%20By%20Tharuwa%20%280767597317%29&security=tls&alpn=h2%2Chttp%2F1.1&fp=chrome&type=ws&sni=zoom.us#Free%20By%20Tharuwa%20TLS%20ON";
        let res2 = adapt(uri2.as_bytes()).unwrap();
        assert_eq!(res2.len(), 1);
        let node2 = &res2[0];
        assert_eq!(node2.outbound_type, "trojan");
        assert_eq!(node2.tag, "Free By Tharuwa TLS ON");
        assert_eq!(
            node2.fields.get("server").unwrap().as_str().unwrap(),
            "172.237.90.117"
        );
        assert_eq!(
            node2.fields.get("server_port").unwrap().as_u64().unwrap(),
            35287
        );
        assert_eq!(
            node2.fields.get("transport").unwrap()["path"]
                .as_str()
                .unwrap(),
            "/FREE By Tharuwa (0767597317)"
        );
        assert_eq!(
            node2.fields.get("tls").unwrap()["server_name"]
                .as_str()
                .unwrap(),
            "zoom.us"
        );

        // URI 3: VLESS WS TLS
        let uri3 = "vless://408172dd-f3c7-4229-aff7-a741f78bda55@azureedge.duckdns.org:443?alpn=h2%2Chttp%2F1.1&encryption=none&fp=chrome&host=azureedge.duckdns.org&path=%2Fazureedge&security=tls&sni=aka.ms&type=ws#One-Piece-Zoom-In";
        let res3 = adapt(uri3.as_bytes()).unwrap();
        assert_eq!(res3.len(), 1);
        let node3 = &res3[0];
        assert_eq!(node3.outbound_type, "vless");
        assert_eq!(node3.tag, "One-Piece-Zoom-In");
        assert_eq!(
            node3.fields.get("server").unwrap().as_str().unwrap(),
            "azureedge.duckdns.org"
        );
        assert_eq!(
            node3.fields.get("server_port").unwrap().as_u64().unwrap(),
            443
        );
        assert_eq!(
            node3.fields.get("transport").unwrap()["path"]
                .as_str()
                .unwrap(),
            "/azureedge"
        );
        assert_eq!(
            node3.fields.get("transport").unwrap()["headers"]["Host"]
                .as_str()
                .unwrap(),
            "azureedge.duckdns.org"
        );
        assert_eq!(
            node3.fields.get("tls").unwrap()["server_name"]
                .as_str()
                .unwrap(),
            "aka.ms"
        );
    }
}
