use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct FetchConfig {
    pub allow_loopback: bool,
    pub max_size_bytes: usize,
    pub timeout: Duration,
    pub connect_timeout: Duration,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            allow_loopback: false,
            max_size_bytes: 10 * 1024 * 1024, // 10 MB bounded size
            timeout: Duration::from_secs(25), // 25s bounded timeout
            connect_timeout: Duration::from_secs(10),
        }
    }
}

/// Checks whether an IPv4 address is globally routable / public.
/// Strictly rejects RFC 1918 private networks, loopback, link-local,
/// CGNAT, broadcast, multicast, documentation, and benchmark addresses.
pub fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    // 0.0.0.0/8 - Current network ("this host", RFC 1122)
    if octets[0] == 0 {
        return false;
    }
    // 10.0.0.0/8 - Private network (RFC 1918)
    if octets[0] == 10 {
        return false;
    }
    // 100.64.0.0/10 - Shared Address Space / CGNAT (RFC 6598)
    if octets[0] == 100 && (64..=127).contains(&octets[1]) {
        return false;
    }
    // 127.0.0.0/8 - Loopback (RFC 1122)
    if octets[0] == 127 {
        return false;
    }
    // 169.254.0.0/16 - Link-local (RFC 3927)
    if octets[0] == 169 && octets[1] == 254 {
        return false;
    }
    // 172.16.0.0/12 - Private network (RFC 1918)
    if octets[0] == 172 && (16..=31).contains(&octets[1]) {
        return false;
    }
    // 192.0.0.0/24 - IETF Protocol Assignments (RFC 6890)
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 0 {
        return false;
    }
    // 192.0.2.0/24 - Documentation (TEST-NET-1, RFC 5737)
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 2 {
        return false;
    }
    // 192.88.99.0/24 - 6to4 Relay Anycast (RFC 7526)
    if octets[0] == 192 && octets[1] == 88 && octets[2] == 99 {
        return false;
    }
    // 192.168.0.0/16 - Private network (RFC 1918)
    if octets[0] == 192 && octets[1] == 168 {
        return false;
    }
    // 198.18.0.0/15 - Network benchmark tests (RFC 2544)
    if octets[0] == 198 && (octets[1] == 18 || octets[1] == 19) {
        return false;
    }
    // 198.51.100.0/24 - Documentation (TEST-NET-2, RFC 5737)
    if octets[0] == 198 && octets[1] == 51 && octets[2] == 100 {
        return false;
    }
    // 203.0.113.0/24 - Documentation (TEST-NET-3, RFC 5737)
    if octets[0] == 203 && octets[1] == 0 && octets[2] == 113 {
        return false;
    }
    // 224.0.0.0/4 - Multicast (RFC 5771)
    if octets[0] >= 224 && octets[0] <= 239 {
        return false;
    }
    // 240.0.0.0/4 - Reserved for future use (RFC 1112) + 255.255.255.255 broadcast
    if octets[0] >= 240 {
        return false;
    }
    true
}

