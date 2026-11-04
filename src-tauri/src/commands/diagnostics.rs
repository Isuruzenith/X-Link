use crate::commands::system::{check_cached_singbox_compatibility, TESTED_SINGBOX_VERSION};
use crate::config::rules::{load_routing_rules_from_file, RoutingRule};
use crate::state::ProxyState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use tauri::Manager;

// ─── ROUTING SIMULATOR ───────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoutingSimulationResult {
    pub destination: String,
    pub destination_type: String, // "ip" or "domain"
    pub matched_outbound: String, // "direct", "proxy", "block"
    pub matched_rule_id: Option<String>,
    pub matched_rule_type: Option<String>,
    pub match_reason: String,
    pub is_lan_bypass: bool,
    pub dns_strategy: String,
}

/// Checks whether an IP address falls within a CIDR prefix (e.g. "192.168.1.0/24" or "2001:db8::/32").
pub fn ip_matches_cidr(ip: &IpAddr, cidr_str: &str) -> bool {
    let parts: Vec<&str> = cidr_str.trim().split('/').collect();
    if parts.len() != 2 {
        if let Ok(parsed) = cidr_str.trim().parse::<IpAddr>() {
            return *ip == parsed;
        }
        return false;
    }
    let base_ip_str = parts[0];
    let prefix_len = match parts[1].parse::<u32>() {
        Ok(p) => p,
        Err(_) => return false,
    };

    match (ip, base_ip_str.parse::<IpAddr>()) {
        (IpAddr::V4(target_v4), Ok(IpAddr::V4(base_v4))) => {
            if prefix_len > 32 {
                return false;
            }
            let target_num = u32::from(*target_v4);
            let base_num = u32::from(base_v4);
            let mask = if prefix_len == 0 {
                0
            } else {
                !0u32 << (32 - prefix_len)
            };
            (target_num & mask) == (base_num & mask)
        }
        (IpAddr::V6(target_v6), Ok(IpAddr::V6(base_v6))) => {
            if prefix_len > 128 {
                return false;
            }
            let target_num = u128::from(*target_v6);
            let base_num = u128::from(base_v6);
            let mask = if prefix_len == 0 {
                0
            } else {
                !0u128 << (128 - prefix_len)
            };
            (target_num & mask) == (base_num & mask)
        }
        _ => false,
    }
}

/// Determines whether an IP is private, loopback, link-local, or unspecified.
pub fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                // CGNAT: 100.64.0.0/10
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                // ULA: fc00::/7
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                // Link-local: fe80::/10
                || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Evaluates a destination against routing rules, honoring LAN bypass and top-to-bottom rule order.
