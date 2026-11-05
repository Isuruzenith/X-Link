use crate::config::adapters::SingBoxOutbound;
use crate::config::rules::{OutboundGroup, OutboundGroupType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use tauri::AppHandle;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

pub const RESERVED_OUTBOUND_TAGS: &[&str] = &["proxy", "direct", "block", "dns"];
pub const MAX_AUTOMATED_GROUP_NODES: usize = 10;
pub const MIN_HEALTH_CHECK_INTERVAL_SECS: u64 = 300;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigValidationError {
    pub code: String,
    pub offending_tags: Vec<String>,
    pub cycle_path: Vec<String>,
    pub message: String,
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ConfigValidationError {}

/// Parses duration strings such as "300s", "10m", "1h", or plain seconds into total seconds.
pub fn parse_interval_seconds(interval: &str) -> Option<u64> {
    let trimmed = interval.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(s) = trimmed.strip_suffix('s') {
        s.trim().parse::<u64>().ok()
    } else if let Some(m) = trimmed.strip_suffix('m') {
        m.trim().parse::<u64>().ok().map(|v| v.saturating_mul(60))
    } else if let Some(h) = trimmed.strip_suffix('h') {
        h.trim().parse::<u64>().ok().map(|v| v.saturating_mul(3600))
    } else {
        trimmed.parse::<u64>().ok()
    }
}

/// Pre-validation normalization: strips `detour` fields referencing non-existent or pruned node tags.
/// Note: detour targets matching an OutboundGroup tag are preserved here so `validate_detour_graph`
/// can reject them explicitly with `DETOUR_TO_GROUP`.
pub fn normalize_detour_references(
    nodes: &mut [SingBoxOutbound],
    groups: &[OutboundGroup],
) -> Vec<String> {
    let valid_node_tags: HashSet<String> = nodes.iter().map(|n| n.tag.clone()).collect();
    let group_tags: HashSet<String> = groups.iter().map(|g| g.tag.clone()).collect();
    let mut stripped_nodes = Vec::new();

    for node in nodes.iter_mut() {
        let detour_target = node
            .fields
            .get("detour")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string());

        if let Some(target) = detour_target {
            if target.is_empty() {
                node.fields.remove("detour");
            } else if !valid_node_tags.contains(&target) && !group_tags.contains(&target) {
                log::warn!(
                    "Stripping dangling detour reference '{}' from node '{}'",
                    target,
                    node.tag
                );
                node.fields.remove("detour");
                stripped_nodes.push(node.tag.clone());
            }
        }
    }

    stripped_nodes
}

/// Validates Outbound Groups and Node Detour topology using Kahn's Directed Acyclic Graph (DAG) algorithm.
pub fn validate_detour_graph(
    nodes: &[SingBoxOutbound],
    groups: &[OutboundGroup],
) -> Result<(), ConfigValidationError> {
    let node_tag_set: HashSet<&str> = nodes.iter().map(|n| n.tag.as_str()).collect();
    let mut seen_group_tags: HashSet<&str> = HashSet::new();

    // 1. Validate Outbound Groups (reserved tags, collisions, 10-node cap, 300s interval floor)
    for group in groups {
        let tag_trimmed = group.tag.trim();
        if tag_trimmed.is_empty() {
            return Err(ConfigValidationError {
                code: "INVALID_GROUP_TAG".to_string(),
                offending_tags: vec![group.tag.clone()],
                cycle_path: vec![],
                message: "Outbound group tag cannot be empty".to_string(),
            });
        }

        if RESERVED_OUTBOUND_TAGS
            .iter()
            .any(|reserved| reserved.eq_ignore_ascii_case(tag_trimmed))
        {
            return Err(ConfigValidationError {
                code: "RESERVED_TAG".to_string(),
                offending_tags: vec![group.tag.clone()],
                cycle_path: vec![],
                message: format!(
                    "Outbound group tag '{}' is a reserved system tag",
                    group.tag
                ),
            });
        }

        if node_tag_set.contains(tag_trimmed) || !seen_group_tags.insert(tag_trimmed) {
            return Err(ConfigValidationError {
                code: "TAG_COLLISION".to_string(),
                offending_tags: vec![group.tag.clone()],
                cycle_path: vec![],
                message: format!(
                    "Outbound group tag '{}' collides with an existing node or group tag",
                    group.tag
                ),
            });
        }

        if matches!(
            group.r#type,
            OutboundGroupType::Urltest | OutboundGroupType::Fallback
        ) && group.nodes.len() > MAX_AUTOMATED_GROUP_NODES
        {
            return Err(ConfigValidationError {
                code: "GROUP_SIZE_EXCEEDED".to_string(),
                offending_tags: vec![group.tag.clone()],
                cycle_path: vec![],
                message: format!(
                    "Automated outbound group '{}' contains {} nodes (maximum allowed is {})",
                    group.tag,
                    group.nodes.len(),
                    MAX_AUTOMATED_GROUP_NODES
                ),
            });
        }

        if let Some(ref interval_str) = group.interval {
            if !interval_str.trim().is_empty() {
                match parse_interval_seconds(interval_str) {
                    Some(secs) if secs < MIN_HEALTH_CHECK_INTERVAL_SECS => {
                        return Err(ConfigValidationError {
                            code: "INTERVAL_TOO_SHORT".to_string(),
                            offending_tags: vec![group.tag.clone()],
                            cycle_path: vec![],
                            message: format!(
                                "Health-check interval '{}' on group '{}' is below the 300s minimum floor",
                                interval_str, group.tag
                            ),
                        });
                    }
                    None => {
                        return Err(ConfigValidationError {
                            code: "INVALID_INTERVAL".to_string(),
                            offending_tags: vec![group.tag.clone()],
                            cycle_path: vec![],
                            message: format!(
                                "Invalid health-check interval '{}' on group '{}'",
                                interval_str, group.tag
                            ),
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    // 2. Normalize dangling detour references on a working copy before DAG checks
    let mut normalized_nodes = nodes.to_vec();
    normalize_detour_references(&mut normalized_nodes, groups);

    let node_by_tag: HashMap<&str, &SingBoxOutbound> = normalized_nodes
        .iter()
        .map(|n| (n.tag.as_str(), n))
        .collect();

    // 3. Check detour to group & build adjacency map
    // Edge u -> v means node u detours through node v
    let mut detour_target_of: HashMap<&str, &str> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = normalized_nodes
        .iter()
        .map(|n| (n.tag.as_str(), 0))
        .collect();
    let mut adj: HashMap<&str, Vec<&str>> = normalized_nodes
        .iter()
        .map(|n| (n.tag.as_str(), Vec::new()))
        .collect();

    for node in &normalized_nodes {
        if let Some(target) = node
            .fields
            .get("detour")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            if seen_group_tags.contains(target) {
                return Err(ConfigValidationError {
                    code: "DETOUR_TO_GROUP".to_string(),
                    offending_tags: vec![node.tag.clone(), target.to_string()],
                    cycle_path: vec![],
                    message: format!(
                        "Node '{}' cannot detour through outbound group '{}'; detours must target a primary node",
                        node.tag, target
                    ),
                });
            }

            if node_by_tag.contains_key(target) {
                detour_target_of.insert(node.tag.as_str(), target);
                adj.entry(node.tag.as_str()).or_default().push(target);
                *in_degree.entry(target).or_insert(0) += 1;
            }
        }
    }

    // 4. Kahn's Algorithm topological sort for cycle detection
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter_map(|(&tag, &deg)| if deg == 0 { Some(tag) } else { None })
        .collect();

    let mut visited_count = 0usize;
    while let Some(u) = queue.pop_front() {
        visited_count += 1;
        if let Some(neighbors) = adj.get(u) {
            for &v in neighbors {
                if let Some(deg) = in_degree.get_mut(v) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(v);
                    }
                }
            }
        }
    }

    if visited_count < normalized_nodes.len() {
        // Extract deterministic cycle path and offending tags
        let mut offending: Vec<String> = normalized_nodes
            .iter()
            .filter(|n| in_degree.get(n.tag.as_str()).copied().unwrap_or(0) > 0)
            .map(|n| n.tag.clone())
            .collect();
        offending.sort();

        let mut cycle_path = Vec::new();
        if let Some(start_tag) = offending.first() {
            let mut curr = start_tag.as_str();
            let mut seen_in_walk: Vec<&str> = Vec::new();
            while let Some(&next) = detour_target_of.get(curr) {
                seen_in_walk.push(curr);
                if let Some(pos) = seen_in_walk.iter().position(|&t| t == next) {
                    for &t in &seen_in_walk[pos..] {
                        cycle_path.push(t.to_string());
                    }
                    cycle_path.push(next.to_string());
                    break;
                }
                curr = next;
            }
        }

        return Err(ConfigValidationError {
            code: "CYCLIC_DETOUR".to_string(),
            offending_tags: offending,
            cycle_path: cycle_path.clone(),
            message: format!(
                "Circular detour dependency detected: {}",
                cycle_path.join(" -> ")
            ),
        });
    }

    // 5. Enforce 2-hop cap (Relay -> Exit) and UDP protocol compatibility
    for node in &normalized_nodes {
        if let Some(&relay_tag) = detour_target_of.get(node.tag.as_str()) {
            // If the relay itself detours to a third node, depth > 2 hops
            if let Some(&third_tag) = detour_target_of.get(relay_tag) {
                return Err(ConfigValidationError {
                    code: "DETOUR_DEPTH_EXCEEDED".to_string(),
                    offending_tags: vec![
                        node.tag.clone(),
                        relay_tag.to_string(),
                        third_tag.to_string(),
                    ],
                    cycle_path: vec![
                        node.tag.clone(),
                        relay_tag.to_string(),
                        third_tag.to_string(),
                    ],
                    message: format!(
                        "Detour chain '{} -> {} -> {}' exceeds maximum 2-hop depth",
                        node.tag, relay_tag, third_tag
                    ),
                });
            }

            // Check UDP-dependent exit protocols over TCP-only HTTP relays
            if let Some(relay_node) = node_by_tag.get(relay_tag) {
                let exit_proto = node.outbound_type.to_lowercase();
                let relay_proto = relay_node.outbound_type.to_lowercase();
                let is_udp_dependent = matches!(
                    exit_proto.as_str(),
                    "tuic" | "hysteria2" | "hysteria" | "wireguard"
                );
                if is_udp_dependent && relay_proto == "http" {
                    return Err(ConfigValidationError {
                        code: "INCOMPATIBLE_DETOUR_PROTOCOL".to_string(),
                        offending_tags: vec![node.tag.clone(), relay_tag.to_string()],
                        cycle_path: vec![],
                        message: format!(
                            "UDP-dependent node '{}' ({}) cannot detour through TCP-only HTTP proxy '{}'",
                            node.tag, node.outbound_type, relay_tag
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}

pub async fn validate_singbox_config(app: &AppHandle, config_path: &Path) -> Result<(), String> {
    // Verify sing-box sidecar binary version compatibility before validating config
    crate::commands::system::ensure_sidecar_compatible(app)
        .await
        .map_err(|e| format!("Configuration validation aborted: {}", e))?;

    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let _ = std::fs::create_dir_all(&app_data_dir);

    let sidecar = app
        .shell()
        .sidecar("sing-box")
        .map_err(|e| format!("Failed to find sing-box sidecar: {}", e))?;

    let output = sidecar
        .args([
            "check",
            "-c",
            config_path.to_str().unwrap(),
            "-D",
            app_data_dir.to_str().unwrap(),
        ])
        .env("ENABLE_DEPRECATED_GEOSITE", "true")
        .env("ENABLE_DEPRECATED_GEOIP", "true")
        .env("ENABLE_DEPRECATED_LEGACY_DNS_FAKEIP_OPTIONS", "true")
        .env("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS", "true")
        .env("ENABLE_DEPRECATED_OUTBOUND_DNS_RULE_ITEM", "true")
        .output()
        .await
        .map_err(|e| format!("Failed to execute sing-box check: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let err_str = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(format!("sing-box check failed: {}", err_str.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(tag: &str, outbound_type: &str, detour: Option<&str>) -> SingBoxOutbound {
        let mut fields = HashMap::new();
        fields.insert("server".to_string(), serde_json::json!("1.2.3.4"));
        fields.insert("server_port".to_string(), serde_json::json!(443));
        if let Some(d) = detour {
            fields.insert("detour".to_string(), serde_json::json!(d));
        }
        SingBoxOutbound {
            outbound_type: outbound_type.to_string(),
            tag: tag.to_string(),
            fields,
        }
    }

    #[test]
    fn test_valid_1_hop_and_2_hop_detour_chains() {
        let nodes = vec![
            make_node("relay-1", "vless", None),
            make_node("exit-1", "vless", Some("relay-1")),
            make_node("standalone", "trojan", None),
        ];
        assert!(validate_detour_graph(&nodes, &[]).is_ok());
    }

    #[test]
    fn test_2_hop_self_and_mutual_cycle_rejected() {
        let nodes = vec![
            make_node("node-a", "vless", Some("node-b")),
            make_node("node-b", "vless", Some("node-a")),
        ];
        let err = validate_detour_graph(&nodes, &[]).unwrap_err();
        assert_eq!(err.code, "CYCLIC_DETOUR");
        assert!(err.offending_tags.contains(&"node-a".to_string()));
        assert!(err.offending_tags.contains(&"node-b".to_string()));
        assert!(err.cycle_path.len() >= 3);
    }

    #[test]
    fn test_3_hop_cycle_rejected_with_cycle_path() {
        let nodes = vec![
            make_node("node-a", "vless", Some("node-b")),
            make_node("node-b", "vless", Some("node-c")),
            make_node("node-c", "vless", Some("node-a")),
        ];
        let err = validate_detour_graph(&nodes, &[]).unwrap_err();
        assert_eq!(err.code, "CYCLIC_DETOUR");
        assert_eq!(err.offending_tags.len(), 3);
        assert_eq!(err.cycle_path.first(), err.cycle_path.last());
    }

    #[test]
    fn test_detour_depth_exceeding_2_hops_rejected() {
        let nodes = vec![
            make_node("hop-1", "vless", None),
            make_node("hop-2", "vless", Some("hop-1")),
            make_node("hop-3", "vless", Some("hop-2")),
        ];
        let err = validate_detour_graph(&nodes, &[]).unwrap_err();
        assert_eq!(err.code, "DETOUR_DEPTH_EXCEEDED");
    }

    #[test]
    fn test_dangling_node_detour_is_stripped_cleanly() {
        let mut nodes = vec![make_node("exit-1", "vless", Some("deleted-relay"))];
        assert!(validate_detour_graph(&nodes, &[]).is_ok());
        let stripped = normalize_detour_references(&mut nodes, &[]);
        assert_eq!(stripped, vec!["exit-1".to_string()]);
        assert!(!nodes[0].fields.contains_key("detour"));
    }

    #[test]
    fn test_udp_protocol_over_http_relay_rejected() {
        let nodes = vec![
            make_node("http-relay", "http", None),
            make_node("tuic-exit", "tuic", Some("http-relay")),
        ];
        let err = validate_detour_graph(&nodes, &[]).unwrap_err();
        assert_eq!(err.code, "INCOMPATIBLE_DETOUR_PROTOCOL");
    }

    #[test]
    fn test_reserved_group_tags_and_collisions_rejected() {
        let nodes = vec![make_node("sg-01", "vless", None)];
        for reserved in ["proxy", "direct", "block", "dns"] {
            let group = OutboundGroup {
                id: "g1".into(),
                tag: reserved.into(),
                r#type: OutboundGroupType::Urltest,
                nodes: vec!["sg-01".into()],
                url: None,
                interval: Some("600s".into()),
                tolerance: Some(50),
            };
            let err = validate_detour_graph(&nodes, &[group]).unwrap_err();
            assert_eq!(err.code, "RESERVED_TAG");
        }

        let colliding_group = OutboundGroup {
            id: "g2".into(),
            tag: "sg-01".into(),
            r#type: OutboundGroupType::Selector,
            nodes: vec!["sg-01".into()],
            url: None,
            interval: None,
            tolerance: None,
        };
        let err = validate_detour_graph(&nodes, &[colliding_group]).unwrap_err();
        assert_eq!(err.code, "TAG_COLLISION");
    }

    #[test]
    fn test_automated_group_size_cap_and_interval_floor() {
        let nodes: Vec<SingBoxOutbound> = (1..=12)
            .map(|i| make_node(&format!("node-{}", i), "vless", None))
            .collect();

        let oversized_group = OutboundGroup {
            id: "g1".into(),
            tag: "Auto-Pool".into(),
            r#type: OutboundGroupType::Urltest,
            nodes: (1..=11).map(|i| format!("node-{}", i)).collect(),
            url: None,
            interval: Some("600s".into()),
            tolerance: Some(50),
        };
        let err = validate_detour_graph(&nodes, &[oversized_group]).unwrap_err();
        assert_eq!(err.code, "GROUP_SIZE_EXCEEDED");

        let short_interval_group = OutboundGroup {
            id: "g2".into(),
            tag: "Fast-Pool".into(),
            r#type: OutboundGroupType::Urltest,
            nodes: vec!["node-1".into()],
            url: None,
            interval: Some("120s".into()),
            tolerance: Some(50),
        };
        let err2 = validate_detour_graph(&nodes, &[short_interval_group]).unwrap_err();
        assert_eq!(err2.code, "INTERVAL_TOO_SHORT");
    }
}
