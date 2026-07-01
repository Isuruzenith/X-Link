use serde::Serialize;
use std::net::{TcpStream, ToSocketAddrs};
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
    let addr = format!("{}:{}", server, port);

    // Run the blocking TCP connect in a separate thread to avoid blocking the async runtime
    let result = tokio::task::spawn_blocking(move || {
        let start = Instant::now();
        let socket_addrs = addr
            .to_socket_addrs()
            .map_err(|e| format!("Invalid address or DNS lookup failed: {}", e))?;

        for socket_addr in socket_addrs {
            if TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5)).is_ok() {
                return Ok(start.elapsed().as_millis() as u32);
            }
        }

        Err("Connection failed for all resolved addresses".to_string())
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
            let addr = format!("{}:{}", server, port);
            let result = tokio::task::spawn_blocking(move || {
                let start = Instant::now();
                match addr.to_socket_addrs() {
                    Ok(socket_addrs) => {
                        for socket_addr in socket_addrs {
                            if TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5))
                                .is_ok()
                            {
                                return Ok(start.elapsed().as_millis() as u32);
                            }
                        }
                        Err("Connection failed".to_string())
                    }
                    Err(e) => Err(format!("DNS resolution failed: {}", e)),
                }
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
