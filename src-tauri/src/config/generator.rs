use super::adapters::SingBoxOutbound;
use super::build_route_exclude_addresses;
use super::resolve_dns_address;
use super::resolve_server_ips;
use serde_json::Value;
use std::collections::HashMap;

/// Compilation-only overlay; never serialized into a sing-box outbound.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BugHostMode {
    #[default]
    Disabled,
    SniSpoof,
    CdnFronting,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BugHostSettings {
    pub mode: BugHostMode,
    pub bug_host: String,
    pub original_server: String,
}

/// Extract a valid WebSocket early-data parameter without disturbing other query parameters.
pub fn parse_ws_path_and_early_data(raw_path: &str) -> (String, Option<u32>) {
    let Some((path, query)) = raw_path.split_once('?') else {
        return (raw_path.to_string(), None);
    };
    let mut early_data = None;
    let mut retained = Vec::new();
    for part in query.split('&') {
        if let Some(value) = part.strip_prefix("ed=") {
            if let Ok(size) = value.parse::<u32>() {
                if size > 0 {
                    if early_data.replace(size).is_some() {
                        return (raw_path.to_string(), None);
                    }
                    continue;
                }
            }
        }
        retained.push(part);
    }
    if early_data.is_none() {
        return (raw_path.to_string(), None);
    }
    let clean = if retained.is_empty() {
        path.to_string()
    } else {
        format!("{}?{}", path, retained.join("&"))
    };
    (clean, early_data)
}

fn clean_host(value: &str) -> Result<String, String> {
    let value = value.trim();
    let bare = value.strip_prefix('[').and_then(|s| s.strip_suffix(']'));
    if let Some(ip) = bare {
        return ip
            .parse::<std::net::Ipv6Addr>()
            .map(|ip| ip.to_string())
            .map_err(|_| "Invalid bracketed IPv6 server address".to_string());
    }
    if let Ok(ip) = value.parse::<std::net::IpAddr>() {
        return Ok(ip.to_string());
    }
    if value.is_empty()
        || value.len() > 253
        || value.contains(':')
        || value.trim_end_matches('.').split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err("Invalid server host (expected a hostname or IP without a port)".to_string());
    }
    Ok(value.to_string())
}

