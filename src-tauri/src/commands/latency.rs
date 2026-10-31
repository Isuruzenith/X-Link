use serde::Serialize;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};
use tauri::Manager;

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

fn controller_for_latency(settings: &crate::state::UserSettings) -> Result<(u16, &str), String> {
    if !settings.api_enabled || settings.api_secret.trim().is_empty() {
        return Err("Clash API is disabled; use TCP latency testing instead.".to_string());
    }
    Ok((settings.api_port, &settings.api_secret))
}

/// Tests true proxy URL latency through a specific node tag via sing-box's Clash API.
/// This measures the real end-to-end handshake and HTTP round-trip through the proxy tunnel,
/// distinct from basic TCP connection timing.
#[tauri::command]
pub async fn test_proxy_url_latency(
    app: tauri::AppHandle,
    tag: String,
    test_url: Option<String>,
) -> Result<u32, String> {
    let state = app.state::<crate::state::ProxyState>();
    if !state.is_running() {
        return Err(
            "Proxy core is not running. Please connect to test true URL latency.".to_string(),
        );
    }

    let settings = state.get_settings();
    let (api_port, api_secret) = controller_for_latency(&settings)?;

    let url_to_test =
        test_url.unwrap_or_else(|| "http://cp.cloudflare.com/generate_204".to_string());
    let encoded_tag =
        percent_encoding::utf8_percent_encode(&tag, percent_encoding::NON_ALPHANUMERIC).to_string();
    let query_url = format!(
        "http://127.0.0.1:{}/proxies/{}/delay?url={}&timeout=5000",
        api_port, encoded_tag, url_to_test
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .map_err(|e| format!("Client creation failed: {}", e))?;

    let resp = client
        .get(&query_url)
        .header("Authorization", format!("Bearer {}", api_secret))
        .send()
        .await
        .map_err(|e| format!("Proxy delay query failed: {}", e))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Proxy URL test failed (HTTP {}): {}", status, body));
    }

    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    let delay = val.get("delay").and_then(|d| d.as_u64()).ok_or_else(|| {
        val.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error from proxy core")
            .to_string()
    })?;

    Ok(delay as u32)
}

