/// Builds the list of proxy bypass patterns accepted by system proxy APIs (WinINet on Windows, etc.).
///
/// On Windows WinINet, wildcard asterisks are used instead of CIDR notation.
/// Private RFC1918 subnets are covered:
/// - 10.0.0.0/8 via `10.*`
/// - 172.16.0.0/12 via `172.16.*` through `172.31.*` (all 16 /16 ranges)
/// - 192.168.0.0/16 via `192.168.*`
///
/// Also includes localhost and loopback entries (`localhost`, `127.*`, and `<local>` on Windows).
#[allow(clippy::vec_init_then_push)]
pub fn build_system_proxy_bypass_entries() -> Vec<String> {
    let mut entries = Vec::new();

    #[cfg(target_os = "windows")]
    entries.push("<local>".to_string());

    entries.push("localhost".to_string());
    entries.push("127.*".to_string());
    entries.push("10.*".to_string());

    for octet in 16..=31 {
        entries.push(format!("172.{}.*", octet));
    }

    entries.push("192.168.*".to_string());
    entries
}

/// Returns the semicolon-delimited system proxy bypass string.
pub fn build_system_proxy_bypass() -> String {
    build_system_proxy_bypass_entries().join(";")
}

pub fn enable_system_proxy(host: &str, port: u16) -> Result<(), String> {
    let bypass = build_system_proxy_bypass();
    let sys = sysproxy::Sysproxy {
        enable: true,
        host: host.to_string(),
        port,
        bypass,
    };
    sys.set_system_proxy()
        .map_err(|e| format!("Failed to set system proxy: {}", e))
}

pub fn disable_system_proxy() -> Result<(), String> {
    if let Ok(mut sys) = sysproxy::Sysproxy::get_system_proxy() {
        sys.enable = false;
        sys.set_system_proxy()
            .map_err(|e| format!("Failed to clear system proxy: {}", e))
    } else {
        // Fallback: set a default disabled proxy configuration
        let sys = sysproxy::Sysproxy {
            enable: false,
            host: "127.0.0.1".to_string(),
            port: 7892,
            bypass: "".to_string(),
        };
        sys.set_system_proxy()
            .map_err(|e| format!("Failed to clear system proxy (fallback): {}", e))
    }
}

pub fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        is_elevated::is_elevated()
    }
    #[cfg(not(target_os = "windows"))]
    {
        unsafe { libc::getuid() == 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_proxy_bypass_contains_boundaries_and_all_intermediate() {
        let entries = build_system_proxy_bypass_entries();

        // Boundaries 172.16.* and 172.31.*
        assert!(
            entries.contains(&"172.16.*".to_string()),
            "Should contain boundary 172.16.*"
        );
        assert!(
            entries.contains(&"172.31.*".to_string()),
            "Should contain boundary 172.31.*"
        );

        // All 16 intermediate /16 ranges
        for octet in 16..=31 {
            let pattern = format!("172.{}.*", octet);
            assert!(
                entries.contains(&pattern),
                "Missing RFC1918 range {}",
                pattern
            );
        }
    }

    #[test]
    fn test_system_proxy_bypass_contains_original_and_rfc1918_entries() {
        let entries = build_system_proxy_bypass_entries();
        assert!(entries.contains(&"localhost".to_string()));
        assert!(entries.contains(&"127.*".to_string()));
        assert!(entries.contains(&"10.*".to_string()));
        assert!(entries.contains(&"192.168.*".to_string()));

        #[cfg(target_os = "windows")]
        assert!(entries.contains(&"<local>".to_string()));

        let bypass_str = build_system_proxy_bypass();
        assert!(bypass_str.contains("localhost"));
        assert!(bypass_str.contains("127.*"));
        assert!(bypass_str.contains("10.*"));
        assert!(bypass_str.contains("172.16.*"));
        assert!(bypass_str.contains("172.31.*"));
        assert!(bypass_str.contains("192.168.*"));
    }

    #[test]
    fn test_system_proxy_bypass_contains_no_unsupported_cidr() {
        let bypass_str = build_system_proxy_bypass();
        // WinINet does not accept CIDR notation like /8, /12, /16
        assert!(
            !bypass_str.contains('/'),
            "Bypass list must not contain CIDR slashes"
        );
    }

    /// Evaluates if a given hostname/IP matches simple wildcard patterns like `10.*` or `172.16.*`.
    fn matches_wildcard_pattern(target: &str, pattern: &str) -> bool {
        if pattern == "<local>" {
            return !target.contains('.');
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            target.starts_with(prefix)
        } else {
            target.eq_ignore_ascii_case(pattern)
        }
    }

    #[test]
    fn test_representative_private_addresses_match_bypass() {
        let entries = build_system_proxy_bypass_entries();

        let private_ips = [
            "127.0.0.1",
            "localhost",
            "10.0.0.1",
            "10.254.12.3",
            "172.16.0.1",
            "172.20.1.100",
            "172.31.255.254",
            "192.168.0.1",
            "192.168.100.50",
        ];

        for ip in private_ips {
            let matched = entries
                .iter()
                .any(|pattern| matches_wildcard_pattern(ip, pattern));
            assert!(
                matched,
                "Expected private IP {} to match bypass entries",
                ip
            );
        }

        let public_ips = [
            "1.1.1.1",
            "8.8.8.8",
            "11.0.0.1",
            "172.15.0.1",
            "172.32.0.1",
            "192.169.1.1",
        ];

        for ip in public_ips {
            let matched = entries
                .iter()
                .any(|pattern| matches_wildcard_pattern(ip, pattern));
            assert!(
                !matched,
                "Expected public IP {} NOT to match bypass entries",
                ip
            );
        }
    }
}