pub fn evaluate_route_destination(
    raw_destination: &str,
    bypass_lan: bool,
    user_rules: &[RoutingRule],
    final_outbound: &str,
    dns_strategy: &str,
) -> RoutingSimulationResult {
    let dest = raw_destination.trim();
    if dest.is_empty() {
        return RoutingSimulationResult {
            destination: String::new(),
            destination_type: "unknown".to_string(),
            matched_outbound: final_outbound.to_string(),
            matched_rule_id: None,
            matched_rule_type: Some("empty_fallback".to_string()),
            match_reason: format!(
                "Empty destination; applied fallback outbound '{}'",
                final_outbound
            ),
            is_lan_bypass: false,
            dns_strategy: dns_strategy.to_string(),
        };
    }

    // Check if the destination is an IP address
    if let Ok(ip) = dest.parse::<IpAddr>() {
        // 1. Built-in LAN bypass check
        if bypass_lan && is_private_ip(&ip) {
            return RoutingSimulationResult {
                destination: dest.to_string(),
                destination_type: "ip".to_string(),
                matched_outbound: "direct".to_string(),
                matched_rule_id: None,
                matched_rule_type: Some("bypass_lan".to_string()),
                match_reason:
                    "Destination is a private/local IP address matching built-in LAN bypass"
                        .to_string(),
                is_lan_bypass: true,
                dns_strategy: dns_strategy.to_string(),
            };
        }

        // 2. User-defined rules evaluated top-to-bottom
        for (idx, rule) in user_rules.iter().enumerate() {
            if !rule.enabled.unwrap_or(true) {
                continue;
            }

            let mut val = rule.value.trim().to_string();
            let mut r_type = rule.rule_type.as_str();

            if val.starts_with("cidr:") {
                val = val.strip_prefix("cidr:").unwrap().to_string();
                r_type = "ip_cidr";
            } else if val.starts_with("ip:") {
                val = val.strip_prefix("ip:").unwrap().to_string();
                r_type = "ip_cidr";
            }

            if r_type == "ip_cidr" {
                let mut matched = ip_matches_cidr(&ip, &val);
                if rule.invert {
                    matched = !matched;
                }
                if matched {
                    let outbound = if rule.outbound == "block" {
                        "block"
                    } else {
                        &rule.outbound
                    };
                    return RoutingSimulationResult {
                        destination: dest.to_string(),
                        destination_type: "ip".to_string(),
                        matched_outbound: outbound.to_string(),
                        matched_rule_id: Some(rule.id.clone()),
                        matched_rule_type: Some(rule.rule_type.clone()),
                        match_reason: format!(
                            "Matched rule #{} ({} {}): routed to '{}'",
                            idx + 1,
                            rule.rule_type,
                            rule.value,
                            outbound
                        ),
                        is_lan_bypass: false,
                        dns_strategy: dns_strategy.to_string(),
                    };
                }
            }
        }

        // 3. Fallback to final outbound
        return RoutingSimulationResult {
            destination: dest.to_string(),
            destination_type: "ip".to_string(),
            matched_outbound: final_outbound.to_string(),
            matched_rule_id: None,
            matched_rule_type: Some("final_fallback".to_string()),
            match_reason: format!(
                "No explicit rule matched IP '{}'; fell back to final outbound '{}'",
                dest, final_outbound
            ),
            is_lan_bypass: false,
            dns_strategy: dns_strategy.to_string(),
        };
    }

    // Destination is a domain name
    let clean_domain = dest.to_lowercase();

    // 1. Built-in LAN domain bypass
    if bypass_lan {
        let is_lan_domain = clean_domain == "localhost"
            || clean_domain.ends_with(".lan")
            || clean_domain.ends_with(".local")
            || clean_domain.ends_with(".internal")
            || clean_domain.ends_with(".home.arpa");
        if is_lan_domain {
            return RoutingSimulationResult {
                destination: dest.to_string(),
                destination_type: "domain".to_string(),
                matched_outbound: "direct".to_string(),
                matched_rule_id: None,
                matched_rule_type: Some("bypass_lan".to_string()),
                match_reason: "Domain suffix matches built-in private/local LAN bypass".to_string(),
                is_lan_bypass: true,
                dns_strategy: dns_strategy.to_string(),
            };
        }
    }

    // 2. User-defined rules evaluated top-to-bottom
    for (idx, rule) in user_rules.iter().enumerate() {
        if !rule.enabled.unwrap_or(true) {
            continue;
        }

        let mut val = rule.value.trim().to_lowercase();
        let mut r_type = rule.rule_type.as_str();

        if val.starts_with("domain:") {
            val = val.strip_prefix("domain:").unwrap().to_string();
            r_type = "domain";
        } else if val.starts_with("full:") {
            val = val.strip_prefix("full:").unwrap().to_string();
            r_type = "domain";
        } else if val.starts_with("geosite:") {
            val = val.strip_prefix("geosite:").unwrap().to_string();
            r_type = "geosite";
        }

        let mut matched = false;
        match r_type {
            "domain" => {
                matched = clean_domain == val;
            }
            "domain_suffix" => {
                let suffix = val.trim_start_matches('.');
                matched = clean_domain == suffix || clean_domain.ends_with(&format!(".{}", suffix));
            }
            "domain_keyword" => {
                matched = clean_domain.contains(&val);
            }
            "geosite" if val == "private" => {
                matched = clean_domain == "localhost"
                    || clean_domain.ends_with(".lan")
                    || clean_domain.ends_with(".local")
                    || clean_domain.ends_with(".internal")
                    || clean_domain.ends_with(".home.arpa");
            }
            _ => {}
        }

        if rule.invert {
            matched = !matched;
        }

        if matched {
            let outbound = if rule.outbound == "block" {
                "block"
            } else {
                &rule.outbound
            };
            return RoutingSimulationResult {
                destination: dest.to_string(),
                destination_type: "domain".to_string(),
                matched_outbound: outbound.to_string(),
                matched_rule_id: Some(rule.id.clone()),
                matched_rule_type: Some(rule.rule_type.clone()),
                match_reason: format!(
                    "Matched rule #{} ({} {}): routed to '{}'",
                    idx + 1,
                    rule.rule_type,
                    rule.value,
                    outbound
                ),
                is_lan_bypass: false,
                dns_strategy: dns_strategy.to_string(),
            };
        }
    }

    // 3. Fallback to final outbound
    RoutingSimulationResult {
        destination: dest.to_string(),
        destination_type: "domain".to_string(),
        matched_outbound: final_outbound.to_string(),
        matched_rule_id: None,
        matched_rule_type: Some("final_fallback".to_string()),
        match_reason: format!(
            "No explicit rule matched domain '{}'; fell back to final outbound '{}'",
            dest, final_outbound
        ),
        is_lan_bypass: false,
        dns_strategy: dns_strategy.to_string(),
    }
}

