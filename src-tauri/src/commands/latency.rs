use serde::Serialize;
use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::sync::{mpsc, Mutex};

const DEFAULT_BATCH_CONCURRENCY: usize = 8;
const DEFAULT_PER_ITEM_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_OVERALL_TIMEOUT: Duration = Duration::from_secs(60);

struct NodeTcpTarget {
    server: String,
    port: u16,
}

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
            let socket_addrs = tokio::net::lookup_host(addr)
                .await
                .map_err(|e| format!("Invalid address or DNS lookup failed: {}", e))?;
            match socket_addrs.into_iter().next() {
                Some(sa) => sa.ip(),
                None => return Err("DNS lookup failed".to_string()),
            }
        }
    };

    connect_tcp(ip, port).await
}

async fn connect_tcp(ip: IpAddr, port: u16) -> Result<u32, String> {
    let start = Instant::now();
    match tokio::time::timeout(
        Duration::from_secs(5),
        tokio::net::TcpStream::connect(SocketAddr::new(ip, port)),
    )
    .await
    {
        Ok(Ok(_)) => Ok(start.elapsed().as_millis() as u32),
        _ => Err("Connection failed".to_string()),
    }
}

async fn test_single_node_tcp_latency(tag: String, target: NodeTcpTarget) -> LatencyResult {
    let NodeTcpTarget { server, port } = target;
    let result = if server.is_empty() {
        Err("No server address".to_string())
    } else {
        let ip = match resolve_hostname_doh(&server).await {
            Some(ip) => Some(ip),
            None => tokio::net::lookup_host((server.as_str(), port))
                .await
                .ok()
                .and_then(|mut addrs| addrs.next().map(|addr| addr.ip())),
        };
        match ip {
            Some(ip) => connect_tcp(ip, port).await,
            None => Err("DNS resolution failed".to_string()),
        }
    };
    match result {
        Ok(ms) => LatencyResult {
            tag,
            latency_ms: Some(ms),
            error: None,
        },
        Err(error) => LatencyResult {
            tag,
            latency_ms: None,
            error: Some(error),
        },
    }
}

