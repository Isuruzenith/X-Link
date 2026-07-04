use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::Serialize;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};
use tauri::Manager;

async fn query_clash_api_delay(api_port: u16, api_secret: &str, tag: &str) -> Option<u32> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .no_proxy()
        .build()
        .ok()?;

    let encoded_tag = utf8_percent_encode(tag, NON_ALPHANUMERIC).to_string();
    let url = format!(
        "http://127.0.0.1:{}/proxies/{}/delay?url=http://www.gstatic.com/generate_204&timeout=2500",
        api_port, encoded_tag
    );

    let mut request = client.get(&url);
    if !api_secret.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", api_secret));
    }

    if let Ok(response) = request.send().await {
        #[derive(serde::Deserialize)]
        struct DelayResponse {
            delay: u32,
        }
        if let Ok(res) = response.json::<DelayResponse>().await {
            return Some(res.delay);
        }
    }

    None
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LatencyResult {
    pub tag: String,
    pub latency_ms: Option<u32>,
    pub error: Option<String>,
}

/// Resolves hostname using DNS-over-HTTPS (DoH) to bypass local FakeIP interception when VPN is active.
async fn resolve_hostname_doh(host: &str) -> Option<IpAddr> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Some(ip);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?;

    // Try Cloudflare DoH first
    let url = format!("https://cloudflare-dns.com/dns-query?name={}&type=A", host);
    if let Ok(response) = client
        .get(&url)
        .header("accept", "application/dns-json")
        .send()
        .await
    {
        #[derive(serde::Deserialize)]
        struct DnsAnswer {
            data: String,
        }
        #[derive(serde::Deserialize)]
        struct DnsResponse {
            #[serde(rename = "Answer")]
            answer: Option<Vec<DnsAnswer>>,
        }

        if let Ok(dns_res) = response.json::<DnsResponse>().await {
            if let Some(answers) = dns_res.answer {
                for ans in answers {
                    if let Ok(ip) = ans.data.trim().parse::<IpAddr>() {
                        return Some(ip);
                    }
                }
            }
        }
    }

    // Fallback to Google DoH
    let url = format!("https://dns.google/resolve?name={}&type=A", host);
    if let Ok(response) = client.get(&url).send().await {
        #[derive(serde::Deserialize)]
        struct DnsAnswer {
            data: String,
        }
        #[derive(serde::Deserialize)]
        struct DnsResponse {
            #[serde(rename = "Answer")]
            answer: Option<Vec<DnsAnswer>>,
        }

        if let Ok(dns_res) = response.json::<DnsResponse>().await {
            if let Some(answers) = dns_res.answer {
                for ans in answers {
                    if let Ok(ip) = ans.data.trim().parse::<IpAddr>() {
                        return Some(ip);
                    }
                }
            }
        }
    }

    None
}

/// Tests TCP connection latency to a server:port pair.
/// Returns the connection time in milliseconds or an error message.
#[tauri::command]
pub async fn test_node_latency(server: String, port: u16) -> Result<u32, String> {
    let ip = match resolve_hostname_doh(&server).await {
        Some(ip) => ip,
        None => {
            // Fallback to system resolution
            let addr = format!("{}:{}", server, port);
            let socket_addrs = addr
                .to_socket_addrs()
                .map_err(|e| format!("Invalid address or DNS lookup failed: {}", e))?;
            match socket_addrs.into_iter().next() {
                Some(sa) => sa.ip(),
                None => return Err("DNS lookup failed".to_string()),
            }
        }
    };

    let socket_addr = SocketAddr::new(ip, port);
    let result = tokio::task::spawn_blocking(move || {
        let start = Instant::now();
        if TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5)).is_ok() {
            return Ok(start.elapsed().as_millis() as u32);
        }
        Err("Connection failed".to_string())
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?;

    result
}

/// Tests all nodes in a profile by reading the config and attempting TCP connections.
/// Returns latency results for every node.
#[tauri::command]
pub async fn test_all_nodes(
    app: tauri::AppHandle,
    profile_id: String,
) -> Result<Vec<LatencyResult>, String> {
    let state = app.state::<crate::state::ProxyState>();
    let status = state.get_status();
    let (api_enabled, api_port, api_secret) = if let Ok(settings) = state.settings.lock() {
        (
            settings.api_enabled,
            settings.api_port,
            settings.api_secret.clone(),
        )
    } else {
        (false, 9090, "".to_string())
    };
    let is_connected = status == crate::state::ConnectionStatus::Connected;

    let nodes = crate::commands::config::get_profile_outbounds(app, profile_id)?;

    let mut handles = Vec::new();

    for node in &nodes {
        let tag = node
            .get("tag")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown")
            .to_string();
        let server = node
            .get("server")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let port = node
            .get("server_port")
            .and_then(|p| p.as_u64())
            .unwrap_or(443) as u16;

        if server.is_empty() {
            handles.push(tokio::spawn(async move {
                LatencyResult {
                    tag,
                    latency_ms: None,
                    error: Some("No server address".to_string()),
                }
            }));
            continue;
        }

        let api_secret_clone = api_secret.clone();
        handles.push(tokio::spawn(async move {
            // 1. Try Clash API if connected and enabled
            if is_connected && api_enabled {
                if let Some(delay) = query_clash_api_delay(api_port, &api_secret_clone, &tag).await
                {
                    return LatencyResult {
                        tag,
                        latency_ms: Some(delay),
                        error: None,
                    };
                }
            }

            // 2. Fallback to direct TCP ping
            let ip = match resolve_hostname_doh(&server).await {
                Some(ip) => Some(ip),
                None => {
                    // Fallback to system resolution
                    let addr = format!("{}:{}", server, port);
                    addr.to_socket_addrs()
                        .ok()
                        .and_then(|mut addrs| addrs.next().map(|sa| sa.ip()))
                }
            };

            let socket_addr = ip.map(|i| SocketAddr::new(i, port));

            let result = tokio::task::spawn_blocking(move || {
                let socket_addr = match socket_addr {
                    Some(sa) => sa,
                    None => return Err("DNS resolution failed".to_string()),
                };
                let start = Instant::now();
                if TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5)).is_ok() {
                    return Ok(start.elapsed().as_millis() as u32);
                }
                Err("Connection failed".to_string())
            })
            .await;

            match result {
                Ok(Ok(ms)) => LatencyResult {
                    tag,
                    latency_ms: Some(ms),
                    error: None,
                },
                Ok(Err(e)) => LatencyResult {
                    tag,
                    latency_ms: None,
                    error: Some(e),
                },
                Err(e) => LatencyResult {
                    tag,
                    latency_ms: None,
                    error: Some(format!("Task error: {}", e)),
                },
            }
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(result) = handle.await {
            results.push(result);
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_hostname_doh() {
        let ip = resolve_hostname_doh("one.one.one.one").await;
        assert!(ip.is_some());
        let ip_addr = ip.unwrap();
        assert!(ip_addr.is_ipv4() || ip_addr.is_ipv6());
    }
}