/// Tests true proxy URL latency for all nodes via sing-box Clash API when running.
#[tauri::command]
pub async fn test_all_nodes_url_latency(
    app: tauri::AppHandle,
    profile_id: Option<String>,
    test_url: Option<String>,
) -> Result<Vec<LatencyResult>, String> {
    let state = app.state::<crate::state::ProxyState>();
    if !state.is_running() {
        return Err(
            "Proxy core is not running. Please connect to test proxy URL latency.".to_string(),
        );
    }
    controller_for_latency(&state.get_settings())?;

    let nodes = if let Some(pid) = profile_id {
        crate::commands::config::get_profile_outbounds(app.clone(), pid)?
    } else {
        crate::commands::config::get_config_outbounds(app.clone())?
    };

    let url_to_test =
        test_url.unwrap_or_else(|| "http://cp.cloudflare.com/generate_204".to_string());
    let mut handles = Vec::new();

    for node in nodes {
        let tag = node
            .get("tag")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown")
            .to_string();
        let app_handle = app.clone();
        let url_clone = url_to_test.clone();
        handles.push(tokio::spawn(async move {
            match test_proxy_url_latency(app_handle, tag.clone(), Some(url_clone)).await {
                Ok(ms) => LatencyResult {
                    tag,
                    latency_ms: Some(ms),
                    error: None,
                },
                Err(e) => LatencyResult {
                    tag,
                    latency_ms: None,
                    error: Some(e),
                },
            }
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(res) = handle.await {
            results.push(res);
        }
    }
    Ok(results)
}

/// Selects the HTTP-capable proxy port to use for URL latency testing or health checks.
/// When `use_separate_ports` is true, the dedicated HTTP listener port (`http_port`) is used.
/// When `use_separate_ports` is false, the unified mixed listener port (`mixed_port`) is used.
/// Returns an error if the selected port is absent or invalid (0).
pub fn select_active_proxy_port(
    use_separate_ports: bool,
    http_port: Option<u16>,
    mixed_port: Option<u16>,
) -> Result<u16, String> {
    let (port_opt, mode_name) = if use_separate_ports {
        (http_port, "HTTP")
    } else {
        (mixed_port, "mixed")
    };

    match port_opt {
        Some(0) => Err(format!(
            "Configured {} proxy port is invalid (0)",
            mode_name
        )),
        Some(port) => Ok(port),
        None => Err(format!("Configured {} proxy port is missing", mode_name)),
    }
}

/// Convenience helper to select the HTTP-capable proxy port directly from `UserSettings`.
pub fn active_proxy_http_port_from_settings(
    settings: &crate::state::UserSettings,
) -> Result<u16, String> {
    select_active_proxy_port(
        settings.use_separate_ports,
        Some(settings.http_port),
        Some(settings.mixed_port),
    )
}

/// Executes an end-to-end HTTP request through a local HTTP-capable proxy listener.
/// Accepts any 2xx response status or 204 No Content.
pub async fn execute_active_proxy_url_test(
    proxy_port: u16,
    url: &str,
    timeout: Duration,
) -> Result<u32, String> {
    let proxy = reqwest::Proxy::all(format!("http://127.0.0.1:{}", proxy_port))
        .map_err(|e| format!("Failed to configure proxy: {}", e))?;
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(timeout)
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let start = Instant::now();
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    if !resp.status().is_success() && resp.status().as_u16() != 204 {
        return Err(format!("Proxy responded with status {}", resp.status()));
    }
    Ok(start.elapsed().as_millis() as u32)
}

/// Tests end-to-end connectivity and HTTP latency through the currently active proxy tunnel
/// via the local HTTP-capable listener (HTTP port in separate-port mode, mixed port otherwise).
#[tauri::command]
pub async fn test_active_proxy_url(
    app: tauri::AppHandle,
    test_url: Option<String>,
) -> Result<u32, String> {
    let state = app.state::<crate::state::ProxyState>();
    if !state.is_running() {
        return Err("Proxy core is not running.".to_string());
    }
    let settings = state.get_settings();
    let proxy_port = active_proxy_http_port_from_settings(&settings)?;
    let url = test_url.unwrap_or_else(|| "http://cp.cloudflare.com/generate_204".to_string());

    execute_active_proxy_url_test(proxy_port, &url, Duration::from_secs(6)).await
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

    #[test]
    fn test_controller_for_latency_requires_enabled_authenticated_api() {
        let mut settings = crate::state::UserSettings::default();
        assert!(controller_for_latency(&settings).is_err());

        settings.api_enabled = true;
        settings.api_secret.clear();
        assert!(controller_for_latency(&settings).is_err());

        settings.api_secret = "token".to_string();
        assert_eq!(controller_for_latency(&settings), Ok((9090, "token")));
    }

    #[test]
    fn test_select_active_proxy_port_modes() {
        // Mixed mode (separate ports false) -> mixed_port
        let port = select_active_proxy_port(false, Some(7890), Some(7892));
        assert_eq!(port, Ok(7892));

        // Separate ports mode (separate ports true) -> http_port
        let port = select_active_proxy_port(true, Some(7890), Some(7892));
        assert_eq!(port, Ok(7890));
    }

    #[test]
    fn test_select_active_proxy_port_absent_and_ignored() {
        // Mixed mode requires mixed_port; http_port can be absent
        let port = select_active_proxy_port(false, None, Some(7892));
        assert_eq!(port, Ok(7892));

        let port_err = select_active_proxy_port(false, Some(7890), None);
        assert!(port_err.is_err());
        assert!(port_err
            .unwrap_err()
            .contains("mixed proxy port is missing"));

        // Separate ports mode requires http_port; mixed_port can be absent
        let port = select_active_proxy_port(true, Some(7890), None);
        assert_eq!(port, Ok(7890));

        let port_err = select_active_proxy_port(true, None, Some(7892));
        assert!(port_err.is_err());
        assert!(port_err.unwrap_err().contains("HTTP proxy port is missing"));
    }

    #[test]
    fn test_select_active_proxy_port_invalid_zero() {
        let err_mixed = select_active_proxy_port(false, Some(7890), Some(0));
        assert!(err_mixed.is_err());
        assert!(err_mixed
            .unwrap_err()
            .contains("mixed proxy port is invalid (0)"));

        let err_http = select_active_proxy_port(true, Some(0), Some(7892));
        assert!(err_http.is_err());
        assert!(err_http
            .unwrap_err()
            .contains("HTTP proxy port is invalid (0)"));
    }

    #[test]
    fn test_active_proxy_http_port_from_settings() {
        let mut settings = crate::state::UserSettings::default();
        // Default settings has use_separate_ports = false, mixed_port = 7892, http_port = 7890
        assert_eq!(active_proxy_http_port_from_settings(&settings), Ok(7892));

        settings.use_separate_ports = true;
        assert_eq!(active_proxy_http_port_from_settings(&settings), Ok(7890));

        settings.http_port = 0;
        assert!(active_proxy_http_port_from_settings(&settings).is_err());

        settings.use_separate_ports = false;
        settings.mixed_port = 0;
        assert!(active_proxy_http_port_from_settings(&settings).is_err());
    }

    #[tokio::test]
    async fn test_execute_active_proxy_url_test_mock_success() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local test proxy");
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let response =
                    "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });

        let latency = execute_active_proxy_url_test(
            port,
            "http://example.com/generate_204",
            Duration::from_secs(2),
        )
        .await;
        assert!(latency.is_ok());
    }

    #[tokio::test]
    async fn test_execute_active_proxy_url_test_mock_error_status() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local test proxy");
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let response =
                    "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });

        let latency = execute_active_proxy_url_test(
            port,
            "http://example.com/generate_204",
            Duration::from_secs(2),
        )
        .await;
        assert!(latency.is_err());
        assert!(latency
            .unwrap_err()
            .contains("Proxy responded with status 502"));
    }

    #[tokio::test]
    async fn test_execute_active_proxy_url_test_unreachable_port() {
        let port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };

        let result = execute_active_proxy_url_test(
            port,
            "http://example.com/generate_204",
            Duration::from_millis(500),
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Request failed"));
    }
}
