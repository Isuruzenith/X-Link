use std::collections::HashMap;
use serde_json::Value;

pub mod clash;
pub mod raw_uri;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SingBoxOutbound {
    #[serde(rename = "type")]
    pub outbound_type: String,
    pub tag: String,
    #[serde(flatten)]
    pub fields: HashMap<String, Value>,
}

pub enum SubscriptionFormat {
    SingBoxNative,
    ClashYaml,
    RawUriList,
    Unknown,
}

pub fn detect_format(raw: &[u8]) -> SubscriptionFormat {
    let raw_str = String::from_utf8_lossy(raw);
    let trimmed = raw_str.trim();

    // 1. Check if SingBoxNative JSON
    if trimmed.starts_with('{') && trimmed.contains("\"outbounds\"") {
        return SubscriptionFormat::SingBoxNative;
    }

    // 2. Check if Clash YAML
    if trimmed.contains("proxies:") || (trimmed.contains("proxies") && trimmed.contains("- name:")) {
        return SubscriptionFormat::ClashYaml;
    }

    // 3. Check if RawUriList
    if trimmed.contains("vmess://") || trimmed.contains("vless://") || trimmed.contains("ss://") || trimmed.contains("trojan://") {
        return SubscriptionFormat::RawUriList;
    }

    SubscriptionFormat::Unknown
}

pub fn adapt(raw: &[u8]) -> Result<Vec<SingBoxOutbound>, String> {
    // Attempt base64 decode first (supporting unpadded base64 and stripping internal whitespace)
    let raw_str = String::from_utf8_lossy(raw);
    let mut base64_candidate = raw_str.replace(|c: char| c.is_ascii_whitespace(), "");
    while base64_candidate.len() % 4 != 0 {
        base64_candidate.push('=');
    }

    let decoded_bytes = match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_candidate.as_bytes()) {
        Ok(decoded) => decoded,
        Err(_) => {
            // Also try URL-safe base64
            match base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE, base64_candidate.as_bytes()) {
                Ok(decoded) => decoded,
                Err(_) => raw.to_vec(),
            }
        }
    };

    match detect_format(&decoded_bytes) {
        SubscriptionFormat::SingBoxNative => {
            // Parse as full sing-box JSON and extract outbounds
            #[derive(serde::Deserialize)]
            struct SingboxConfig {
                outbounds: Option<Vec<SingBoxOutbound>>,
            }
            let config: SingboxConfig = serde_json::from_slice(&decoded_bytes)
                .map_err(|e| format!("Failed to parse sing-box native JSON: {}", e))?;
            
            Ok(config.outbounds.unwrap_or_default())
        }
        SubscriptionFormat::ClashYaml => {
            clash::adapt(&decoded_bytes)
        }
        SubscriptionFormat::RawUriList => {
            raw_uri::adapt(&decoded_bytes)
        }
        SubscriptionFormat::Unknown => {
            let hint = String::from_utf8_lossy(&decoded_bytes);
            let limit = std::cmp::min(100, hint.len());
            Err(format!("subscription_format_unknown: {}", &hint[0..limit]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapt_multiline_base64() {
        // Base64 for: ss://YWVzLTI1Ni1nY206c2VjcmV0cGFzc3dvcmQ@1.2.3.4:8388#Shadowsocks%20Node
        // split with newlines and carriage returns
        let multiline_b64 = "c3M6Ly9ZV1Z6TFRJMU5pMW5ZMjA2\r\nY2JWamNtVjBjR0Z6YzNkdmNtU\n\tUAxLjIuMy40OjgzODgjU2hhZG93c29ja3MlMjBOb2Rl";
        let res = adapt(multiline_b64.as_bytes()).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].outbound_type, "shadowsocks");
        assert_eq!(res[0].tag, "Shadowsocks Node");
    }
}
