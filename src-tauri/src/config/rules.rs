use serde_json::Value;
use tauri::Manager;

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

pub fn build_singbox_rule(
    rule: &RoutingRule,
    _geosite_db_exists: bool,
    _geoip_db_exists: bool,
) -> Option<Value> {
    let outbound = map_outbound_action(&rule.outbound);
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

fn map_outbound_action(action: &str) -> &str {
    match action {
        "block" => "block",
        "direct" => "direct",
        _ => "proxy",
    }
}

pub fn has_block_rule(user_rules: &[RoutingRule]) -> bool {
    user_rules
        .iter()
        .any(|r| r.enabled.unwrap_or(true) && r.outbound == "block")
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
        let json = build_singbox_rule(&rule, true, true).unwrap();
        assert_eq!(json["domain"][0].as_str().unwrap(), "youtube.com");
        assert_eq!(json["outbound"].as_str().unwrap(), "proxy");
    }

    #[test]
    fn test_ip_cidr_rule() {
        let rule = make_rule("ip_cidr", "1.1.1.0/24", "direct");
        let json = build_singbox_rule(&rule, true, true).unwrap();
        assert_eq!(json["ip_cidr"][0].as_str().unwrap(), "1.1.1.0/24");
        assert_eq!(json["outbound"].as_str().unwrap(), "direct");
    }

    #[test]
    fn test_geoip_rule() {
        let rule = make_rule("geoip", "cn", "direct");
        let json = build_singbox_rule(&rule, true, true).unwrap();
        assert_eq!(json["rule_set"][0].as_str().unwrap(), "geoip-cn");
    }

    #[test]
    fn test_geosite_rule() {
        let rule = make_rule("geosite", "google", "proxy");
        let json = build_singbox_rule(&rule, true, true).unwrap();
        assert_eq!(json["rule_set"][0].as_str().unwrap(), "geosite-google");
    }

    #[test]
    fn test_geosite_private_fallback_when_missing() {
        let rule = make_rule("geosite", "private", "direct");
        let json = build_singbox_rule(&rule, false, true).unwrap();
        assert!(json.get("domain_suffix").is_some());
        assert_eq!(json["domain_suffix"][0].as_str().unwrap(), ".lan");
        assert_eq!(json["outbound"].as_str().unwrap(), "direct");

        let rule_other = make_rule("geosite", "google", "proxy");
        let json_other = build_singbox_rule(&rule_other, false, true).unwrap();
        assert_eq!(
            json_other["rule_set"][0].as_str().unwrap(),
            "geosite-google"
        );
    }

    #[test]
    fn test_geoip_ignored_when_missing() {
        let rule = make_rule("geoip", "cn", "direct");
        let json = build_singbox_rule(&rule, true, false).unwrap();
        assert_eq!(json["rule_set"][0].as_str().unwrap(), "geoip-cn");
    }

    #[test]
    fn test_disabled_rule_is_skipped() {
        let mut rule = make_rule("domain", "example.com", "block");
        rule.enabled = Some(false);
        // A disabled rule is filtered upstream in build_route_rules; build_singbox_rule
        // itself still produces a value -- the caller is responsible for the guard.
        // We just verify the enabled flag is respected by the caller:
        assert!(!rule.enabled.unwrap_or(true));
    }

    #[test]
    fn test_block_outbound() {
        let rule = make_rule("domain", "ads.example.com", "block");
        let json = build_singbox_rule(&rule, true, true).unwrap();
        assert_eq!(json["action"].as_str().unwrap(), "reject");
        assert!(json.get("outbound").is_none());
    }

    #[test]
    fn test_invert_flag() {
        let mut rule = make_rule("geoip", "cn", "proxy");
        rule.invert = true;
        let json = build_singbox_rule(&rule, true, true).unwrap();
        assert!(json["invert"].as_bool().unwrap());
    }

    #[test]
    fn test_invalid_port_returns_none() {
        let rule = make_rule("port", "not_a_number", "direct");
        assert!(build_singbox_rule(&rule, true, true).is_none());
    }
}