async fn run_bounded_batch_with_timeouts<T, F, Fut>(
    items: Vec<(String, T)>,
    concurrency_limit: usize,
    per_item_timeout: Duration,
    overall_timeout: Duration,
    worker: F,
) -> Vec<LatencyResult>
where
    T: Send + 'static,
    F: Fn(String, T) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = LatencyResult> + Send + 'static,
{
    if items.is_empty() {
        return Vec::new();
    }

    let total = items.len();
    let limit = concurrency_limit.max(1).min(total);
    let tags: Vec<String> = items.iter().map(|(tag, _)| tag.clone()).collect();
    let queue = Arc::new(Mutex::new(
        items
            .into_iter()
            .enumerate()
            .map(|(idx, (tag, value))| (idx, tag, value))
            .collect::<VecDeque<_>>(),
    ));
    let worker = Arc::new(worker);
    let (tx, mut rx) = mpsc::channel::<(usize, LatencyResult)>(limit);
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..limit {
        let queue = Arc::clone(&queue);
        let worker = Arc::clone(&worker);
        let tx = tx.clone();
        tasks.spawn(async move {
            loop {
                let item = queue.lock().await.pop_front();
                let (idx, tag, value) = match item {
                    Some(item) => item,
                    None => break,
                };
                let result = match tokio::time::timeout(
                    per_item_timeout,
                    worker(tag.clone(), value),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => LatencyResult {
                        tag,
                        latency_ms: None,
                        error: Some("Latency test timed out".to_string()),
                    },
                };
                if tx.send((idx, result)).await.is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);

    let mut results: Vec<Option<LatencyResult>> = (0..total).map(|_| None).collect();
    let timed_out = tokio::time::timeout(overall_timeout, async {
        while let Some((idx, result)) = rx.recv().await {
            results[idx] = Some(result);
        }
    })
    .await
    .is_err();
    // Dropping the JoinSet cancels any queued or running async tests, even when the IPC call is cancelled.
    drop(tasks);
    tags.into_iter()
        .enumerate()
        .map(|(idx, tag)| {
            results[idx].take().unwrap_or_else(|| LatencyResult {
                tag,
                latency_ms: None,
                error: Some(
                    if timed_out {
                        "Overall latency test timeout exceeded"
                    } else {
                        "Task execution failed"
                    }
                    .to_string(),
                ),
            })
        })
        .collect()
}

async fn run_bounded_batch<T, F, Fut>(items: Vec<(String, T)>, worker: F) -> Vec<LatencyResult>
where
    T: Send + 'static,
    F: Fn(String, T) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = LatencyResult> + Send + 'static,
{
    run_bounded_batch_with_timeouts(
        items,
        DEFAULT_BATCH_CONCURRENCY,
        DEFAULT_PER_ITEM_TIMEOUT,
        DEFAULT_OVERALL_TIMEOUT,
        worker,
    )
    .await
}

/// Tests all nodes in a profile by reading the config and attempting TCP connections.
/// Returns latency results for every node.
#[tauri::command]
pub async fn test_all_nodes(
    app: tauri::AppHandle,
    profile_id: String,
) -> Result<Vec<LatencyResult>, String> {
    let nodes = crate::commands::config::get_profile_outbounds(app, profile_id)?;

    let items = nodes
        .into_iter()
        .map(|node| {
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
            (tag, NodeTcpTarget { server, port })
        })
        .collect();
    Ok(run_bounded_batch(items, test_single_node_tcp_latency).await)
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
    query_clash_proxy_delay(api_port, api_secret, &tag, &url_to_test).await
}

async fn query_clash_proxy_delay(
    api_port: u16,
    api_secret: &str,
    tag: &str,
    test_url: &str,
) -> Result<u32, String> {
    let encoded_tag =
        percent_encoding::utf8_percent_encode(tag, percent_encoding::NON_ALPHANUMERIC).to_string();
    let mut query_url = reqwest::Url::parse(&format!(
        "http://127.0.0.1:{}/proxies/{}/delay",
        api_port, encoded_tag
    ))
    .map_err(|e| format!("Invalid API URL: {}", e))?;
    query_url
        .query_pairs_mut()
        .append_pair("url", test_url)
        .append_pair("timeout", "5000");

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(6))
        .build()
        .map_err(|e| format!("Client creation failed: {}", e))?;
    let resp = client
        .get(query_url)
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
    u32::try_from(delay).map_err(|_| "Invalid proxy delay".to_string())
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
    let settings = state.get_settings();
    let (api_port, api_secret) = controller_for_latency(&settings)?;
    let api_secret = api_secret.to_string();

    let nodes = if let Some(pid) = profile_id {
        crate::commands::config::get_profile_outbounds(app.clone(), pid)?
    } else {
        crate::commands::config::get_config_outbounds(app.clone())?
    };

    let url_to_test =
        test_url.unwrap_or_else(|| "http://cp.cloudflare.com/generate_204".to_string());
    let items = nodes
        .into_iter()
        .map(|node| {
            let tag = node
                .get("tag")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();
            (tag, ())
        })
        .collect();
    Ok(run_bounded_batch(items, move |tag, ()| {
        let url = url_to_test.clone();
        let secret = api_secret.clone();
        async move {
            match query_clash_proxy_delay(api_port, &secret, &tag, &url).await {
                Ok(ms) => LatencyResult {
                    tag,
                    latency_ms: Some(ms),
                    error: None,
                },
                Err(error) => LatencyResult {
                    tag,
                    latency_ms: None,
                    error: Some(error),
                },
            }
        }
    })
    .await)
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
    use std::sync::atomic::{AtomicUsize, Ordering};

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
    async fn test_bounded_batch_limits_in_flight_tests_and_preserves_order() {
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let items: Vec<_> = (0..12)
            .map(|idx| (format!("node-{idx}"), 12 - idx))
            .collect();
        let active_worker = Arc::clone(&active);
        let max_worker = Arc::clone(&max_active);
        let results = run_bounded_batch_with_timeouts(
            items,
            3,
            Duration::from_secs(2),
            Duration::from_secs(5),
            move |tag, delay| {
                let active = Arc::clone(&active_worker);
                let max_active = Arc::clone(&max_worker);
                async move {
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    max_active.fetch_max(now, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(delay * 5)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    LatencyResult {
                        tag,
                        latency_ms: Some(delay as u32),
                        error: None,
                    }
                }
            },
        )
        .await;
        assert!(max_active.load(Ordering::SeqCst) <= 3);
        assert!(max_active.load(Ordering::SeqCst) > 1);
        assert_eq!(results.len(), 12);
        for (idx, result) in results.iter().enumerate() {
            assert_eq!(result.tag, format!("node-{idx}"));
            assert_eq!(result.latency_ms, Some((12 - idx) as u32));
        }
    }

    #[tokio::test]
    async fn test_bounded_batch_reports_item_and_overall_timeouts() {
        let results = run_bounded_batch_with_timeouts(
            vec![("quick".to_string(), 1u64), ("slow".to_string(), 100)],
            1,
            Duration::from_millis(30),
            Duration::from_secs(2),
            |tag, delay| async move {
                tokio::time::sleep(Duration::from_millis(delay)).await;
                LatencyResult {
                    tag,
                    latency_ms: Some(delay as u32),
                    error: None,
                }
            },
        )
        .await;
        assert_eq!(results[0].latency_ms, Some(1));
        assert!(results[1].error.as_deref().unwrap().contains("timed out"));

        let results = run_bounded_batch_with_timeouts(
            (0..5).map(|idx| (format!("node-{idx}"), ())).collect(),
            1,
            Duration::from_secs(1),
            Duration::from_millis(40),
            |tag, ()| async move {
                tokio::time::sleep(Duration::from_secs(1)).await;
                LatencyResult {
                    tag,
                    latency_ms: Some(1),
                    error: None,
                }
            },
        )
        .await;
        assert_eq!(results.len(), 5);
        for (idx, result) in results.iter().enumerate() {
            assert_eq!(result.tag, format!("node-{idx}"));
            assert!(result
                .error
                .as_deref()
                .unwrap()
                .contains("Overall latency test timeout"));
        }
    }

    #[tokio::test]
    async fn test_bounded_batch_empty_and_worker_error() {
        let empty: Vec<(String, ())> = Vec::new();
        assert!(run_bounded_batch(empty, |_, ()| async { unreachable!() })
            .await
            .is_empty());
        let results = run_bounded_batch(vec![("node".to_string(), ())], |tag, ()| async move {
            LatencyResult {
                tag,
                latency_ms: None,
                error: Some("Connection refused".to_string()),
            }
        })
        .await;
        assert_eq!(results[0].error.as_deref(), Some("Connection refused"));
    }

    #[tokio::test]
    async fn test_single_node_tcp_latency_on_loopback() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let result = test_single_node_tcp_latency(
            "local".to_string(),
            NodeTcpTarget {
                server: "127.0.0.1".to_string(),
                port,
            },
        )
        .await;
        assert_eq!(result.tag, "local");
        assert!(result.latency_ms.is_some());
        assert!(result.error.is_none());
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
    async fn test_query_clash_proxy_delay_encodes_url_and_authenticates() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let size = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..size]);
            assert!(request.contains("GET /proxies/myproxy/delay?url=http%3A%2F%2Fexample.com%2Ftest%3Fx%3D1%26y%3D2&timeout=5000"));
            assert!(request
                .to_lowercase()
                .contains("authorization: bearer secret-123"));
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 12\r\nConnection: close\r\n\r\n{\"delay\":42}").await.unwrap();
        });
        let result = query_clash_proxy_delay(
            port,
            "secret-123",
            "myproxy",
            "http://example.com/test?x=1&y=2",
        )
        .await;
        server.await.unwrap();
        assert_eq!(result, Ok(42));
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
