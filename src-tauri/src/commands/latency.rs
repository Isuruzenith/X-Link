use serde::Serialize;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LatencyResult {
    pub tag: String,
    pub latency_ms: Option<u32>,
    pub error: Option<String>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct DnsAnswer {
    pub data: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct DnsResponse {
    #[serde(rename = "Answer")]
    pub answer: Option<Vec<DnsAnswer>>,
}

/// Parses a DoH JSON response body and extracts the first valid IP address.
pub fn parse_doh_json_response(body: &str) -> Option<IpAddr> {
    let dns_res: DnsResponse = serde_json::from_str(body).ok()?;
    parse_doh_response_struct(&dns_res)
}

/// Extracts the first valid IP address from a deserialized DoH response structure.
pub fn parse_doh_response_struct(dns_res: &DnsResponse) -> Option<IpAddr> {
    let answers = dns_res.answer.as_ref()?;
    for ans in answers {
        if let Ok(ip) = ans.data.trim().parse::<IpAddr>() {
            return Some(ip);
        }
    }
    None
}

/// Queries a single DoH endpoint URL and parses the response.
pub async fn query_doh_endpoint(client: &reqwest::Client, endpoint_url: &str) -> Option<IpAddr> {
    let response = client
        .get(endpoint_url)
        .header("accept", "application/dns-json")
        .send()
        .await
        .ok()?;
    let text = response.text().await.ok()?;
    parse_doh_json_response(&text)
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
    if let Some(ip) = query_doh_endpoint(&client, &url).await {
        return Some(ip);
    }

    // Fallback to Google DoH
    let url = format!("https://dns.google/resolve?name={}&type=A", host);
    if let Some(ip) = query_doh_endpoint(&client, &url).await {
        return Some(ip);
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
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_parse_doh_response_ipv4() {
        let fixture = r#"{
            "Status": 0,
            "TC": false,
            "RD": true,
            "RA": true,
            "AD": false,
            "CD": false,
            "Question": [{"name": "example.com.", "type": 1}],
            "Answer": [
                {"name": "example.com.", "type": 1, "TTL": 300, "data": "93.184.216.34"}
            ]
        }"#;

        let ip = parse_doh_json_response(fixture);
        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
    }

    #[test]
    fn test_parse_doh_response_ipv6() {
        let fixture = r#"{
            "Status": 0,
            "Answer": [
                {"name": "example.com.", "type": 28, "TTL": 300, "data": "2606:2800:220:1:248:1893:25c8:1946"}
            ]
        }"#;

        let ip = parse_doh_json_response(fixture);
        assert_eq!(
            ip,
            Some(IpAddr::V6(
                "2606:2800:220:1:248:1893:25c8:1946"
                    .parse::<Ipv6Addr>()
                    .unwrap()
            ))
        );
    }

    #[test]
    fn test_parse_doh_response_cname_first() {
        let fixture = r#"{
            "Status": 0,
            "Answer": [
                {"name": "www.example.com.", "type": 5, "TTL": 300, "data": "example.com."},
                {"name": "example.com.", "type": 1, "TTL": 300, "data": "93.184.216.34"}
            ]
        }"#;

        let ip = parse_doh_json_response(fixture);
        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
    }

    #[test]
    fn test_parse_doh_response_empty_answers() {
        let fixture_empty = r#"{"Status": 3, "Answer": []}"#;
        assert_eq!(parse_doh_json_response(fixture_empty), None);

        let fixture_none = r#"{"Status": 3}"#;
        assert_eq!(parse_doh_json_response(fixture_none), None);
    }

    #[test]
    fn test_parse_doh_response_malformed_json() {
        assert_eq!(parse_doh_json_response("not-json"), None);
        assert_eq!(parse_doh_json_response(""), None);
    }

    #[test]
    fn test_parse_doh_response_trimmed_whitespace() {
        let fixture = r#"{"Answer": [{"data": "  1.2.3.4  \n"}]}"#;
        assert_eq!(
            parse_doh_json_response(fixture),
            Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)))
        );
    }

    #[tokio::test]
    async fn test_doh_local_mock_http_server() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local test server");
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let body =
                    r#"{"Answer":[{"name":"mock.local","type":1,"TTL":60,"data":"192.0.2.1"}]}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/dns-json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });

        let client = reqwest::Client::builder().build().unwrap();
        let url = format!("http://127.0.0.1:{}/resolve", port);
        let ip = query_doh_endpoint(&client, &url).await;
        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1))));
    }

    #[tokio::test]
    #[ignore = "requires public WAN access to Cloudflare/Google DoH; run with: cargo test test_live_resolve_hostname_doh -- --ignored"]
    async fn test_live_resolve_hostname_doh() {
        let ip = resolve_hostname_doh("one.one.one.one").await;
        assert!(ip.is_some());
        let ip_addr = ip.unwrap();
        assert!(ip_addr.is_ipv4() || ip_addr.is_ipv6());
    }
}
