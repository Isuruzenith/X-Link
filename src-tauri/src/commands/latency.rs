use serde::Serialize;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LatencyResult {
    pub tag: String,
    pub latency_ms: Option<u32>,
    pub error: Option<String>,
}

/// Tests TCP connection latency to a server:port pair.
/// Returns the connection time in milliseconds or an error message.
#[tauri::command]
pub async fn test_node_latency(server: String, port: u16) -> Result<u32, String> {
    let ip = match crate::config::resolve_hostname_doh(&server).await {
        Some(ip) => ip,
        None => {
            // Fallback to system resolution
            let addr = format!("{}:{}", server, port);
            let socket_addrs = addr
                .to_socket_addrs()
                .map_err(|e| format!("Invalid address or DNS lookup failed: {}", e))?;
            match socket_addrs.into_iter().next() {
                Some(sa) => {
                    let ip = sa.ip();
                    if crate::config::is_fake_ip(ip) {
                        return Err("DNS lookup returned FakeIP".to_string());
                    }
                    ip
                }
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

        handles.push(tokio::spawn(async move {
            let ip = match crate::config::resolve_hostname_doh(&server).await {
                Some(ip) => Some(ip),
                None => {
                    // Fallback to system resolution
                    let addr = format!("{}:{}", server, port);
                    addr.to_socket_addrs()
                        .ok()
                        .and_then(|mut addrs| {
                            addrs.next().and_then(|sa| {
                                let ip = sa.ip();
                                if crate::config::is_fake_ip(ip) {
                                    None
                                } else {
                                    Some(ip)
                                }
                            })
                        })
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

    #[tokio::test]
    async fn test_resolve_hostname_doh() {
        let ip = crate::config::resolve_hostname_doh("one.one.one.one").await;
        assert!(ip.is_some());
        let ip_addr = ip.unwrap();
        assert!(ip_addr.is_ipv4() || ip_addr.is_ipv6());
    }
}