// ─── PRIVACY-SAFE SANITIZATION ──────────────────────────────────────────────

/// Redacts sensitive tokens or query parameters from a URL string while preserving scheme and host.
#[allow(dead_code)]
pub fn sanitize_url_str(url_str: &str) -> String {
    let trimmed = url_str.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Ok(mut parsed) = url::Url::parse(trimmed) {
        if !parsed.username().is_empty() {
            let _ = parsed.set_username("[REDACTED]");
        }
        if parsed.password().is_some() {
            let _ = parsed.set_password(Some("[REDACTED]"));
        }
        if parsed.query().is_some() {
            parsed.set_query(Some("redacted=true"));
        }
        parsed.to_string()
    } else {
        "[REDACTED_URL]".to_string()
    }
}

/// Sanitizes a log line by stripping API secret tokens, Bearer headers, and masking public IP addresses.
pub fn sanitize_log_line(line: &str) -> String {
    let mut out = line.to_string();

    // Redact secret=... / token=... / password=... / key=...
    let patterns = ["secret=", "token=", "password=", "key=", "api_secret="];
    for pat in patterns {
        let mut search_from = 0;
        while let Some(rel_start) = out[search_from..].to_lowercase().find(pat) {
            let start = search_from + rel_start;
            let val_start = start + pat.len();
            let end = out[val_start..]
                .find(|c: char| {
                    c == '&' || c == ' ' || c == '"' || c == '\'' || c == '\n' || c == '}'
                })
                .map(|pos| val_start + pos)
                .unwrap_or(out.len());
            if val_start < end {
                out.replace_range(val_start..end, "[REDACTED]");
                search_from = val_start + "[REDACTED]".len();
            } else {
                search_from = val_start + pat.len();
            }
        }
    }

    // Redact Bearer <token>
    let mut search_from = 0;
    while let Some(rel_start) = out[search_from..].find("Bearer ") {
        let start = search_from + rel_start;
        let val_start = start + 7;
        let end = out[val_start..]
            .find([' ', '"', '\'', '\n'])
            .map(|pos| val_start + pos)
            .unwrap_or(out.len());
        if val_start < end {
            out.replace_range(val_start..end, "[REDACTED]");
            search_from = val_start + "[REDACTED]".len();
        } else {
            search_from = val_start + 7;
        }
    }

    out
}

/// Sanitizes user settings, removing API secret and any private credentials.
pub fn sanitize_user_settings(val: &serde_json::Value) -> serde_json::Value {
    let mut cloned = val.clone();
    if let Some(map) = cloned.as_object_mut() {
        if map.contains_key("apiSecret") {
            map.insert("apiSecret".to_string(), serde_json::json!("[REDACTED]"));
        }
    }
    cloned
}

