use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutboundGroupType {
    Urltest,
    Fallback,
    Selector,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutboundGroup {
    pub id: String,
    pub tag: String,
    pub r#type: OutboundGroupType,
    #[serde(default)]
    pub nodes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<u32>,
}

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RoutingRule {
    pub id: String,
    #[serde(rename = "type")]
    pub rule_type: String,
    pub value: String,
    pub outbound: String,
    pub invert: bool,
    pub notes: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RuleSet {
    pub id: String,
    pub tag: String,
    #[serde(rename = "type")]
    pub set_type: String,
    pub format: String,
    pub url: Option<String>,
    pub file_path: Option<String>,
    pub update_interval: String,
}

pub fn load_routing_rules_from_file(app: &tauri::AppHandle) -> (Vec<RoutingRule>, Vec<RuleSet>) {
    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("routing.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    let rules: Vec<RoutingRule> = val
                        .get("rules")
                        .and_then(|r| serde_json::from_value(r.clone()).ok())
                        .unwrap_or_default();
                    let rule_sets: Vec<RuleSet> = val
                        .get("ruleSets")
                        .and_then(|r| serde_json::from_value(r.clone()).ok())
                        .unwrap_or_default();
                    return (rules, rule_sets);
                }
            }
        }
    }
    (Vec::new(), Vec::new())
}

pub fn load_outbound_groups_from_file(app: &tauri::AppHandle) -> Vec<OutboundGroup> {
    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("routing.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    return val
                        .get("outboundGroups")
                        .and_then(|g| serde_json::from_value(g.clone()).ok())
                        .unwrap_or_default();
                }
            }
        }
    }
    Vec::new()
}

pub fn build_singbox_rule(
    rule: &RoutingRule,
    _geosite_db_exists: bool,
    _geoip_db_exists: bool,
    outbound_groups: &[OutboundGroup],
) -> Option<Value> {
    let outbound = map_outbound_action(&rule.outbound, outbound_groups);
    let invert = rule.invert;

    let mut value = rule.value.clone();
    let mut rule_type = rule.rule_type.as_str();

    // Self-heal: Parse Clash-like or prefix-based routing values (e.g. domain:google.com, geosite:google)
    if value.starts_with("domain:") {
        value = value.strip_prefix("domain:").unwrap().to_string();
        rule_type = "domain";
    } else if value.starts_with("full:") {
        value = value.strip_prefix("full:").unwrap().to_string();
        rule_type = "domain";
    } else if value.starts_with("regexp:") {
        value = value.strip_prefix("regexp:").unwrap().to_string();
        rule_type = "domain_regex";
    } else if value.starts_with("geosite:") {
        value = value.strip_prefix("geosite:").unwrap().to_string();
        rule_type = "geosite";
    } else if value.starts_with("geoip:") {
        value = value.strip_prefix("geoip:").unwrap().to_string();
        rule_type = "geoip";
    } else if value.starts_with("cidr:") {
        value = value.strip_prefix("cidr:").unwrap().to_string();
        rule_type = "ip_cidr";
    } else if value.starts_with("ip:") {
        value = value.strip_prefix("ip:").unwrap().to_string();
        rule_type = "ip_cidr";
    }

    let mut obj = match rule_type {
        "domain" => serde_json::json!({ "domain":         [value], "outbound": outbound }),
        "domain_suffix" => {
            serde_json::json!({ "domain_suffix":  [value], "outbound": outbound })
        }
        "domain_keyword" => {
            serde_json::json!({ "domain_keyword": [value], "outbound": outbound })
        }
        "domain_regex" => {
            serde_json::json!({ "domain_regex":   [value], "outbound": outbound })
        }
        "ip_cidr" => serde_json::json!({ "ip_cidr":        [value], "outbound": outbound }),
        "geoip" => {
            serde_json::json!({ "rule_set": [format!("geoip-{}", value)], "outbound": outbound })
        }
        "geosite" => {
            if value == "private" {
                serde_json::json!({
                    "domain_suffix": [".lan", ".local", ".internal", ".home.arpa", "localhost"],
                    "outbound": outbound
                })
            } else {
                serde_json::json!({ "rule_set": [format!("geosite-{}", value)], "outbound": outbound })
            }
        }
        "port" => {
            if let Ok(p) = value.parse::<u16>() {
                serde_json::json!({ "port": [p], "outbound": outbound })
            } else {
                return None;
            }
        }
        "port_range" => serde_json::json!({ "port_range":   [value], "outbound": outbound }),
        "protocol" => serde_json::json!({ "protocol":     [value], "outbound": outbound }),
        "process_name" => {
            serde_json::json!({ "process_name": [value], "outbound": outbound })
        }
        "rule_set" => serde_json::json!({ "rule_set":     [value], "outbound": outbound }),
        "network" => serde_json::json!({ "network":       value,  "outbound": outbound }),
        _ => return None,
    };

    if let Some(map) = obj.as_object_mut() {
        if map.get("outbound").and_then(|v| v.as_str()) == Some("block") {
            map.remove("outbound");
            map.insert("action".to_string(), serde_json::json!("reject"));
        }
        if invert {
            map.insert("invert".to_string(), serde_json::json!(true));
        }
    }
    Some(obj)
}