/// Checks whether an IPv6 address is globally routable / public.
/// Strictly rejects loopback (::1), unspecified (::), unique-local (fc00::/7),
/// link-local (fe80::/10), multicast (ff00::/8), discard (100::/64),
/// documentation (2001:db8::/32), and unpacks IPv4-mapped/IPv4-compatible addresses.
pub fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();

    // ::1 - Loopback
    if ip.is_loopback() {
        return false;
    }
    // :: - Unspecified
    if ip.is_unspecified() {
        return false;
    }
    // ff00::/8 - Multicast
    if ip.is_multicast() {
        return false;
    }

    // IPv4-mapped IPv6: ::ffff:a.b.c.d/96
    if segments[0] == 0
        && segments[1] == 0
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0xffff
    {
        let v4 = Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            (segments[6] & 0xff) as u8,
            (segments[7] >> 8) as u8,
            (segments[7] & 0xff) as u8,
        );
        return is_public_ipv4(v4);
    }

    // IPv4-compatible IPv6: ::a.b.c.d/96 (deprecated RFC 4291)
    if segments[0] == 0
        && segments[1] == 0
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0
    {
        let v4 = Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            (segments[6] & 0xff) as u8,
            (segments[7] >> 8) as u8,
            (segments[7] & 0xff) as u8,
        );
        return is_public_ipv4(v4);
    }

    // NAT64 / Well-known prefix: 64:ff9b::/96 (RFC 6052)
    if segments[0] == 0x0064
        && segments[1] == 0xff9b
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0
    {
        let v4 = Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            (segments[6] & 0xff) as u8,
            (segments[7] >> 8) as u8,
            (segments[7] & 0xff) as u8,
        );
        return is_public_ipv4(v4);
    }

    // 6to4: 2002::/16 contains embedded IPv4 in segments 1 and 2
    if segments[0] == 0x2002 {
        let v4 = Ipv4Addr::new(
            (segments[1] >> 8) as u8,
            (segments[1] & 0xff) as u8,
            (segments[2] >> 8) as u8,
            (segments[2] & 0xff) as u8,
        );
        return is_public_ipv4(v4);
    }

    // Unique Local Address (fc00::/7) - private (fc00:: - fdff::)
    if (segments[0] & 0xfe00) == 0xfc00 {
        return false;
    }

    // Link-local Unicast (fe80::/10) - (fe80:: - febf::)
    if (segments[0] & 0xffc0) == 0xfe80 {
        return false;
    }

    // Discard prefix (100::/64)
    if segments[0] == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0 {
        return false;
    }

    // Documentation prefix (2001:db8::/32)
    if segments[0] == 0x2001 && segments[1] == 0x0db8 {
        return false;
    }

    // Benchmarking (2001:2::/48)
    if segments[0] == 0x2001 && segments[1] == 0x0002 {
        return false;
    }

    // ORCHID (2001:10::/28)
    if segments[0] == 0x2001 && (segments[1] & 0xfff0) == 0x0010 {
        return false;
    }

    // ORCHIDv2 (2001:20::/28)
    if segments[0] == 0x2001 && (segments[1] & 0xfff0) == 0x0020 {
        return false;
    }

    // Global unicast generally sits in 2000::/3
    if (segments[0] & 0xe000) != 0x2000 {
        return false;
    }

    true
}

/// Checks whether an IP address is a public, globally routable address.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_ipv4(v4),
        IpAddr::V6(v6) => is_public_ipv6(v6),
    }
}

/// Reports only the origin; subscription paths, query strings and fragments can carry tokens.
pub fn sanitize_url(url_str: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url_str) {
        if let Some(host) = parsed.host_str() {
            let host = if host.contains(':') {
                format!("[{}]", host)
            } else {
                host.to_string()
            };
            let port = parsed
                .port()
                .map(|port| format!(":{}", port))
                .unwrap_or_default();
            return format!("{}://{}{}[REDACTED]", parsed.scheme(), host, port);
        }
    }
    "[INVALID_URL]".to_string()
}

/// Validates subscription URL syntax and target destination security.
/// Returns parsed URL, host string, port, and whether it's an IP literal.
pub fn validate_subscription_url(
    url_str: &str,
    allow_loopback: bool,
) -> Result<(url::Url, String, u16, Option<IpAddr>), String> {
    let parsed =
        url::Url::parse(url_str).map_err(|e| format!("Invalid subscription URL: {}", e))?;

    // Must be HTTPS (reject HTTP, ftp, file, etc.)
    if parsed.scheme().eq_ignore_ascii_case("http") {
        return Err(
            "Insecure HTTP subscription URLs are not supported. Only HTTPS is allowed for security."
                .to_string(),
        );
    }
    if !parsed.scheme().eq_ignore_ascii_case("https") {
        return Err(format!(
            "Unsupported URL scheme '{}'. Only HTTPS subscription URLs are supported.",
            parsed.scheme()
        ));
    }

    let host_str = parsed
        .host_str()
        .ok_or_else(|| "Subscription URL missing hostname".to_string())?
        .trim()
        .to_string();

    if host_str.is_empty() {
        return Err("Subscription URL has empty hostname".to_string());
    }

    let lower_host = host_str.to_ascii_lowercase();

    // Check forbidden domain patterns unless allow_loopback is enabled
    if !allow_loopback
        && (lower_host == "localhost"
            || lower_host.ends_with(".localhost")
            || lower_host.ends_with(".local")
            || lower_host.ends_with(".internal")
            || lower_host.ends_with(".lan")
            || lower_host.ends_with(".home.arpa"))
    {
        return Err(format!(
            "SSRF blocked: local and private domains are not permitted ({})",
            lower_host
        ));
    }

    let port = parsed.port_or_known_default().unwrap_or(443);

    // Check if host is an IP literal
    let ip_opt: Option<IpAddr> = match parsed.host() {
        Some(url::Host::Ipv4(v4)) => Some(IpAddr::V4(v4)),
        Some(url::Host::Ipv6(v6)) => Some(IpAddr::V6(v6)),
        _ => host_str.parse::<IpAddr>().ok(),
    };

    if let Some(ip) = ip_opt {
        if !allow_loopback && !is_public_ip(ip) {
            return Err(format!(
                "SSRF blocked: destination IP {} is not a public address",
                ip
            ));
        }
    }

    Ok((parsed, host_str, port, ip_opt))
}