fn prepare_node_outbound(
    node: &mut SingBoxOutbound,
    bug_host: Option<&BugHostSettings>,
) -> Result<(), String> {
    let Some(server) = node.fields.get("server").and_then(Value::as_str) else {
        if bug_host.is_some_and(|settings| settings.mode != BugHostMode::Disabled) {
            return Err(format!("Bug-host node '{}' has no server", node.tag));
        }
        // Some legacy direct/group outbounds have no dial address.
        return Ok(());
    };
    let server = clean_host(server)?;
    node.fields
        .insert("server".into(), Value::String(server.clone()));

    if let Some(settings) = bug_host.filter(|s| s.mode != BugHostMode::Disabled) {
        let original = clean_host(&settings.original_server)?;
        let bug = clean_host(&settings.bug_host)?;
        if original != server || bug.parse::<std::net::IpAddr>().is_ok() {
            return Err(format!(
                "Invalid or stale bug-host settings for node '{}'",
                node.tag
            ));
        }
        if settings.mode == BugHostMode::CdnFronting && original.parse::<std::net::IpAddr>().is_ok()
        {
            return Err(format!(
                "CDN fronting requires an original domain for node '{}'",
                node.tag
            ));
        }
        if !matches!(node.outbound_type.as_str(), "vless" | "vmess" | "trojan") {
            return Err(format!(
                "Bug-host mode requires a TLS-capable TCP outbound: '{}'",
                node.tag
            ));
        }
        let tls = node
            .fields
            .get_mut("tls")
            .and_then(Value::as_object_mut)
            .filter(|tls| tls.get("enabled").and_then(Value::as_bool) == Some(true))
            .ok_or_else(|| format!("Bug-host mode requires TLS for node '{}'", node.tag))?;
        if tls.get("reality").and_then(|r| r.get("enabled")) == Some(&Value::Bool(true)) {
            return Err(format!(
                "Bug-host mode is incompatible with Reality for node '{}'",
                node.tag
            ));
        }
        tls.insert(
            "server_name".into(),
            Value::String(if settings.mode == BugHostMode::SniSpoof {
                bug.clone()
            } else {
                original.clone()
            }),
        );
        if settings.mode == BugHostMode::SniSpoof {
            tls.insert("insecure".into(), serde_json::json!(true));
        }
        let transport = node
            .fields
            .get_mut("transport")
            .and_then(Value::as_object_mut);
        if let Some(transport) = transport {
            match transport.get("type").and_then(Value::as_str) {
                Some("ws") => {
                    let headers = transport
                        .entry("headers")
                        .or_insert_with(|| serde_json::json!({}));
                    let headers = headers.as_object_mut().ok_or("Invalid WS headers")?;
                    headers.retain(|key, _| !key.eq_ignore_ascii_case("host"));
                    headers.insert(
                        "Host".into(),
                        Value::String(if settings.mode == BugHostMode::SniSpoof {
                            bug.clone()
                        } else {
                            original.clone()
                        }),
                    );
                }
                Some("httpupgrade") => {
                    transport.insert(
                        "host".into(),
                        Value::String(if settings.mode == BugHostMode::SniSpoof {
                            bug.clone()
                        } else {
                            original.clone()
                        }),
                    );
                    if let Some(headers) =
                        transport.get_mut("headers").and_then(Value::as_object_mut)
                    {
                        headers.retain(|key, _| !key.eq_ignore_ascii_case("host"));
                    }
                }
                Some("http") => {
                    transport.insert(
                        "host".into(),
                        serde_json::json!([if settings.mode == BugHostMode::SniSpoof {
                            bug.clone()
                        } else {
                            original.clone()
                        }]),
                    );
                }
                Some("grpc") | Some("quic") => {
                    return Err(format!(
                        "Bug-host mode cannot set HTTP Host for node '{}'",
                        node.tag
                    ))
                }
                _ => {}
            }
        }
        if settings.mode == BugHostMode::CdnFronting {
            node.fields.insert("server".into(), Value::String(bug));
        }
    }

    let transport_type = node
        .fields
        .get("transport")
        .and_then(|t| t.get("type"))
        .and_then(Value::as_str)
        .map(str::to_string);
    if transport_type.as_deref() == Some("ws") {
        if let Some(transport) = node
            .fields
            .get_mut("transport")
            .and_then(Value::as_object_mut)
        {
            if let Some(raw_path) = transport.get("path").and_then(Value::as_str) {
                let (path, early_data) = parse_ws_path_and_early_data(raw_path);
                if let Some(size) = early_data {
                    transport.insert("path".into(), Value::String(path));
                    transport
                        .entry("max_early_data")
                        .or_insert(serde_json::json!(size));
                    transport
                        .entry("early_data_header_name")
                        .or_insert(Value::String("Sec-WebSocket-Protocol".into()));
                }
            }
        }
    }
    if node.fields.get("flow").and_then(Value::as_str) == Some("xtls-rprx-vision")
        && !matches!(transport_type.as_deref(), None | Some("tcp"))
    {
        node.fields.remove("flow");
    }
    if node
        .fields
        .get("flow")
        .and_then(Value::as_str)
        .is_some_and(|flow| flow.contains("vision"))
    {
        node.fields.remove("multiplex");
    }
    if transport_type.as_deref() == Some("grpc") {
        if let Some(transport) = node
            .fields
            .get_mut("transport")
            .and_then(Value::as_object_mut)
        {
            transport
                .entry("idle_timeout".to_string())
                .or_insert(serde_json::json!("15s"));
            transport
                .entry("ping_timeout".to_string())
                .or_insert(serde_json::json!("15s"));
            transport
                .entry("permit_without_stream".to_string())
                .or_insert(serde_json::json!(true));
        }
    }
    if let Some(tls) = node.fields.get("tls").and_then(Value::as_object) {
        if tls.get("reality").and_then(|r| r.get("enabled")) == Some(&Value::Bool(true)) {
            let key = tls
                .get("reality")
                .and_then(|r| r.get("public_key"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let fingerprint = tls
                .get("utls")
                .and_then(Value::as_object)
                .filter(|utls| utls.get("enabled").and_then(Value::as_bool) == Some(true))
                .and_then(|utls| utls.get("fingerprint"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if tls.get("enabled").and_then(Value::as_bool) != Some(true)
                || tls.get("insecure").and_then(Value::as_bool) == Some(true)
                || key.trim().is_empty()
                || fingerprint.trim().is_empty()
            {
                return Err(format!(
                    "Invalid Reality TLS settings for node '{}'",
                    node.tag
                ));
            }
        }
    }
    Ok(())
}

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
    // A detour to the plain "direct" outbound is rejected by newer sing-box
    // ("detour to an empty direct outbound makes no sense") and is redundant:
    // DNS servers without a detour already dial directly.
    if let Some(d) = detour {
        if !d.is_empty() && d != "direct" {
            server_obj.insert("detour".to_string(), serde_json::json!(d));
        }
    }

    if address == "local" || address.is_empty() {
        server_obj.insert("type".to_string(), serde_json::json!("local"));
    } else if address.starts_with("https://") {
        // sing-box `https` servers take host / server_port / path separately,
        // never a full URL in `server`.
        server_obj.insert("type".to_string(), serde_json::json!("https"));
        let stripped = address.strip_prefix("https://").unwrap();
        let (authority, path) = match stripped.find('/') {
            Some(idx) => (&stripped[..idx], &stripped[idx..]),
            None => (stripped, ""),
        };
        let (host, port) = split_host_port(authority, 443);
        server_obj.insert("server".to_string(), serde_json::json!(host));
        if port != 443 {
            server_obj.insert("server_port".to_string(), serde_json::json!(port));
        }
        if !path.is_empty() && path != "/dns-query" {
            server_obj.insert("path".to_string(), serde_json::json!(path));
        }
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
    pub block_quic: bool,
    pub routing_mode: crate::state::RoutingMode,
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
            primary_dns: "tls://1.1.1.1".to_string(),
            fallback_dns: "tls://8.8.8.8".to_string(),
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
            block_quic: false,
            routing_mode: crate::state::RoutingMode::Rule,
        }
    }
}

pub fn should_include_tun_interface_name(target_os: &str) -> bool {
    target_os != "macos"
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
    app_data_dir: &std::path::Path,
    outbound_groups: &[crate::config::rules::OutboundGroup],
    host_dns_servers: &[std::net::IpAddr],
) -> Result<String, String> {
    generate_singbox_config_with_bug_hosts(
        mixed_port,
        outbounds,
        proxy_mode,
        dns_address,
        listen_address,
        tun_settings,
        default_outbound_tag,
        user_rules,
        rule_sets,
        app_data_dir,
        outbound_groups,
        host_dns_servers,
        &HashMap::new(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn generate_singbox_config_with_bug_hosts(
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
    bug_hosts: &HashMap<String, BugHostSettings>,
) -> Result<String, String> {
    let mut overlay_tags = std::collections::HashSet::new();
    for node in &mut outbounds {
        if bug_hosts.contains_key(&node.tag) && !overlay_tags.insert(node.tag.clone()) {
            return Err(format!(
                "Ambiguous bug-host settings for node '{}'",
                node.tag
            ));
        }
        prepare_node_outbound(node, bug_hosts.get(&node.tag))?;
    }
    if bug_hosts.keys().any(|tag| !overlay_tags.contains(tag)) {
        return Err("Bug-host settings reference a missing outbound".into());
    }
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
    if !dialable_node_tags.is_empty() {
        selector_tags.push("auto".to_string());
    }
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

    // Add automatic selection after the root selector without changing the selected node.
    if !dialable_node_tags.is_empty() {
        final_outbounds.push(serde_json::json!({
            "type": "urltest",
            "tag": "auto",
            "outbounds": dialable_node_tags.iter().take(crate::config::validator::MAX_AUTOMATED_GROUP_NODES).collect::<Vec<_>>(),
            "url": "https://cp.cloudflare.com/generate_204",
            "interval": "600s",
            "tolerance": 50
        }));
    }

    // Next, push user-defined OutboundGroup outbounds
    final_outbounds.extend(compiled_groups);

    // Push the parsed node outbounds
    for node in outbounds {
        if node.outbound_type == "wireguard" {
            let address = node
                .fields
                .get("address")
                .or_else(|| node.fields.get("local_address"))
                .filter(|addr| {
                    addr.as_array().is_some_and(|arr| !arr.is_empty())
                        || addr.as_str().is_some_and(|s| !s.is_empty())
                })
                .ok_or_else(|| format!("WireGuard node '{}' requires a local address", node.tag))?;
            let address = if address.is_array() {
                address.clone()
            } else {
                serde_json::json!([address])
            };

            let private_key = node
                .fields
                .get("private_key")
                .or_else(|| node.fields.get("secret_key"))
                .and_then(|k| k.as_str())
                .filter(|key| !key.trim().is_empty())
                .ok_or_else(|| format!("WireGuard node '{}' requires a private key", node.tag))?
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
                .filter(|key| !key.trim().is_empty())
                .ok_or_else(|| format!("WireGuard node '{}' requires a peer public key", node.tag))?
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
            if let Some(psk) = node.fields.get("pre_shared_key") {
                peer_obj["pre_shared_key"] = psk.clone();
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
            if matches!(outbound_type, "vless" | "vmess" | "trojan")
                && !obj
                    .get("flow")
                    .and_then(Value::as_str)
                    .is_some_and(|flow| flow.contains("vision"))
                && !obj.contains_key("multiplex")
            {
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

        // Preserve the imported ALPN, including its absence, except for WebSocket and HTTPUpgrade.
        // WebSocket and HTTPUpgrade transports use HTTP/1.1 Upgrade mechanisms. If TLS negotiates
        // "h2", the server expects HTTP/2 binary framing while the client sends HTTP/1.1 ASCII text,
        // causing the connection to fail with EOF or malformed HTTP response. Strip "h2" so negotiation
        // uses HTTP/1.1.
        let transport_type = obj
            .get("transport")
            .and_then(|t| t.get("type"))
            .and_then(|t| t.as_str())
            .map(str::to_string);
        if matches!(transport_type.as_deref(), Some("ws" | "httpupgrade")) {
            if let Some(tls) = obj.get_mut("tls").and_then(Value::as_object_mut) {
                if let Some(alpn) = tls.get_mut("alpn").and_then(Value::as_array_mut) {
                    alpn.retain(|v| v.as_str() != Some("h2"));
                    if alpn.is_empty() {
                        *alpn = vec![Value::String("http/1.1".to_string())];
                    }
                }
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

        // If outbound server is a domain name, pre-resolve it to avoid DNS deadlock on zero-data ISPs
        let server_str = obj
            .get("server")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let server_port = obj
            .get("server_port")
            .and_then(Value::as_u64)
            .unwrap_or(443) as u16;

        if !server_str.is_empty() && server_str.parse::<std::net::IpAddr>().is_err() {
            let resolved_ips = crate::config::resolve_host_ip_addrs(&server_str, server_port);
            let chosen_ip = resolved_ips
                .iter()
                .find(|ip| ip.is_ipv4())
                .or_else(|| resolved_ips.first());
            if let Some(ip) = chosen_ip {
                // If TLS is configured on this outbound, ensure SNI server_name is preserved
                if let Some(tls) = obj.get_mut("tls").and_then(Value::as_object_mut) {
                    let has_sni = tls
                        .get("server_name")
                        .and_then(Value::as_str)
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);
                    if !has_sni {
                        tls.insert("server_name".to_string(), Value::String(server_str.clone()));
                    }
                }
                obj.insert("server".to_string(), Value::String(ip.to_string()));
            }
        }

        // Extract transport host before mutable borrow of obj
        let transport_host = obj
            .get("transport")
            .and_then(|t| {
                t.get("host").or_else(|| {
                    t.get("headers")
                        .and_then(|h| h.get("Host").or_else(|| h.get("host")))
                })
            })
            .and_then(Value::as_str)
            .map(|h| h.trim().to_lowercase());

        // If TLS is configured, detect if SNI is spoofed for ISP zero-data / bug host.
        // When transport host matches the server domain, but SNI is different,
        // the server's certificate is valid for the server domain, not the spoofed SNI.
        // Auto-set insecure to prevent x509 verification failure.
        if let Some(tls) = obj.get_mut("tls").and_then(Value::as_object_mut) {
            let is_reality = tls
                .get("reality")
                .and_then(|r| r.get("enabled"))
                == Some(&Value::Bool(true));

            if !is_reality && tls.get("enabled").and_then(Value::as_bool) == Some(true) {
                let is_already_insecure = tls
                    .get("insecure")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                if !is_already_insecure {
                    if let Some(sni) = tls.get("server_name").and_then(Value::as_str) {
                        let sni_clean = sni.trim().to_lowercase();
                        let server_clean = server_str.trim().to_lowercase();

                        if let Some(ref th) = transport_host {
                            let matches_th_and_server = !server_clean.is_empty()
                                && (th == &server_clean
                                    || server_clean.ends_with(&format!(".{}", th))
                                    || th.ends_with(&format!(".{}", server_clean)));
                            let sni_matches_server = sni_clean == server_clean
                                || server_clean.ends_with(&format!(".{}", sni_clean))
                                || sni_clean.ends_with(&format!(".{}", server_clean));

                            if matches_th_and_server && !sni_matches_server && !sni_clean.is_empty() {
                                tls.insert("insecure".to_string(), Value::Bool(true));
                            }
                        }
                    }
                }
            }
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
    let route_rules = crate::config::build_route_rules_with_quic(
        tun_settings.bypass_lan,
        tun_settings.sniff_enabled,
        tun_settings.block_quic,
        user_rules,
        rule_sets,
        geosite_db_exists,
        geoip_db_exists,
        outbound_groups,
        tun_settings.routing_mode,
    );

    // Determine effective route final outbound based on routing mode
    let effective_final_outbound = match tun_settings.routing_mode {
        crate::state::RoutingMode::SniZeroData | crate::state::RoutingMode::Global => {
            "proxy".to_string()
        }
        crate::state::RoutingMode::Direct => "direct".to_string(),
        crate::state::RoutingMode::Rule => tun_settings.final_outbound.clone(),
    };

    let mut route_section = serde_json::json!({
        "rules": route_rules,
        "final": effective_final_outbound,
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
        // First, add predefined static DNS answers for any server domains that resolve to IPs
        for domain in &server_domains {
            let ips = crate::config::resolve_host_ip_addrs(domain, 443);
            if !ips.is_empty() {
                let mut answers = Vec::new();
                for ip in ips {
                    match ip {
                        std::net::IpAddr::V4(v4) => {
                            answers.push(format!("{}. 300 IN A {}", domain, v4));
                        }
                        std::net::IpAddr::V6(v6) => {
                            answers.push(format!("{}. 300 IN AAAA {}", domain, v6));
                        }
                    }
                }
                if !answers.is_empty() {
                    dns_rules.push(serde_json::json!({
                        "domain": [domain],
                        "action": "predefined",
                        "answer": answers
                    }));
                }
            }
        }

        // Second, ensure fallback bootstrap rule to local-dns is always present
        dns_rules.push(serde_json::json!({
            "domain": server_domains,
            "server": "local-dns"
        }));
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
    if is_fakeip && proxy_mode == "tun" {
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

    // Determine DNS final server based on routing mode
    let dns_final = match tun_settings.routing_mode {
        crate::state::RoutingMode::Direct => "local-dns",
        _ => "remote-dns", // SniZeroData, Rule, Global all use remote-dns
    };

    // In TUN mode, maintain the complete Phase 2 DNS architecture:
    // remote-dns detoured through proxy for unblocked resolution, local-dns direct for bootstrap,
    // and server domain local-dns rule to prevent bootstrap deadlocks.
    // In System Proxy / Manual mode, local-dns is used directly for bootstrap/internal queries;
    // remote domain resolution is performed remotely by the outbound proxy protocol (HTTP CONNECT / SOCKS5).
    let mut dns_section = if proxy_mode == "tun" {
        serde_json::json!({
            "servers": dns_servers,
            "rules": dns_rules,
            "strategy": tun_settings.dns_strategy,
            "final": dns_final,
            "independent_cache": true
        })
    } else {
        serde_json::json!({
            "servers": [
                parse_dns_server("local-dns", &local_dns_addr, Some("direct"))
            ],
            "rules": dns_rules,
            "strategy": tun_settings.dns_strategy,
            "final": "local-dns",
            "independent_cache": true
        })
    };
    dns_section["disable_cache"] = serde_json::json!(!tun_settings.dns_caching);

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

    fn compile_test_node(
        fields: HashMap<String, Value>,
        mode: Option<BugHostSettings>,
    ) -> Result<Value, String> {
        let outbounds = vec![SingBoxOutbound {
            outbound_type: "vless".into(),
            tag: "sample-node".into(),
            fields,
        }];
        let overlays = mode
            .map(|settings| HashMap::from([("sample-node".into(), settings)]))
            .unwrap_or_default();
        let json = generate_singbox_config_with_bug_hosts(
            7890,
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
            &overlays,
        )?;
        let config: Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        Ok(config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["tag"] == "sample-node")
            .unwrap()
            .clone())
    }

    #[test]
    fn test_sni_spoof_and_early_data_generation() {
        let fields = HashMap::from([
            ("server".into(), serde_json::json!("proxy.example")),
            ("server_port".into(), serde_json::json!(443)),
            (
                "uuid".into(),
                serde_json::json!("00000000-0000-0000-0000-000000000000"),
            ),
            (
                "tls".into(),
                serde_json::json!({"enabled": true, "server_name": "original.example"}),
            ),
            (
                "transport".into(),
                serde_json::json!({"type":"ws", "path":"/tunnel?x=1&ed=2048", "headers":{"host":"old.example", "X-Test":"yes"}}),
            ),
        ]);
        let node = compile_test_node(
            fields.clone(),
            Some(BugHostSettings {
                mode: BugHostMode::SniSpoof,
                bug_host: "zoom.us".into(),
                original_server: "proxy.example".into(),
            }),
        )
        .unwrap();
        assert_eq!(node["server"], "proxy.example");
        assert_eq!(node["tls"]["server_name"], "zoom.us");
        assert_eq!(node["transport"]["headers"]["Host"], "zoom.us");
        assert!(node["transport"]["headers"].get("host").is_none());
        assert_eq!(node["transport"]["path"], "/tunnel?x=1");
        assert_eq!(node["transport"]["max_early_data"], 2048);
        assert_eq!(
            node["transport"]["early_data_header_name"],
            "Sec-WebSocket-Protocol"
        );
        assert_eq!(node["transport"]["headers"]["X-Test"], "yes");

        let fronted = compile_test_node(
            fields,
            Some(BugHostSettings {
                mode: BugHostMode::CdnFronting,
                bug_host: "cdn.example".into(),
                original_server: "proxy.example".into(),
            }),
        )
        .unwrap();
        assert_eq!(fronted["server"], "cdn.example");
        assert_eq!(fronted["tls"]["server_name"], "proxy.example");
        assert_eq!(fronted["transport"]["headers"]["Host"], "proxy.example");
    }

    #[test]
    fn test_generator_outbound_guardrails() {
        assert_eq!(
            parse_ws_path_and_early_data("/t?ed=bad&x=1"),
            ("/t?ed=bad&x=1".into(), None)
        );
        assert_eq!(
            parse_ws_path_and_early_data("/t?ed=2048&x=1"),
            ("/t?x=1".into(), Some(2048))
        );
        let mut fields = HashMap::from([
            ("server".into(), serde_json::json!("[2001:db8::1]")),
            ("tls".into(), serde_json::json!({"enabled":true})),
            (
                "transport".into(),
                serde_json::json!({"type":"ws", "path":"/t?ed=1024"}),
            ),
            ("flow".into(), serde_json::json!("xtls-rprx-vision")),
        ]);
        let node = compile_test_node(fields.clone(), None).unwrap();
        assert_eq!(node["server"], "2001:db8::1");
        assert!(node.get("flow").is_none());
        assert_eq!(node["transport"]["max_early_data"], 1024);
        fields.insert("tls".into(), serde_json::json!({"enabled":true,"insecure":true,"reality":{"enabled":true,"public_key":"key"}}));
        assert!(compile_test_node(fields.clone(), None)
            .unwrap_err()
            .contains("Reality"));
        fields.insert(
            "tls".into(),
            serde_json::json!({"enabled":true,"reality":{"enabled":true}}),
        );
        assert!(compile_test_node(fields.clone(), None)
            .unwrap_err()
            .contains("Reality"));
        fields.insert(
            "tls".into(),
            serde_json::json!({"enabled":true,"reality":{"enabled":true,"public_key":"key"}}),
        );
        assert!(compile_test_node(fields.clone(), None)
            .unwrap_err()
            .contains("Reality"));
        fields.insert(
            "tls".into(),
            serde_json::json!({"enabled":true,"reality":{"enabled":true,"public_key":"key"},"utls":{"enabled":false,"fingerprint":"chrome"}}),
        );
        assert!(compile_test_node(fields.clone(), None)
            .unwrap_err()
            .contains("Reality"));
        fields.insert(
            "tls".into(),
            serde_json::json!({"enabled":true,"reality":{"enabled":true,"public_key":"key"},"utls":{"enabled":true,"fingerprint":"chrome"}}),
        );
        assert!(compile_test_node(fields, None).is_ok());
    }

    #[test]
    fn test_auto_urltest_follows_root_selector() {
        let node = SingBoxOutbound {
            outbound_type: "vless".into(),
            tag: "sample-node".into(),
            fields: HashMap::from([
                ("server".into(), serde_json::json!("proxy.example")),
                ("server_port".into(), serde_json::json!(443)),
                ("uuid".into(), serde_json::json!("user-id")),
            ]),
        };
        let generated = generate_singbox_config(
            7892,
            vec![node],
            "system",
            "1.1.1.1",
            "127.0.0.1",
            &TunSettings::default(),
            Some("sample-node"),
            &[],
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .unwrap();
        let config: Value = serde_json::from_str(&generated).unwrap();
        assert_eq!(config["outbounds"][0]["tag"], "proxy");
        assert_eq!(config["outbounds"][0]["default"], "sample-node");
        assert!(config["outbounds"][0]["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tag| tag == "auto"));
        assert_eq!(config["outbounds"][1]["type"], "urltest");
        assert_eq!(config["outbounds"][1]["tag"], "auto");
        assert_eq!(
            config["outbounds"][1]["outbounds"],
            serde_json::json!(["sample-node"])
        );
    }

    #[test]
    fn test_udp_tls_keeps_protocol_alpn_defaults() {
        for outbound_type in ["hysteria2", "tuic"] {
            let mut fields = HashMap::from([
                ("server".into(), serde_json::json!("proxy.example")),
                ("server_port".into(), serde_json::json!(443)),
                ("tls".into(), serde_json::json!({"enabled": true})),
            ]);
            fields.insert("password".into(), serde_json::json!("secret"));
            if outbound_type == "tuic" {
                fields.insert(
                    "uuid".into(),
                    serde_json::json!("00000000-0000-0000-0000-000000000000"),
                );
            }
            let json = generate_singbox_config(
                7890,
                vec![SingBoxOutbound {
                    outbound_type: outbound_type.into(),
                    tag: "udp-node".into(),
                    fields,
                }],
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
            let config: Value = serde_json::from_str(&json).unwrap();
            let node = config["outbounds"]
                .as_array()
                .unwrap()
                .iter()
                .find(|outbound| outbound["tag"] == "udp-node")
                .unwrap();
            assert!(node["tls"].get("alpn").is_none());
        }
    }

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
        let system_dns_servers = config_local["dns"]["servers"].as_array().unwrap();
        for rule in config_local["dns"]["rules"].as_array().unwrap() {
            if let Some(target) = rule.get("server").and_then(Value::as_str) {
                assert!(
                    system_dns_servers
                        .iter()
                        .any(|server| server["tag"] == target),
                    "DNS rule references an undefined server: {target}"
                );
            }
        }

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
    fn test_generate_singbox_config_preserves_explicit_alpn() {
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
        // For WebSocket transport, h2 is stripped to prevent server EOF due to HTTP/2 vs HTTP/1.1 mismatch
        assert_eq!(alpn, &vec![serde_json::json!("http/1.1")]);
    }

    #[test]
    fn test_generate_singbox_config_preserves_explicit_alpn_on_tcp() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("server".to_string(), serde_json::json!("proxy.example"));
        fields.insert("server_port".to_string(), serde_json::json!(443));
        fields.insert(
            "uuid".to_string(),
            serde_json::json!("88dacb71-7530-475b-9ed6-a431caef6b3f"),
        );
        fields.insert(
            "tls".to_string(),
            serde_json::json!({
                "enabled": true,
                "server_name": "aka.ms",
                "alpn": ["h2", "http/1.1"]
            }),
        );

        let outbounds = vec![super::super::adapters::SingBoxOutbound {
            outbound_type: "vless".to_string(),
            tag: "TCP-Node".to_string(),
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
        let node_outbound = outbounds_arr
            .iter()
            .find(|o| o["tag"].as_str() == Some("TCP-Node"))
            .expect("Should find our node outbound");

        let alpn = node_outbound["tls"]["alpn"].as_array().unwrap();
        assert_eq!(alpn, &vec![serde_json::json!("h2"), serde_json::json!("http/1.1")]);
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
        // For HTTPUpgrade transport, h2 is stripped to prevent malformed HTTP response binary framing errors
        assert_eq!(node["tls"]["alpn"], serde_json::json!(["http/1.1"]));
        assert!(node["tls"].get("insecure").is_none());
        assert_eq!(node["transport"]["host"], "cdn.example");
        assert_eq!(node["transport"]["headers"]["X-Custom"], "value");
        assert!(node["transport"]["headers"].get("Host").is_none());
    }

    #[test]
    fn test_auto_detect_spoofed_sni_sets_insecure() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("server".to_string(), serde_json::json!("azure.ezgateway.net"));
        fields.insert("server_port".to_string(), serde_json::json!(443));
        fields.insert(
            "uuid".to_string(),
            serde_json::json!("7efbd5f5-f9f9-425e-b0eb-d46a8a6f3bd9"),
        );
        fields.insert(
            "tls".to_string(),
            serde_json::json!({
                "enabled": true,
                "server_name": "cloud.microsoft",
                "alpn": ["h2", "http/1.1"]
            }),
        );
        fields.insert(
            "transport".to_string(),
            serde_json::json!({
                "type": "ws",
                "path": "/azure",
                "headers": { "Host": "azure.ezgateway.net" }
            }),
        );

        let outbounds = vec![super::super::adapters::SingBoxOutbound {
            outbound_type: "vless".to_string(),
            tag: "One Piece SG-Prabathya".to_string(),
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
            .find(|o| o["tag"] == "One Piece SG-Prabathya")
            .unwrap();
        assert_eq!(node["tls"]["server_name"], "cloud.microsoft");
        assert_eq!(node["tls"]["insecure"], serde_json::json!(true));
        assert_eq!(node["tls"]["alpn"], serde_json::json!(["http/1.1"]));
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
        assert!(node["tls"].get("alpn").is_none());
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
        assert!(parsed_bare_ipv6.get("detour").is_none());

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
        assert!(parsed_local.get("detour").is_none());
        assert!(parsed_local.get("server").is_none());

        let parsed_empty = parse_dns_server("local-dns", "", None);
        assert_eq!(parsed_empty["type"], "local");

        let parsed_doh = parse_dns_server("doh", "https://1.1.1.1/dns-query", Some("proxy"));
        assert_eq!(parsed_doh["type"], "https");
        assert_eq!(parsed_doh["server"], "1.1.1.1");
        assert_eq!(parsed_doh["detour"], "proxy");
        assert!(parsed_doh.get("path").is_none());
        assert!(parsed_doh.get("server_port").is_none());

        let parsed_doh_custom =
            parse_dns_server("doh2", "https://dns.example.com:8443/custom", Some("proxy"));
        assert_eq!(parsed_doh_custom["server"], "dns.example.com");
        assert_eq!(parsed_doh_custom["server_port"], 8443);
        assert_eq!(parsed_doh_custom["path"], "/custom");

        let parsed_dot = parse_dns_server("dot", "tls://1.1.1.1:853", Some("proxy"));
        assert_eq!(parsed_dot["type"], "tls");
        assert_eq!(parsed_dot["server"], "1.1.1.1");
        assert_eq!(parsed_dot["server_port"], 853);

        let parsed_udp_prefix = parse_dns_server("udp", "udp://1.1.1.1:53", Some("direct"));
        assert_eq!(parsed_udp_prefix["type"], "udp");
        assert_eq!(parsed_udp_prefix["server"], "1.1.1.1");
        assert_eq!(parsed_udp_prefix["server_port"], 53);
    }

    #[test]
    fn test_routing_mode_config_generation_overrides() {
        let outbounds = vec![SingBoxOutbound {
            outbound_type: "vless".into(),
            tag: "sample-node".into(),
            fields: HashMap::new(),
        }];

        // Case 1: SniZeroData mode
        let mut tun_zero_data = TunSettings::default();
        tun_zero_data.routing_mode = crate::state::RoutingMode::SniZeroData;
        tun_zero_data.final_outbound = "direct".to_string(); // User attempted direct, should be overridden

        let json_zero_data = generate_singbox_config(
            7890,
            outbounds.clone(),
            "tun",
            "1.1.1.1",
            "127.0.0.1",
            &tun_zero_data,
            None,
            &[],
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .expect("SniZeroData should generate valid config with fail-closed DNS and route engine");
        let config_zd: Value = serde_json::from_str(&json_zero_data).unwrap();

        // 1. Route final must be "proxy" (overriding "direct")
        assert_eq!(config_zd["route"]["final"], "proxy");
        // 2. DNS final must be "remote-dns"
        assert_eq!(config_zd["dns"]["final"], "remote-dns");
        // 3. remote-dns server has detour "proxy"
        let remote_dns = config_zd["dns"]["servers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["tag"] == "remote-dns")
            .expect("remote-dns server must exist");
        assert_eq!(remote_dns["detour"], "proxy");
        assert_eq!(config_zd["dns"]["independent_cache"], true);

        // Case 2: Direct routing mode
        let mut tun_direct = TunSettings::default();
        tun_direct.routing_mode = crate::state::RoutingMode::Direct;

        let json_dir = generate_singbox_config(
            7890,
            outbounds,
            "tun",
            "1.1.1.1",
            "127.0.0.1",
            &tun_direct,
            None,
            &[],
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .unwrap();
        let config_dir: Value = serde_json::from_str(&json_dir).unwrap();

        // 1. Route final must be "direct"
        assert_eq!(config_dir["route"]["final"], "direct");
        // 2. DNS final must be "local-dns"
        assert_eq!(config_dir["dns"]["final"], "local-dns");

        // Case 3: System proxy mode uses isolated local-dns to prevent DoT deadlocks
        let json_sys = generate_singbox_config(
            7890,
            vec![SingBoxOutbound {
                outbound_type: "vless".to_string(),
                tag: "proxy-node".to_string(),
                fields: HashMap::from([
                    ("server".to_string(), serde_json::json!("1.2.3.4")),
                    ("server_port".to_string(), serde_json::json!(443)),
                ]),
            }],
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
        let config_sys: Value = serde_json::from_str(&json_sys).unwrap();
        let sys_dns_servers = config_sys["dns"]["servers"].as_array().unwrap();
        assert_eq!(sys_dns_servers.len(), 1);
        assert_eq!(sys_dns_servers[0]["tag"], "local-dns");
        assert_eq!(config_sys["dns"]["final"], "local-dns");
    }

    #[test]
    fn test_phase2_singbox_check_acceptance_criteria() {
        use crate::config::rules::{RuleMatchType, RuleOutbound, RoutingRule};

        let mut node_fields = HashMap::new();
        node_fields.insert("server".to_string(), serde_json::json!("msteams.opik.net"));
        node_fields.insert("server_port".to_string(), serde_json::json!(443));
        node_fields.insert("uuid".to_string(), serde_json::json!("814dfb49-8e2c-4760-92cb-d9f8564714a4"));
        node_fields.insert("flow".to_string(), serde_json::json!(""));
        node_fields.insert(
            "tls".to_string(),
            serde_json::json!({
                "enabled": true,
                "server_name": "reroute443.microsoft.com",
                "utls": {
                    "enabled": true,
                    "fingerprint": "chrome"
                },
                "reality": {
                    "enabled": true,
                    "public_key": "viW8W5uDJ1xz1e-Mtz9mhMxNH-FRuOE7KQRCpaCkFFM",
                    "short_id": "813ac8a5d7f806"
                }
            }),
        );
        node_fields.insert(
            "transport".to_string(),
            serde_json::json!({
                "type": "grpc",
                "service_name": "teams",
                "idle_timeout": "15s",
                "ping_timeout": "15s",
                "permit_without_stream": true
            }),
        );

        let outbounds = vec![
            SingBoxOutbound {
                outbound_type: "vless".to_string(),
                tag: "Zoom-test".to_string(),
                fields: node_fields.clone(),
            },
        ];

        let user_rules = vec![
            RoutingRule::new(RuleMatchType::Domain, "ads.example.com", RuleOutbound::Block),
            RoutingRule::new(RuleMatchType::DomainSuffix, "google.com", RuleOutbound::Proxy),
        ];

        let mut tun_settings = TunSettings::default();
        tun_settings.routing_mode = crate::state::RoutingMode::Rule;
        tun_settings.block_quic = true;
        tun_settings.primary_dns = "tls://1.1.1.1".to_string();

        let config_str = generate_singbox_config(
            7890,
            outbounds,
            "tun",
            "1.1.1.1",
            "127.0.0.1",
            &tun_settings,
            None,
            &user_rules,
            &[],
            std::path::Path::new(""),
            &[],
            &[],
        )
        .expect("Config generation must succeed");

        // 1. Acceptance Criterion: No occurrences of "outbound": "block" or "outbound": "dns"
        assert!(!config_str.contains("\"outbound\": \"block\""));
        assert!(!config_str.contains("\"outbound\":\"block\""));
        assert!(!config_str.contains("\"outbound\": \"dns\""));
        assert!(!config_str.contains("\"outbound\":\"dns\""));

        let config: Value = serde_json::from_str(&config_str).unwrap();

        // 2. Acceptance Criterion: DNS server remote-dns points to detour proxy
        let dns_servers = config["dns"]["servers"].as_array().expect("dns.servers array");
        let remote_dns = dns_servers
            .iter()
            .find(|s| s["tag"] == "remote-dns")
            .expect("remote-dns server must exist");
        assert_eq!(remote_dns["detour"].as_str().unwrap(), "proxy");
        assert_eq!(remote_dns["type"].as_str().unwrap(), "tls");
        assert_eq!(remote_dns["server"].as_str().unwrap(), "1.1.1.1");
        assert_eq!(remote_dns["server_port"].as_u64().unwrap(), 853);

        // Bootstrap DNS rule for proxy server domain
        let dns_rules = config["dns"]["rules"].as_array().expect("dns.rules array");
        let bootstrap_rule = dns_rules
            .iter()
            .find(|r| r.get("server").and_then(Value::as_str) == Some("local-dns"))
            .expect("bootstrap rule to local-dns must exist");
        let bootstrap_domains = bootstrap_rule["domain"].as_array().unwrap();
        assert!(bootstrap_domains.iter().any(|d| d.as_str() == Some("msteams.opik.net")));

        // DNS final must be remote-dns in Rule mode
        assert_eq!(config["dns"]["final"], "remote-dns");
        assert_eq!(config["dns"]["strategy"], "prefer_ipv4");
        assert_eq!(config["dns"]["independent_cache"], true);

        // 3. Acceptance Criterion: Port 53 packets captured by action: hijack-dns
        let route_rules = config["route"]["rules"].as_array().expect("route.rules array");
        let hijack_rule = route_rules
            .iter()
            .find(|r| r.get("action").and_then(Value::as_str) == Some("hijack-dns"))
            .expect("hijack-dns rule must exist");
        assert_eq!(hijack_rule["protocol"].as_str().unwrap(), "dns");

        // 4. Acceptance Criterion: UDP 443 is rejected when block_quic is enabled
        let quic_proto_rule = route_rules
            .iter()
            .find(|r| r.get("protocol").and_then(Value::as_str) == Some("quic"))
            .expect("quic protocol rule must exist");
        assert_eq!(quic_proto_rule["action"].as_str().unwrap(), "reject");

        let quic_udp_rule = route_rules
            .iter()
            .find(|r| {
                r.get("network").and_then(Value::as_str) == Some("udp")
                    && r.get("port").and_then(Value::as_u64) == Some(443)
            })
            .expect("udp 443 reject rule must exist");
        assert_eq!(quic_udp_rule["action"].as_str().unwrap(), "reject");

        // Custom block rule outputs action: reject (never outbound: block)
        let custom_block = route_rules
            .iter()
            .find(|r| {
                r.get("domain")
                    .and_then(Value::as_array)
                    .map(|arr| arr.iter().any(|d| d.as_str() == Some("ads.example.com")))
                    .unwrap_or(false)
            })
            .expect("custom ads block rule must exist");
        assert_eq!(custom_block["action"].as_str().unwrap(), "reject");
        assert!(custom_block.get("outbound").is_none());

        // 5. Acceptance Criterion: Verify with sing-box check CLI
        let binary_candidates = [
            std::path::PathBuf::from("binaries/sing-box-x86_64-pc-windows-msvc.exe"),
            std::path::PathBuf::from("src-tauri/binaries/sing-box-x86_64-pc-windows-msvc.exe"),
            std::path::PathBuf::from("../src-tauri/binaries/sing-box-x86_64-pc-windows-msvc.exe"),
        ];

        if let Some(bin) = binary_candidates.iter().find(|p| p.exists()) {
            let temp_dir = std::env::temp_dir();

            // Check 5a: Rule mode config check
            let test_config_path = temp_dir.join(format!("test_phase2_singbox_{}.json", std::process::id()));
            std::fs::write(&test_config_path, &config_str).expect("write temp config");

            let output = std::process::Command::new(bin)
                .arg("check")
                .arg("-c")
                .arg(&test_config_path)
                .output()
                .expect("execute sing-box check");

            let _ = std::fs::remove_file(&test_config_path);

            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success(),
                "sing-box check failed for Rule mode: stdout={}, stderr={}",
                stdout,
                stderr
            );

            // Check 5b: SniZeroData mode config check
            let mut zero_data_settings = tun_settings.clone();
            zero_data_settings.routing_mode = crate::state::RoutingMode::SniZeroData;
            let zero_data_config_str = generate_singbox_config(
                7890,
                vec![
                    SingBoxOutbound {
                        outbound_type: "vless".to_string(),
                        tag: "Zoom-test".to_string(),
                        fields: node_fields,
                    },
                ],
                "tun",
                "1.1.1.1",
                "127.0.0.1",
                &zero_data_settings,
                None,
                &user_rules,
                &[],
                std::path::Path::new(""),
                &[],
                &[],
            )
            .expect("SniZeroData config generation must succeed");

            let zd_config_path = temp_dir.join(format!("test_phase2_zd_singbox_{}.json", std::process::id()));
            std::fs::write(&zd_config_path, &zero_data_config_str).expect("write temp zero-data config");

            let zd_output = std::process::Command::new(bin)
                .arg("check")
                .arg("-c")
                .arg(&zd_config_path)
                .output()
                .expect("execute sing-box check on zero-data");

            let _ = std::fs::remove_file(&zd_config_path);

            let zd_stderr = String::from_utf8_lossy(&zd_output.stderr);
            let zd_stdout = String::from_utf8_lossy(&zd_output.stdout);
            assert!(
                zd_output.status.success(),
                "sing-box check failed for SniZeroData mode: stdout={}, stderr={}",
                zd_stdout,
                zd_stderr
            );
        }
    }
}