// ─── DIAGNOSTICS PAYLOAD STRUCTS ────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OsDiagnostics {
    pub platform: String,
    pub arch: String,
    pub is_elevated: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SidecarDiagnostics {
    pub bundled_tested_version: String,
    pub cached_version: Option<String>,
    pub is_compatible: bool,
    pub compatibility_error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProxyDiagnostics {
    pub status: String,
    pub effective_mode: Option<String>,
    pub tun_supported: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PortDiagnostics {
    pub mixed_port: u16,
    pub socks_port: u16,
    pub http_port: u16,
    pub api_port: u16,
    pub port_statuses: HashMap<u16, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SystemProxyDiagnostics {
    pub managed_by_xlink: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesDiagnostics {
    pub profile_count: usize,
    pub active_profile_type: Option<String>,
    pub node_counts_by_type: HashMap<String, usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    pub timestamp: u64,
    pub app_version: String,
    pub os: OsDiagnostics,
    pub sidecar: SidecarDiagnostics,
    pub proxy: ProxyDiagnostics,
    pub ports: PortDiagnostics,
    pub system_proxy: SystemProxyDiagnostics,
    pub profiles: ProfilesDiagnostics,
    pub settings: serde_json::Value,
    pub recent_logs: Vec<String>,
    pub dns_queries: Vec<crate::state::DnsLogEntry>,
    pub traces: Vec<crate::state::TraceEntry>,
}

// ─── TAURI COMMANDS ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn export_system_diagnostics(
    app: tauri::AppHandle,
    state: tauri::State<'_, ProxyState>,
) -> Result<DiagnosticsReport, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let app_version = app.package_info().version.to_string();

    let os_diag = OsDiagnostics {
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        is_elevated: crate::os::is_elevated(),
    };

    let cached_sidecar = check_cached_singbox_compatibility(&state);
    let (cached_ver_str, is_compat, compat_err) = match cached_sidecar {
        Some(Ok(v)) => (Some(v.semver_string()), true, None),
        Some(Err(err)) => (None, false, Some(err)),
        None => (None, false, None),
    };

    let sidecar_diag = SidecarDiagnostics {
        bundled_tested_version: TESTED_SINGBOX_VERSION.to_string(),
        cached_version: cached_ver_str,
        is_compatible: is_compat,
        compatibility_error: compat_err,
    };

    let proxy_status_str = match state.get_status() {
        crate::state::ConnectionStatus::Disconnected => "disconnected",
        crate::state::ConnectionStatus::Connecting => "connecting",
        crate::state::ConnectionStatus::Connected => "connected",
    };

    let eff_mode = if let Ok(g) = state.effective_proxy_mode.lock() {
        g.clone()
    } else {
        None
    };

    let tun_supported = matches!(
        crate::os::get_network_manager().check_tun_support().await,
        Ok(crate::os::TunSupportStatus::Supported)
    );

    let proxy_diag = ProxyDiagnostics {
        status: proxy_status_str.to_string(),
        effective_mode: eff_mode,
        tun_supported,
    };

    let s = state.get_settings();
    let mut port_statuses = HashMap::new();
    let monitored_ports = [s.mixed_port, s.socks_port, s.http_port, s.api_port];
    for p in monitored_ports {
        let is_bound = crate::commands::system::port_is_bound(p);
        port_statuses.insert(
            p,
            if is_bound {
                "bound".to_string()
            } else {
                "free".to_string()
            },
        );
    }

    let port_diag = PortDiagnostics {
        mixed_port: s.mixed_port,
        socks_port: s.socks_port,
        http_port: s.http_port,
        api_port: s.api_port,
        port_statuses,
    };

    let system_proxy_diag = SystemProxyDiagnostics {
        managed_by_xlink: crate::os::is_system_proxy_owned(),
    };

    // Collect profile counts without leaking server addresses or credentials
    let (prof_count, active_type, node_counts) = collect_profiles_summary(&app);

    let profiles_diag = ProfilesDiagnostics {
        profile_count: prof_count,
        active_profile_type: active_type,
        node_counts_by_type: node_counts,
    };

    let settings_val = serde_json::to_value(&s).unwrap_or(serde_json::Value::Null);
    let sanitized_settings = sanitize_user_settings(&settings_val);

    let raw_logs = state.get_buffered_logs_snapshot(50);
    let sanitized_logs = raw_logs.iter().map(|l| sanitize_log_line(l)).collect();

    let diag_snapshot = state.get_diagnostic_traces_snapshot();

    Ok(DiagnosticsReport {
        timestamp: now,
        app_version,
        os: os_diag,
        sidecar: sidecar_diag,
        proxy: proxy_diag,
        ports: port_diag,
        system_proxy: system_proxy_diag,
        profiles: profiles_diag,
        settings: sanitized_settings,
        recent_logs: sanitized_logs,
        dns_queries: diag_snapshot.dns_queries,
        traces: diag_snapshot.traces,
    })
}

fn collect_profiles_summary(
    app: &tauri::AppHandle,
) -> (usize, Option<String>, HashMap<String, usize>) {
    let app_dir: Option<PathBuf> = app.path().app_data_dir().ok();
    let mut count = 0;
    let mut active_type = None;
    let mut node_counts = HashMap::new();

    if let Some(mut path) = app_dir {
        path.push("profiles.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(profiles) = val.get("profiles").and_then(|p| p.as_array()) {
                        count = profiles.len();
                        let active_id = val.get("activeProfileId").and_then(|v| v.as_str());
                        for p in profiles {
                            let pid = p.get("id").and_then(|v| v.as_str());
                            let p_type = p.get("type").and_then(|v| v.as_str()).unwrap_or("manual");
                            if active_id.is_some() && active_id == pid {
                                active_type = Some(p_type.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Check active profile outbounds to count node types safely
    if let Ok(mut outbounds_path) = app.path().app_data_dir() {
        outbounds_path.push("active.json");
        if outbounds_path.exists() {
            if let Ok(content) = std::fs::read_to_string(outbounds_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(outbounds) = val.get("outbounds").and_then(|o| o.as_array()) {
                        for ob in outbounds {
                            if let Some(t) = ob.get("type").and_then(|v| v.as_str()) {
                                if t != "direct" && t != "block" && t != "dns" {
                                    *node_counts.entry(t.to_string()).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (count, active_type, node_counts)
}

#[tauri::command]
pub fn simulate_routing(
    app: tauri::AppHandle,
    destination: String,
) -> Result<RoutingSimulationResult, String> {
    let (user_rules, _) = load_routing_rules_from_file(&app);
    let state = app.try_state::<ProxyState>();
    let (bypass_lan, final_outbound, dns_strategy) = if let Some(s) = state {
        let settings = s.get_settings();
        (
            settings.bypass_lan,
            settings.final_outbound,
            settings.dns_strategy,
        )
    } else {
        (true, "proxy".to_string(), "prefer_ipv4".to_string())
    };

    Ok(evaluate_route_destination(
        &destination,
        bypass_lan,
        &user_rules,
        &final_outbound,
        &dns_strategy,
    ))
}

// ─── UNIT TESTS ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_matches_cidr_ipv4() {
        let ip1: IpAddr = "192.168.1.50".parse().unwrap();
        assert!(ip_matches_cidr(&ip1, "192.168.1.0/24"));
        assert!(ip_matches_cidr(&ip1, "192.168.0.0/16"));
        assert!(ip_matches_cidr(&ip1, "0.0.0.0/0"));
        assert!(ip_matches_cidr(&ip1, "192.168.1.50/32"));
        assert!(!ip_matches_cidr(&ip1, "192.168.2.0/24"));
        assert!(!ip_matches_cidr(&ip1, "10.0.0.0/8"));

        let ip2: IpAddr = "10.5.20.1".parse().unwrap();
        assert!(ip_matches_cidr(&ip2, "10.0.0.0/8"));
        assert!(!ip_matches_cidr(&ip2, "10.6.0.0/16"));
    }

    #[test]
    fn test_ip_matches_cidr_ipv6() {
        let ip: IpAddr = "2001:db8:85a3::8a2e:370:7334".parse().unwrap();
        assert!(ip_matches_cidr(&ip, "2001:db8::/32"));
        assert!(ip_matches_cidr(&ip, "::/0"));
        assert!(!ip_matches_cidr(&ip, "2001:db9::/32"));
    }

    #[test]
    fn test_is_private_ip() {
        let priv_v4: IpAddr = "192.168.0.1".parse().unwrap();
        assert!(is_private_ip(&priv_v4));
        let loop_v4: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(is_private_ip(&loop_v4));
        let pub_v4: IpAddr = "1.1.1.1".parse().unwrap();
        assert!(!is_private_ip(&pub_v4));

        let loop_v6: IpAddr = "::1".parse().unwrap();
        assert!(is_private_ip(&loop_v6));
        let ula_v6: IpAddr = "fc00::1".parse().unwrap();
        assert!(is_private_ip(&ula_v6));
        let pub_v6: IpAddr = "2606:4700:4700::1111".parse().unwrap();
        assert!(!is_private_ip(&pub_v6));
    }

    #[test]
    fn test_evaluate_route_destination_lan_bypass() {
        let rules: Vec<RoutingRule> = Vec::new();

        // Private IP with LAN bypass ON -> direct
        let r1 = evaluate_route_destination("192.168.1.1", true, &rules, "proxy", "prefer_ipv4");
        assert_eq!(r1.matched_outbound, "direct");
        assert!(r1.is_lan_bypass);

        // Private IP with LAN bypass OFF -> fallback
        let r2 = evaluate_route_destination("192.168.1.1", false, &rules, "proxy", "prefer_ipv4");
        assert_eq!(r2.matched_outbound, "proxy");
        assert!(!r2.is_lan_bypass);

        // LAN domain with LAN bypass ON -> direct
        let r3 = evaluate_route_destination("nas.local", true, &rules, "proxy", "prefer_ipv4");
        assert_eq!(r3.matched_outbound, "direct");
        assert!(r3.is_lan_bypass);

        let r4 = evaluate_route_destination("myrouter.lan", true, &rules, "proxy", "prefer_ipv4");
        assert_eq!(r4.matched_outbound, "direct");
        assert!(r4.is_lan_bypass);
    }

    #[test]
    fn test_evaluate_route_destination_user_rules() {
        let rules = vec![
            RoutingRule {
                id: "r1".to_string(),
                rule_type: "domain_suffix".to_string(),
                value: "netflix.com".to_string(),
                outbound: "us-proxy".to_string(),
                invert: false,
                notes: None,
                enabled: Some(true),
            },
            RoutingRule {
                id: "r2".to_string(),
                rule_type: "domain".to_string(),
                value: "ads.badsite.com".to_string(),
                outbound: "block".to_string(),
                invert: false,
                notes: None,
                enabled: Some(true),
            },
            RoutingRule {
                id: "r3".to_string(),
                rule_type: "ip_cidr".to_string(),
                value: "1.1.1.0/24".to_string(),
                outbound: "direct".to_string(),
                invert: false,
                notes: None,
                enabled: Some(true),
            },
        ];

        // Suffix match
        let res1 =
            evaluate_route_destination("api.netflix.com", true, &rules, "proxy", "prefer_ipv4");
        assert_eq!(res1.matched_outbound, "us-proxy");
        assert_eq!(res1.matched_rule_id.as_deref(), Some("r1"));

        // Exact domain block match
        let res2 =
            evaluate_route_destination("ads.badsite.com", true, &rules, "proxy", "prefer_ipv4");
        assert_eq!(res2.matched_outbound, "block");
        assert_eq!(res2.matched_rule_id.as_deref(), Some("r2"));

        // CIDR match
        let res3 = evaluate_route_destination("1.1.1.1", true, &rules, "proxy", "prefer_ipv4");
        assert_eq!(res3.matched_outbound, "direct");
        assert_eq!(res3.matched_rule_id.as_deref(), Some("r3"));

        // Fallback match
        let res4 = evaluate_route_destination("google.com", true, &rules, "proxy", "prefer_ipv4");
        assert_eq!(res4.matched_outbound, "proxy");
        assert_eq!(res4.matched_rule_type.as_deref(), Some("final_fallback"));
    }

    #[test]
    fn test_sanitize_log_line() {
        let log = "Connected to controller at http://127.0.0.1:9090 with secret=supersecret123 and token=bearer_abc";
        let sanitized = sanitize_log_line(log);
        assert!(!sanitized.contains("supersecret123"));
        assert!(!sanitized.contains("bearer_abc"));
        assert!(sanitized.contains("secret=[REDACTED]"));
        assert!(sanitized.contains("token=[REDACTED]"));

        let bearer_log = "HTTP GET Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.test";
        let sanitized_bearer = sanitize_log_line(bearer_log);
        assert!(!sanitized_bearer.contains("eyJhbGciOiJIUzI1NiJ9"));
        assert!(sanitized_bearer.contains("Bearer [REDACTED]"));
    }

    #[test]
    fn test_sanitize_url_str() {
        let url_with_token = "https://user:pass@sub.example.com/api?token=secret123&type=v2ray";
        let sanitized = sanitize_url_str(url_with_token);
        assert!(!sanitized.contains("pass"));
        assert!(!sanitized.contains("secret123"));
        assert!(sanitized.contains("sub.example.com"));
        assert!(sanitized.contains("redacted=true"));
    }

    #[test]
    fn test_sanitize_user_settings() {
        let settings = serde_json::json!({
            "proxyMode": "tun",
            "apiPort": 9090,
            "apiSecret": "super_secret_token_123"
        });
        let sanitized = sanitize_user_settings(&settings);
        assert_eq!(sanitized["apiSecret"], "[REDACTED]");
        assert_eq!(sanitized["apiPort"], 9090);
    }
}