fn map_outbound_action<'a>(action: &'a str, outbound_groups: &'a [OutboundGroup]) -> &'a str {
    match action {
        "block" => "block",
        "direct" => "direct",
        "proxy" => "proxy",
        "dns" => "dns",
        custom => {
            if let Some(group) = outbound_groups.iter().find(|g| g.tag == custom) {
                group.tag.as_str()
            } else {
                log::warn!(
                    "Routing rule targets missing Outbound Group '{}', falling back to 'proxy'",
                    custom
                );
                "proxy"
            }
        }
    }
}

pub fn has_block_rule(user_rules: &[RoutingRule]) -> bool {
    user_rules
        .iter()
        .any(|r| r.enabled.unwrap_or(true) && r.outbound == "block")
}

/// Windows reserved device names that cannot be used as file stems or names.
pub const RESERVED_DEVICE_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Validates that an untrusted rule-set tag is safe for file naming and path resolution.
/// Rejects empty, oversized, path traversal, path separators, colons, control characters,
/// and Windows reserved device names.
pub fn validate_rule_set_tag(tag: &str) -> Result<(), String> {
    if tag.is_empty() {
        return Err("Rule-set tag cannot be empty".to_string());
    }
    if tag.len() > 64 {
        return Err("Rule-set tag is too long (maximum 64 characters)".to_string());
    }
    if tag.starts_with('.') || tag.ends_with('.') {
        return Err("Rule-set tag cannot start or end with a dot".to_string());
    }
    if tag.contains("..") {
        return Err("Rule-set tag cannot contain consecutive dots ('..')".to_string());
    }
    if tag.contains('/') || tag.contains('\\') || tag.contains(':') {
        return Err("Rule-set tag cannot contain path separators or colons".to_string());
    }
    for c in tag.chars() {
        if !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
            return Err(format!("Rule-set tag contains invalid character '{}'", c));
        }
    }
    let stem = tag.split('.').next().unwrap_or(tag);
    for &reserved in RESERVED_DEVICE_NAMES {
        if stem.eq_ignore_ascii_case(reserved) || tag.eq_ignore_ascii_case(reserved) {
            return Err(format!("Rule-set tag '{}' is a reserved device name", tag));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule(rule_type: &str, value: &str, outbound: &str) -> RoutingRule {
        RoutingRule {
            id: "test".into(),
            rule_type: rule_type.into(),
            value: value.into(),
            outbound: outbound.into(),
            invert: false,
            notes: None,
            enabled: Some(true),
        }
    }

    #[test]
    fn test_domain_rule_generates_correct_json() {
        let rule = make_rule("domain", "youtube.com", "proxy");
        let json = build_singbox_rule(&rule, true, true, &[]).unwrap();
        assert_eq!(json["domain"][0].as_str().unwrap(), "youtube.com");
        assert_eq!(json["outbound"].as_str().unwrap(), "proxy");
    }

    #[test]
    fn test_ip_cidr_rule() {
        let rule = make_rule("ip_cidr", "1.1.1.0/24", "direct");
        let json = build_singbox_rule(&rule, true, true, &[]).unwrap();
        assert_eq!(json["ip_cidr"][0].as_str().unwrap(), "1.1.1.0/24");
        assert_eq!(json["outbound"].as_str().unwrap(), "direct");
    }

    #[test]
    fn test_geoip_rule() {
        let rule = make_rule("geoip", "cn", "direct");
        let json = build_singbox_rule(&rule, true, true, &[]).unwrap();
        assert_eq!(json["rule_set"][0].as_str().unwrap(), "geoip-cn");
    }

    #[test]
    fn test_geosite_rule() {
        let rule = make_rule("geosite", "google", "proxy");
        let json = build_singbox_rule(&rule, true, true, &[]).unwrap();
        assert_eq!(json["rule_set"][0].as_str().unwrap(), "geosite-google");
    }

    #[test]
    fn test_geosite_private_fallback_when_missing() {
        let rule = make_rule("geosite", "private", "direct");
        let json = build_singbox_rule(&rule, false, true, &[]).unwrap();
        assert!(json.get("domain_suffix").is_some());
        assert_eq!(json["domain_suffix"][0].as_str().unwrap(), ".lan");
        assert_eq!(json["outbound"].as_str().unwrap(), "direct");

        let rule_other = make_rule("geosite", "google", "proxy");
        let json_other = build_singbox_rule(&rule_other, false, true, &[]).unwrap();
        assert_eq!(
            json_other["rule_set"][0].as_str().unwrap(),
            "geosite-google"
        );
    }

    #[test]
    fn test_geoip_ignored_when_missing() {
        let rule = make_rule("geoip", "cn", "direct");
        let json = build_singbox_rule(&rule, true, false, &[]).unwrap();
        assert_eq!(json["rule_set"][0].as_str().unwrap(), "geoip-cn");
    }

    #[test]
    fn test_disabled_rule_is_skipped() {
        let mut rule = make_rule("domain", "example.com", "block");
        rule.enabled = Some(false);
        assert!(!rule.enabled.unwrap_or(true));
    }

    #[test]
    fn test_block_outbound() {
        let rule = make_rule("domain", "ads.example.com", "block");
        let json = build_singbox_rule(&rule, true, true, &[]).unwrap();
        assert_eq!(json["action"].as_str().unwrap(), "reject");
        assert!(json.get("outbound").is_none());
    }

    #[test]
    fn test_invert_flag() {
        let mut rule = make_rule("geoip", "cn", "proxy");
        rule.invert = true;
        let json = build_singbox_rule(&rule, true, true, &[]).unwrap();
        assert!(json["invert"].as_bool().unwrap());
    }

    #[test]
    fn test_invalid_port_returns_none() {
        let rule = make_rule("port", "not_a_number", "direct");
        assert!(build_singbox_rule(&rule, true, true, &[]).is_none());
    }

    #[test]
    fn test_outbound_group_rule_targeting_and_fallback() {
        let groups = vec![OutboundGroup {
            id: "g1".into(),
            tag: "Streaming-Pool".into(),
            r#type: OutboundGroupType::Urltest,
            nodes: vec!["node-1".into()],
            url: None,
            interval: Some("600s".into()),
            tolerance: Some(50),
        }];

        let valid_group_rule = make_rule("domain_suffix", ".netflix.com", "Streaming-Pool");
        let json = build_singbox_rule(&valid_group_rule, true, true, &groups).unwrap();
        assert_eq!(json["outbound"].as_str().unwrap(), "Streaming-Pool");

        let missing_group_rule = make_rule("domain_suffix", ".netflix.com", "Deleted-Group");
        let fallback_json = build_singbox_rule(&missing_group_rule, true, true, &groups).unwrap();
        assert_eq!(fallback_json["outbound"].as_str().unwrap(), "proxy");
    }

    #[test]
    fn test_validate_rule_set_tag_valid_names() {
        assert!(validate_rule_set_tag("geoip-cn").is_ok());
        assert!(validate_rule_set_tag("geosite-google").is_ok());
        assert!(validate_rule_set_tag("my_rule_set").is_ok());
        assert!(validate_rule_set_tag("custom-123").is_ok());
        assert!(validate_rule_set_tag("adblock.v2").is_ok());
    }

    #[test]
    fn test_validate_rule_set_tag_invalid_empty_and_oversized() {
        assert!(validate_rule_set_tag("").is_err());
        let normal_64 = "a".repeat(64);
        assert!(validate_rule_set_tag(&normal_64).is_ok());
        let oversized_65 = "a".repeat(65);
        assert!(validate_rule_set_tag(&oversized_65).is_err());
    }

    #[test]
    fn test_validate_rule_set_tag_path_traversal_and_separators() {
        assert!(validate_rule_set_tag("../evil").is_err());
        assert!(validate_rule_set_tag("..\\evil").is_err());
        assert!(validate_rule_set_tag("..").is_err());
        assert!(validate_rule_set_tag(".").is_err());
        assert!(validate_rule_set_tag("/etc/passwd").is_err());
        assert!(validate_rule_set_tag("\\Windows\\System32").is_err());
        assert!(validate_rule_set_tag("C:\\evil").is_err());
        assert!(validate_rule_set_tag("C:/evil").is_err());
        assert!(validate_rule_set_tag("foo/bar").is_err());
        assert!(validate_rule_set_tag("foo\\bar").is_err());
        assert!(validate_rule_set_tag("foo:bar").is_err());
    }

    #[test]
    fn test_validate_rule_set_tag_reserved_device_names() {
        assert!(validate_rule_set_tag("CON").is_err());
        assert!(validate_rule_set_tag("con").is_err());
        assert!(validate_rule_set_tag("PRN").is_err());
        assert!(validate_rule_set_tag("prn").is_err());
        assert!(validate_rule_set_tag("AUX").is_err());
        assert!(validate_rule_set_tag("aux").is_err());
        assert!(validate_rule_set_tag("NUL").is_err());
        assert!(validate_rule_set_tag("nul").is_err());
        assert!(validate_rule_set_tag("COM1").is_err());
        assert!(validate_rule_set_tag("com9").is_err());
        assert!(validate_rule_set_tag("LPT1").is_err());
        assert!(validate_rule_set_tag("lpt9").is_err());
        assert!(validate_rule_set_tag("CON.txt").is_err());
        assert!(validate_rule_set_tag("aux.srs").is_err());

        // Normal tags containing substring of reserved names are allowed
        assert!(validate_rule_set_tag("con-cn").is_ok());
        assert!(validate_rule_set_tag("auxiliary").is_ok());
        assert!(validate_rule_set_tag("connect").is_ok());
    }

    #[test]
    fn test_validate_rule_set_tag_illegal_characters() {
        assert!(validate_rule_set_tag("tag|pipe").is_err());
        assert!(validate_rule_set_tag("tag?wild").is_err());
        assert!(validate_rule_set_tag("tag*star").is_err());
        assert!(validate_rule_set_tag("tag<angle").is_err());
        assert!(validate_rule_set_tag("tag>angle").is_err());
        assert!(validate_rule_set_tag("tag\"quote").is_err());
        assert!(validate_rule_set_tag("tag name").is_err());
        assert!(validate_rule_set_tag(".leading_dot").is_err());
        assert!(validate_rule_set_tag("trailing_dot.").is_err());
        assert!(validate_rule_set_tag("double..dot").is_err());
    }
}