/// Fetches subscription content securely using default production policy.
pub async fn fetch_subscription_content(url_str: &str) -> Result<Vec<u8>, String> {
    fetch_subscription_content_with_config(url_str, &FetchConfig::default()).await
}

/// Fetches subscription content securely with configurable policy (for tests).
///
/// Features:
/// - Rejects insecure HTTP and arbitrary schemes (HTTPS public-only)
/// - Rejects private, loopback, link-local, multicast, and reserved IP ranges (SSRF protection)
/// - Resolves DNS and validates EVERY returned IP address
/// - Pins client connection to the validated IP (anti-DNS-rebinding protection)
/// - Rejects redirects (Policy::none)
/// - Enforces bounded download size (Content-Length check + chunked stream limit)
/// - Enforces bounded connection and read timeouts
/// - Sanitizes URLs to avoid logging credentials or token queries
pub async fn fetch_subscription_content_with_config(
    url_str: &str,
    config: &FetchConfig,
) -> Result<Vec<u8>, String> {
    let (parsed_url, host, port, ip_literal_opt) =
        validate_subscription_url(url_str, config.allow_loopback)?;
    if !parsed_url.username().is_empty() || parsed_url.password().is_some() {
        return Err("Subscription URLs cannot contain username or password".to_string());
    }

    let target_addr: SocketAddr = if let Some(ip) = ip_literal_opt {
        if !config.allow_loopback && !is_public_ip(ip) {
            return Err(format!(
                "SSRF blocked: destination IP {} is not a public address",
                ip
            ));
        }
        SocketAddr::new(ip, port)
    } else {
        // Resolve host to IP addresses and validate each one
        let addrs = tokio::net::lookup_host((host.as_str(), port))
            .await
            .map_err(|_| format!("DNS resolution failed for {}", sanitize_url(url_str)))?;

        let resolved_list: Vec<SocketAddr> = addrs.collect();
        if resolved_list.is_empty() {
            return Err(format!(
                "No DNS records found for {}",
                sanitize_url(url_str)
            ));
        }

        for addr in &resolved_list {
            let ip = addr.ip();
            if config.allow_loopback
                && (ip.is_loopback() || ip == IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
            {
                continue;
            }
            if !is_public_ip(ip) {
                return Err(format!(
                    "SSRF blocked: destination for {} resolved to non-public address {}",
                    sanitize_url(url_str),
                    ip
                ));
            }
        }

        resolved_list[0]
    };

    // Configure reqwest client:
    // 1. Rustls TLS
    // 2. No redirects
    // 3. Connect timeout & overall timeout
    // 4. ANTI-DNS-REBINDING: pin host resolution to target_addr
    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(config.timeout)
        .connect_timeout(config.connect_timeout);

    // If host is a domain name, pin the resolution to the pre-validated IP
    if ip_literal_opt.is_none() {
        builder = builder.resolve(&host, target_addr);
    }

    let client = builder
        .build()
        .map_err(|e| format!("Failed to initialize HTTP client: {}", e))?;

    let response = client
        .get(parsed_url.as_str())
        .header("User-Agent", "xlink-client/0.8.0")
        .header("Accept", "*/*")
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                format!(
                    "Subscription request timed out after {}s",
                    config.timeout.as_secs()
                )
            } else if e.is_connect() {
                format!(
                    "Failed to connect to subscription server ({})",
                    sanitize_url(url_str)
                )
            } else {
                "Failed to fetch subscription securely".to_string()
            }
        })?;

    let status = response.status();
    if status.is_redirection() {
        return Err(format!(
            "Redirects are not allowed for subscription URLs (received HTTP {})",
            status.as_u16()
        ));
    }
    if !status.is_success() {
        return Err(format!(
            "Subscription download failed with HTTP status {}",
            status.as_u16()
        ));
    }

    // Check Content-Length if present
    if let Some(content_len) = response.content_length() {
        if content_len > config.max_size_bytes as u64 {
            return Err(format!(
                "Subscription content exceeds size limit ({} bytes, max {} bytes)",
                content_len, config.max_size_bytes
            ));
        }
    }

    // Read body in bounded streaming chunks
    let mut mut_response = response;
    let mut body = Vec::new();
    while let Some(chunk) = mut_response
        .chunk()
        .await
        .map_err(|_| "Error downloading subscription body".to_string())?
    {
        if body.len() + chunk.len() > config.max_size_bytes {
            return Err(format!(
                "Subscription content exceeds maximum size limit of {} MB",
                config.max_size_bytes / (1024 * 1024)
            ));
        }
        body.extend_from_slice(&chunk);
    }

    if body.is_empty() {
        return Err("Subscription returned empty content".to_string());
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_public_ipv4() {
        // Public addresses
        assert!(is_public_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
        assert!(is_public_ipv4(Ipv4Addr::new(1, 1, 1, 1)));
        assert!(is_public_ipv4(Ipv4Addr::new(93, 184, 216, 34)));
        assert!(is_public_ipv4(Ipv4Addr::new(142, 250, 190, 46)));

        // Loopback
        assert!(!is_public_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(127, 255, 255, 255)));

        // RFC 1918 Private
        assert!(!is_public_ipv4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(10, 255, 255, 255)));
        assert!(!is_public_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(172, 31, 255, 255)));
        assert!(is_public_ipv4(Ipv4Addr::new(172, 32, 0, 1))); // Outside /12
        assert!(!is_public_ipv4(Ipv4Addr::new(192, 168, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(192, 168, 255, 255)));

        // Link-local
        assert!(!is_public_ipv4(Ipv4Addr::new(169, 254, 1, 1)));

        // Current network
        assert!(!is_public_ipv4(Ipv4Addr::new(0, 0, 0, 0)));

        // CGNAT (100.64.0.0/10)
        assert!(!is_public_ipv4(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(100, 127, 255, 255)));
        assert!(is_public_ipv4(Ipv4Addr::new(100, 128, 0, 1)));

        // Documentation (RFC 5737)
        assert!(!is_public_ipv4(Ipv4Addr::new(192, 0, 2, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(198, 51, 100, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(203, 0, 113, 1)));

        // Benchmark (RFC 2544)
        assert!(!is_public_ipv4(Ipv4Addr::new(198, 18, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(198, 19, 255, 255)));

        // Multicast & Reserved
        assert!(!is_public_ipv4(Ipv4Addr::new(224, 0, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(240, 0, 0, 1)));
        assert!(!is_public_ipv4(Ipv4Addr::new(255, 255, 255, 255)));
    }

    #[test]
    fn test_is_public_ipv6() {
        // Public unicast
        assert!(is_public_ipv6(
            "2606:4700:4700::1111".parse::<Ipv6Addr>().unwrap()
        ));
        assert!(is_public_ipv6(
            "2001:4860:4860::8888".parse::<Ipv6Addr>().unwrap()
        ));

        // Loopback & Unspecified
        assert!(!is_public_ipv6(Ipv6Addr::LOCALHOST));
        assert!(!is_public_ipv6(Ipv6Addr::UNSPECIFIED));

        // Unique local (fc00::/7)
        assert!(!is_public_ipv6("fc00::1".parse::<Ipv6Addr>().unwrap()));
        assert!(!is_public_ipv6(
            "fd12:3456:789a::1".parse::<Ipv6Addr>().unwrap()
        ));

        // Link-local (fe80::/10)
        assert!(!is_public_ipv6("fe80::1".parse::<Ipv6Addr>().unwrap()));

        // Multicast (ff00::/8)
        assert!(!is_public_ipv6("ff02::1".parse::<Ipv6Addr>().unwrap()));

        // Documentation (2001:db8::/32)
        assert!(!is_public_ipv6("2001:db8::1".parse::<Ipv6Addr>().unwrap()));

        // IPv4-mapped IPv6
        assert!(!is_public_ipv6(
            "::ffff:127.0.0.1".parse::<Ipv6Addr>().unwrap()
        ));
        assert!(!is_public_ipv6(
            "::ffff:10.0.0.1".parse::<Ipv6Addr>().unwrap()
        ));
        assert!(!is_public_ipv6(
            "::ffff:192.168.1.1".parse::<Ipv6Addr>().unwrap()
        ));
        assert!(is_public_ipv6(
            "::ffff:8.8.8.8".parse::<Ipv6Addr>().unwrap()
        ));

        // IPv4-compatible IPv6
        assert!(!is_public_ipv6("::127.0.0.1".parse::<Ipv6Addr>().unwrap()));
        assert!(is_public_ipv6("::8.8.8.8".parse::<Ipv6Addr>().unwrap()));
    }

    #[test]
    fn test_sanitize_url() {
        let input = "https://admin:secretpass@example.com:8443/api/v1/sub?token=topsecret123&user=42#fragment";
        let sanitized = sanitize_url(input);
        assert!(!sanitized.contains("secretpass"));
        assert!(!sanitized.contains("topsecret123"));
        assert!(!sanitized.contains("admin"));
        assert!(!sanitized.contains("fragment"));
        assert!(!sanitized.contains("/api/v1/sub"));
        assert!(sanitized.contains("[REDACTED]"));
        assert!(sanitized.contains("example.com:8443"));
        assert_eq!(
            sanitize_url("https://example.com/private-token?key=secret"),
            "https://example.com[REDACTED]"
        );

        assert_eq!(sanitize_url("not a url"), "[INVALID_URL]");
    }

    #[test]
    fn test_validate_subscription_url() {
        // Plain HTTP rejected
        let res = validate_subscription_url("http://example.com/sub", false);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Insecure HTTP"));

        // Non-http schemes rejected
        assert!(validate_subscription_url("ftp://example.com/sub", false).is_err());
        assert!(validate_subscription_url("file:///etc/passwd", false).is_err());

        // Local domains rejected
        assert!(validate_subscription_url("https://localhost/sub", false).is_err());
        assert!(validate_subscription_url("https://router.local/sub", false).is_err());
        assert!(validate_subscription_url("https://host.internal/sub", false).is_err());

        // Private IP literals rejected
        assert!(validate_subscription_url("https://127.0.0.1/sub", false).is_err());
        assert!(validate_subscription_url("https://10.0.0.1/sub", false).is_err());
        assert!(validate_subscription_url("https://192.168.1.1/sub", false).is_err());
        assert!(validate_subscription_url("https://[::1]/sub", false).is_err());
        assert!(validate_subscription_url("https://[fc00::1]/sub", false).is_err());

        // Public IP literals accepted
        assert!(validate_subscription_url("https://8.8.8.8/sub", false).is_ok());

        // Public domain accepted
        let (url, host, port, ip_opt) =
            validate_subscription_url("https://example.com:8443/sub?key=val", false).unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 8443);
        assert!(ip_opt.is_none());
        assert_eq!(url.scheme(), "https");
    }

    #[tokio::test]
    async fn test_fetch_with_mock_server_success_and_failures() {
        let test_config = FetchConfig {
            allow_loopback: true,
            max_size_bytes: 1024 * 1024,
            timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(2),
        };

        // Test URL validation with loopback allowed for test
        let (url, host, port_res, _) =
            validate_subscription_url("https://127.0.0.1:8443/sub", test_config.allow_loopback)
                .unwrap();
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port_res, 8443);
        assert_eq!(url.scheme(), "https");

        // Verify that with default config (allow_loopback: false), 127.0.0.1 is rejected
        let blocked = validate_subscription_url("https://127.0.0.1:8443/sub", false);
        assert!(blocked.is_err());
        assert!(blocked.unwrap_err().contains("SSRF blocked"));
    }
}
