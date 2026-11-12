use super::SingBoxOutbound;
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

fn decode_base64_padded(s: &str) -> Result<Vec<u8>, String> {
    let mut cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    while !cleaned.len().is_multiple_of(4) {
        cleaned.push('=');
    }
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &cleaned)
        .or_else(|_| base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE, &cleaned))
        .map_err(|_| "Base64 decode failed".to_string())
}

fn parse_tls_settings(query_params: &HashMap<String, String>) -> Option<Value> {
    let security = query_params
        .get("security")
        .map(|s| s.as_str())
        .unwrap_or("");
    let sni = query_params
        .get("sni")
        .or_else(|| query_params.get("peer"))
        .or_else(|| query_params.get("serverName"))
        .or_else(|| query_params.get("server_name"));

    if security == "tls" || security == "reality" || sni.is_some() {
        let mut tls = HashMap::new();
        tls.insert("enabled".to_string(), Value::Bool(true));

        if let Some(sni_val) = sni {
            let trimmed = sni_val.trim();
            if !trimmed.is_empty() {
                tls.insert(
                    "server_name".to_string(),
                    Value::String(trimmed.to_string()),
                );
            }
        }

        let fp = query_params.get("fp").map(|s| s.as_str()).unwrap_or("");
        if !fp.is_empty() {
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

            // sing-box requires uTLS for reality client. If fp was omitted, default to chrome:
            if !tls.contains_key("utls") {
                let mut utls = HashMap::new();
                utls.insert("enabled".to_string(), Value::Bool(true));
                utls.insert("fingerprint".to_string(), Value::String("chrome".to_string()));
                tls.insert(
                    "utls".to_string(),
                    Value::Object(utls.into_iter().collect()),
                );
            }
        }

        if let Some(alpn_str) = query_params.get("alpn") {
            let alpn_list: Vec<Value> = alpn_str
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| Value::String(s.to_string()))
                .collect();
            if !alpn_list.is_empty() {
                tls.insert("alpn".to_string(), Value::Array(alpn_list));
            }
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
                if transport_type == "httpupgrade" {
                    transport.insert("host".to_string(), Value::String(host.to_string()));
                } else {
                    let mut headers = HashMap::new();
                    headers.insert("Host".to_string(), Value::String(host.to_string()));
                    transport.insert(
                        "headers".to_string(),
                        Value::Object(headers.into_iter().collect()),
                    );
                }
            }
        }
        "grpc" => {
            if let Some(service_name) = query_params
                .get("serviceName")
                .or_else(|| query_params.get("servicename"))
                .or_else(|| query_params.get("service_name"))
                .or_else(|| query_params.get("path"))
            {
                transport.insert(
                    "service_name".to_string(),
                    Value::String(service_name.to_string()),
                );
            }
            transport.insert("idle_timeout".to_string(), Value::String("15s".to_string()));
            transport.insert("ping_timeout".to_string(), Value::String("15s".to_string()));
            transport.insert("permit_without_stream".to_string(), Value::Bool(true));
        }
        "http" | "h2" => {
            transport.insert("type".to_string(), Value::String("http".to_string()));
            if let Some(path) = query_params.get("path") {
                transport.insert("path".to_string(), Value::String(path.to_string()));
            }
            if let Some(host) = query_params.get("host") {
                let hosts: Vec<Value> = host
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                if !hosts.is_empty() {
                    transport.insert("host".to_string(), Value::Array(hosts));
                }
            }
            transport.insert("idle_timeout".to_string(), Value::String("15s".to_string()));
            transport.insert("ping_timeout".to_string(), Value::String("15s".to_string()));
        }
        "xhttp" | "splithttp" => {
            transport.insert("type".to_string(), Value::String("http".to_string()));
            if let Some(path) = query_params.get("path") {
                transport.insert("path".to_string(), Value::String(path.to_string()));
            }
            if let Some(host) = query_params.get("host") {
                let hosts: Vec<Value> = host
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                if !hosts.is_empty() {
                    transport.insert("host".to_string(), Value::Array(hosts));
                }
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
            if b64_part.contains('@') {
                let url = match Url::parse(trimmed) {
                    Ok(url) => url,
                    Err(_) => continue,
                };
                let (Some(server), Some(port)) = (url.host_str(), url.port()) else {
                    continue;
                };
                let uuid = percent_encoding::percent_decode_str(url.username())
                    .decode_utf8_lossy()
                    .into_owned();
                if uuid.is_empty() || port == 0 {
                    continue;
                }
                let query: HashMap<String, String> = url.query_pairs().into_owned().collect();
                let mut fields = HashMap::from([
                    ("server".into(), serde_json::json!(server)),
                    ("server_port".into(), serde_json::json!(port)),
                    ("uuid".into(), serde_json::json!(uuid)),
                    (
                        "alter_id".into(),
                        serde_json::json!(query
                            .get("aid")
                            .and_then(|s| s.parse::<u16>().ok())
                            .unwrap_or(0)),
                    ),
                    (
                        "security".into(),
                        serde_json::json!(query
                            .get("encryption")
                            .map(String::as_str)
                            .unwrap_or("auto")),
                    ),
                ]);
                if let Some(tls) = parse_tls_settings(&query) {
                    fields.insert("tls".into(), tls);
                }
                if let Some(transport) = parse_transport_settings(&query) {
                    fields.insert("transport".into(), transport);
                }
                let tag = url
                    .fragment()
                    .map(|tag| {
                        percent_encoding::percent_decode_str(tag)
                            .decode_utf8_lossy()
                            .into_owned()
                    })
                    .unwrap_or_else(|| "VMess Proxy".into());
                outbounds.push(SingBoxOutbound {
                    outbound_type: "vmess".into(),
                    tag,
                    fields,
                });
                continue;
            }
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
                Some(Value::Number(n)) => n
                    .as_u64()
                    .and_then(|port| u16::try_from(port).ok())
                    .unwrap_or(0),
                Some(Value::String(s)) => s.parse::<u16>().unwrap_or(0),
                _ => 0,
            };

            let uuid = json
                .get("id")
                .and_then(|u| u.as_str())
                .unwrap_or("")
                .to_string();
            let alter_id = match json.get("aid") {
                Some(Value::Number(n)) => n
                    .as_u64()
                    .and_then(|id| u16::try_from(id).ok())
                    .unwrap_or(0),
                Some(Value::String(s)) => s.parse::<u16>().unwrap_or(0),
                _ => 0,
            };

            if server.trim().is_empty() || port == 0 || uuid.trim().is_empty() {
                continue;
            }

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
        if server.trim().is_empty() || port == 0 {
            continue;
        }

        let mut fields = HashMap::new();
        fields.insert("server".to_string(), Value::String(server));
        fields.insert("server_port".to_string(), Value::Number(port.into()));

        match url.scheme() {
            "vless" => {
                // vless://[uuid]@host:port?query#tag
                let uuid = percent_encoding::percent_decode_str(url.username())
                    .decode_utf8_lossy()
                    .into_owned();
                if uuid.trim().is_empty() {
                    continue;
                }
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
                let password = percent_encoding::percent_decode_str(url.username())
                    .decode_utf8_lossy()
                    .into_owned();
                if password.is_empty() {
                    continue;
                }
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
            "hysteria2" | "hy2" => {
                let username =
                    percent_encoding::percent_decode_str(url.username()).decode_utf8_lossy();
                let password = url
                    .password()
                    .map(|pw| {
                        format!(
                            "{}:{}",
                            username,
                            percent_encoding::percent_decode_str(pw).decode_utf8_lossy()
                        )
                    })
                    .unwrap_or_else(|| username.into_owned());
                if password.is_empty() || port == 0 || fields["server"] == "" {
                    continue;
                }
                fields.insert("password".into(), Value::String(password));
                let query: HashMap<String, String> = url.query_pairs().into_owned().collect();
                let mut tls = serde_json::json!({"enabled": true});
                if let Some(sni) = query.get("sni") {
                    tls["server_name"] = serde_json::json!(sni);
                }
                if query
                    .get("insecure")
                    .or_else(|| query.get("allowInsecure"))
                    .is_some_and(|v| v == "1" || v == "true")
                {
                    tls["insecure"] = serde_json::json!(true);
                }
                fields.insert("tls".into(), tls);
                if let (Some(kind), Some(secret)) = (query.get("obfs"), query.get("obfs-password"))
                {
                    if !kind.is_empty() && !secret.is_empty() {
                        fields.insert(
                            "obfs".into(),
                            serde_json::json!({"type":kind,"password":secret}),
                        );
                    }
                }
                for (key, params) in [("up_mbps", "upmbps"), ("down_mbps", "downmbps")] {
                    if let Some(rate) = query.get(params).and_then(|s| s.parse::<u32>().ok()) {
                        fields.insert(key.into(), serde_json::json!(rate));
                    }
                }
                outbounds.push(SingBoxOutbound {
                    outbound_type: "hysteria2".into(),
                    tag,
                    fields,
                });
            }
            "wireguard" | "wg" => {
                let private_key = percent_encoding::percent_decode_str(url.username())
                    .decode_utf8_lossy()
                    .into_owned();
                let query: HashMap<String, String> = url.query_pairs().into_owned().collect();
                let public_key = query.get("publickey").or_else(|| query.get("public_key"));
                let address = query.get("address").or_else(|| query.get("ip"));
                if private_key.is_empty()
                    || public_key.is_none_or(String::is_empty)
                    || address.is_none_or(String::is_empty)
                    || port == 0
                    || fields["server"] == ""
                {
                    continue;
                }
                fields.insert("private_key".into(), serde_json::json!(private_key));
                fields.insert(
                    "peer_public_key".into(),
                    serde_json::json!(public_key.unwrap()),
                );
                fields.insert(
                    "address".into(),
                    serde_json::json!(address
                        .unwrap()
                        .split(',')
                        .map(str::trim)
                        .filter(|part| !part.is_empty())
                        .collect::<Vec<_>>()),
                );
                if let Some(psk) = query.get("psk").or_else(|| query.get("presharedkey")) {
                    fields.insert("pre_shared_key".into(), serde_json::json!(psk));
                }
                if let Some(reserved) = query.get("reserved") {
                    let bytes: Option<Vec<u8>> = reserved
                        .split(',')
                        .map(|part| part.trim().parse::<u8>().ok())
                        .collect();
                    if bytes.as_ref().is_some_and(|bytes| bytes.len() == 3) {
                        fields.insert("reserved".into(), serde_json::json!(bytes.unwrap()));
                    }
                }
                if let Some(mtu) = query.get("mtu").and_then(|s| s.parse::<u16>().ok()) {
                    fields.insert("mtu".into(), serde_json::json!(mtu));
                }
                outbounds.push(SingBoxOutbound {
                    outbound_type: "wireguard".into(),
                    tag,
                    fields,
                });
            }
            "tuic" => {
                let uuid = percent_encoding::percent_decode_str(url.username())
                    .decode_utf8_lossy()
                    .into_owned();
                let password = url
                    .password()
                    .map(|s| {
                        percent_encoding::percent_decode_str(s)
                            .decode_utf8_lossy()
                            .into_owned()
                    })
                    .unwrap_or_default();
                if uuid.is_empty() || password.is_empty() || port == 0 || fields["server"] == "" {
                    continue;
                }
                fields.insert("uuid".into(), Value::String(uuid));
                fields.insert("password".into(), Value::String(password));
                let query: HashMap<String, String> = url.query_pairs().into_owned().collect();
                fields.insert(
                    "congestion_control".into(),
                    serde_json::json!(query
                        .get("congestion_control")
                        .or_else(|| query.get("congestion-control"))
                        .map(String::as_str)
                        .unwrap_or("bbr")),
                );
                if let Some(mode) = query
                    .get("udp_relay_mode")
                    .or_else(|| query.get("udp-relay-mode"))
                {
                    fields.insert("udp_relay_mode".into(), serde_json::json!(mode));
                }
                let mut tls = serde_json::json!({"enabled": true});
                if let Some(sni) = query.get("sni") {
                    tls["server_name"] = serde_json::json!(sni);
                }
                if query
                    .get("insecure")
                    .or_else(|| query.get("allowInsecure"))
                    .is_some_and(|v| v == "1" || v == "true")
                {
                    tls["insecure"] = serde_json::json!(true);
                }
                fields.insert("tls".into(), tls);
                outbounds.push(SingBoxOutbound {
                    outbound_type: "tuic".into(),
                    tag,
                    fields,
                });
            }
            "socks5" | "socks" => {
                let username = percent_encoding::percent_decode_str(url.username())
                    .decode_utf8_lossy()
                    .into_owned();
                let password = url
                    .password()
                    .map(|p| {
                        percent_encoding::percent_decode_str(p)
                            .decode_utf8_lossy()
                            .into_owned()
                    })
                    .unwrap_or_default();
                fields.insert("version".to_string(), Value::String("5".to_string()));
                if !username.is_empty() {
                    fields.insert("username".to_string(), Value::String(username));
                }
                if !password.is_empty() {
                    fields.insert("password".to_string(), Value::String(password));
                }
                outbounds.push(SingBoxOutbound {
                    outbound_type: "socks".to_string(),
                    tag,
                    fields,
                });
            }
            "http" | "https" => {
                let username = percent_encoding::percent_decode_str(url.username())
                    .decode_utf8_lossy()
                    .into_owned();
                let password = url
                    .password()
                    .map(|p| {
                        percent_encoding::percent_decode_str(p)
                            .decode_utf8_lossy()
                            .into_owned()
                    })
                    .unwrap_or_default();
                if !username.is_empty() {
                    fields.insert("username".to_string(), Value::String(username));
                }
                if !password.is_empty() {
                    fields.insert("password".to_string(), Value::String(password));
                }
                if url.scheme() == "https" {
                    let mut tls = HashMap::new();
                    tls.insert("enabled".to_string(), Value::Bool(true));
                    fields.insert("tls".to_string(), Value::Object(tls.into_iter().collect()));
                }
                outbounds.push(SingBoxOutbound {
                    outbound_type: "http".to_string(),
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
    fn test_encoded_ws_early_data_and_unpadded_base64() {
        let uri = "vless://uuid@proxy.example:443?security=tls&sni=tls.example&type=ws&host=cdn.example&path=%2Ftunnel%3Fed%3D2048#VLESS";
        let nodes = adapt(uri.as_bytes()).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].fields["transport"]["path"], "/tunnel?ed=2048");
        assert_eq!(
            nodes[0].fields["transport"]["headers"]["Host"],
            "cdn.example"
        );
        assert_eq!(nodes[0].fields["tls"]["server_name"], "tls.example");

        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            r#"{"add":"proxy.example","port":"443","id":"uuid","aid":"1"}"#,
        );
        let nodes = adapt(format!("vmess://{}", encoded).as_bytes()).unwrap();
        assert_eq!(nodes[0].fields["alter_id"], 1);
        assert_eq!(nodes[0].fields["server_port"], 443);
    }

    #[test]
    fn test_incomplete_raw_nodes_are_skipped() {
        let nodes = adapt(b"vmess://uuid@proxy.example:443?security=tls\nvless://@proxy.example:443\ntrojan://@proxy.example:443\nvless://uuid@proxy.example:443").unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].outbound_type, "vmess");
        assert_eq!(nodes[1].outbound_type, "vless");
    }

    #[test]
    fn test_tuic_udp_relay_aliases() {
        let nodes = adapt(b"tuic://uuid:password@proxy.example:443?congestion-control=cubic&udp-relay-mode=native").unwrap();
        assert_eq!(nodes[0].fields["congestion_control"], "cubic");
        assert_eq!(nodes[0].fields["udp_relay_mode"], "native");
    }

    #[test]
    fn test_vmess_uri_style_import() {
        let uri = "vmess://user-uuid@proxy.example:443?security=tls&sni=tls.example&type=ws&path=%2Ftunnel%3Fed%3D1024&aid=4#VMess";
        let nodes = adapt(uri.as_bytes()).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].fields["server_port"], 443);
        assert_eq!(nodes[0].fields["alter_id"], 4);
        assert_eq!(nodes[0].fields["tls"]["server_name"], "tls.example");
        assert_eq!(nodes[0].fields["transport"]["path"], "/tunnel?ed=1024");
    }

    #[test]
    fn test_wireguard_share_link() {
        let nodes = adapt(b"wg://privatekey@node.example:51820?publickey=peerkey&address=10.0.0.2%2F32&psk=shared&reserved=1%2C2%2C3#WG").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].outbound_type, "wireguard");
        assert_eq!(
            nodes[0].fields["address"],
            serde_json::json!(["10.0.0.2/32"])
        );
        assert_eq!(nodes[0].fields["peer_public_key"], "peerkey");
        assert_eq!(nodes[0].fields["pre_shared_key"], "shared");
        assert_eq!(nodes[0].fields["reserved"], serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_udp_share_link_protocols() {
        let nodes = adapt(b"hy2://secret@node.example:443?sni=tls.example&obfs=salamander&obfs-password=abc&upmbps=10#HY2\ntuic://uuid:secret@node.example:443?sni=tls.example&congestion_control=bbr#TUIC").unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].outbound_type, "hysteria2");
        assert_eq!(nodes[0].fields["obfs"]["password"], "abc");
        assert_eq!(nodes[0].fields["up_mbps"], 10);
        assert_eq!(nodes[1].outbound_type, "tuic");
        assert_eq!(nodes[1].fields["uuid"], "uuid");
        assert_eq!(nodes[1].fields["tls"]["server_name"], "tls.example");
        assert!(
            adapt(b"hy2://@node.example:443\ntuic://uuid@node.example:443")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_vmess_json_string_aid_and_overflow_port() {
        let vmess = serde_json::json!({
            "ps": "test", "add": "proxy.example", "port": "443", "id": "uuid", "aid": "5"
        });
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            vmess.to_string(),
        );
        let nodes = adapt(format!("vmess://{}", encoded).as_bytes()).unwrap();
        assert_eq!(nodes[0].fields["server_port"], 443);
        assert_eq!(nodes[0].fields["alter_id"], 5);

        let vmess = serde_json::json!({
            "ps": "overflow", "add": "proxy.example", "port": 65536, "id": "uuid", "aid": 65536
        });
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            vmess.to_string(),
        );
        let nodes = adapt(format!("vmess://{}", encoded).as_bytes()).unwrap();
        assert!(
            nodes.is_empty(),
            "nodes with out-of-range ports must be rejected"
        );

        let vmess = serde_json::json!({
            "ps": "overflow-aid", "add": "proxy.example", "port": 443, "id": "uuid", "aid": 65536
        });
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            vmess.to_string(),
        );
        let nodes = adapt(format!("vmess://{}", encoded).as_bytes()).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].fields["alter_id"], 0);
    }

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
        assert!(tls.get("enabled").unwrap().as_bool().unwrap());
        assert_eq!(tls.get("server_name").unwrap().as_str().unwrap(), "aka.ms");
        assert!(tls.get("insecure").unwrap().as_bool().unwrap());
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
    fn test_adapt_httpupgrade_host_field() {
        let uri = "vless://00000000-0000-0000-0000-000000000000@proxy.example:443?type=httpupgrade&host=cdn.example&path=%2Fazure&security=tls&sni=tls.example&alpn=h2%2Chttp%2F1.1#Test";
        let nodes = adapt(uri.as_bytes()).unwrap();
        let transport = &nodes[0].fields["transport"];
        assert_eq!(transport["type"], "httpupgrade");
        assert_eq!(transport["host"], "cdn.example");
        assert_eq!(transport["path"], "/azure");
        assert!(transport.get("headers").is_none());
        assert_eq!(nodes[0].fields["tls"]["server_name"], "tls.example");
    }

    #[test]
    fn test_adapt_preserves_pristine_node_fields_without_allow_insecure() {
        // Real-world zero-data URI where allowInsecure was omitted from query parameters:
        // Importer preserves exact link intent verbatim; runtime generator handles singbox adaptations.
        let uri = "vless://7efbd5f5-f9f9-425e-b0eb-d46a8a6f3bd9@azure.ezgateway.net:443?alpn=h2%2Chttp%2F1.1&encryption=none&fp=chrome&host=azure.ezgateway.net&path=%2Fazure&security=tls&sni=cloud.microsoft&type=ws#One%20Piece%20SG-Prabathya";
        let nodes = adapt(uri.as_bytes()).unwrap();
        assert_eq!(nodes.len(), 1);
        let node = &nodes[0];
        assert_eq!(node.fields["tls"]["server_name"], "cloud.microsoft");
        assert!(node.fields["tls"].get("insecure").is_none());
        assert_eq!(node.fields["server"], "azure.ezgateway.net");
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
        assert!(tls.get("enabled").unwrap().as_bool().unwrap());
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

    #[test]
    fn test_adapt_socks5_and_http_uris() {
        let uris = "socks5://user:pass@1.2.3.4:1080#MySocks\nhttp://admin:secret@5.6.7.8:8080#MyHttp\nhttps://9.10.11.12:8443#MyHttps";
        let nodes = adapt(uris.as_bytes()).unwrap();
        assert_eq!(nodes.len(), 3);

        // Socks5
        assert_eq!(nodes[0].outbound_type, "socks");
        assert_eq!(nodes[0].tag, "MySocks");
        assert_eq!(nodes[0].fields["server"], "1.2.3.4");
        assert_eq!(nodes[0].fields["server_port"], 1080);
        assert_eq!(nodes[0].fields["version"], "5");
        assert_eq!(nodes[0].fields["username"], "user");
        assert_eq!(nodes[0].fields["password"], "pass");

        // Plain HTTP
        assert_eq!(nodes[1].outbound_type, "http");
        assert_eq!(nodes[1].tag, "MyHttp");
        assert_eq!(nodes[1].fields["server"], "5.6.7.8");
        assert_eq!(nodes[1].fields["server_port"], 8080);
        assert_eq!(nodes[1].fields["username"], "admin");
        assert_eq!(nodes[1].fields["password"], "secret");
        assert!(nodes[1].fields.get("tls").is_none());

        // HTTPS with TLS
        assert_eq!(nodes[2].outbound_type, "http");
        assert_eq!(nodes[2].tag, "MyHttps");
        assert_eq!(nodes[2].fields["server"], "9.10.11.12");
        assert_eq!(nodes[2].fields["server_port"], 8443);
        assert_eq!(nodes[2].fields["tls"]["enabled"], true);
    }

    #[test]
    fn test_reality_without_fp_defaults_to_chrome_utls() {
        let uri = "vless://00000000-0000-0000-0000-000000000000@server.example:443?security=reality&pbk=fakekey&sid=1234&sni=example.com#RealityNoFP";
        let nodes = adapt(uri.as_bytes()).unwrap();
        assert_eq!(nodes.len(), 1);
        let tls = &nodes[0].fields["tls"];
        assert_eq!(tls["reality"]["enabled"], true);
        assert_eq!(tls["reality"]["public_key"], "fakekey");
        assert_eq!(tls["reality"]["short_id"], "1234");
        assert_eq!(tls["utls"]["enabled"], true);
        assert_eq!(tls["utls"]["fingerprint"], "chrome");
    }

    #[test]
    fn test_grpc_and_h2_transport_options() {
        let uris = "vless://00000000-0000-0000-0000-000000000000@server.example:443?type=grpc&serviceName=my-service#GrpcNode\nvless://00000000-0000-0000-0000-000000000000@server.example:443?type=http&path=%2Fh2path&host=h2.example.com#H2Node";
        let nodes = adapt(uris.as_bytes()).unwrap();
        assert_eq!(nodes.len(), 2);

        // gRPC
        let grpc_transport = &nodes[0].fields["transport"];
        assert_eq!(grpc_transport["type"], "grpc");
        assert_eq!(grpc_transport["service_name"], "my-service");
        assert_eq!(grpc_transport["idle_timeout"], "15s");
        assert_eq!(grpc_transport["ping_timeout"], "15s");
        assert_eq!(grpc_transport["permit_without_stream"], true);

        // H2
        let h2_transport = &nodes[1].fields["transport"];
        assert_eq!(h2_transport["type"], "http");
        assert_eq!(h2_transport["path"], "/h2path");
        assert_eq!(h2_transport["host"], serde_json::json!(["h2.example.com"]));
    }
}
