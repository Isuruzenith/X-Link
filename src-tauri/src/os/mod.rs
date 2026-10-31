use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedSystemProxy {
    pub enable: bool,
    pub host: String,
    pub port: u16,
    pub bypass: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedProxy {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviousProxyState {
    /// Proxy was disabled (or not configured) before X-Link enabled it.
    CapturedOff(Option<SavedSystemProxy>),
    /// Proxy was enabled with specific settings before X-Link enabled it.
    CapturedOn(SavedSystemProxy),
}

impl PreviousProxyState {
    pub fn as_saved_proxy(&self) -> Option<&SavedSystemProxy> {
        match self {
            PreviousProxyState::CapturedOff(opt) => opt.as_ref(),
            PreviousProxyState::CapturedOn(saved) => Some(saved),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedSnapshot {
    pub previous: PreviousProxyState,
    pub applied: Option<AppliedProxy>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreTarget {
    /// Restore a specific previous proxy configuration.
    RestoreConfig(SavedSystemProxy),
    /// Clear system proxy (disable it).
    ClearProxy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisableAction {
    /// Not owned by X-Link: do nothing, do not touch OS proxy.
    NotOwned,
    /// Third party modified the proxy: avoid overriding, release ownership.
    ExternalModificationAvoided,
    /// Restore target settings to OS.
    Apply(RestoreTarget),
}

/// Detects whether current OS proxy settings have been modified by a third party.
pub fn is_externally_modified(current: &SavedSystemProxy, applied: &AppliedProxy) -> bool {
    !current.enable
        || !current.host.eq_ignore_ascii_case(&applied.host)
        || current.port != applied.port
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ProxyStateManager {
    owned: Option<OwnedSnapshot>,
}

impl ProxyStateManager {
    fn record_before_enable<F>(&mut self, capture: F)
    where
        F: FnOnce() -> Option<SavedSystemProxy>,
    {
        if self.owned.is_none() {
            let previous = match capture() {
                Some(saved) if saved.enable => PreviousProxyState::CapturedOn(saved),
                Some(saved) => PreviousProxyState::CapturedOff(Some(saved)),
                None => PreviousProxyState::CapturedOff(None),
            };
            self.owned = Some(OwnedSnapshot {
                previous,
                applied: None,
            });
        }
    }

    fn record_applied(&mut self, host: &str, port: u16) {
        if let Some(ref mut snapshot) = self.owned {
            snapshot.applied = Some(AppliedProxy {
                host: host.to_string(),
                port,
            });
        }
    }

    fn revert_if_never_applied(&mut self) {
        if let Some(ref snapshot) = self.owned {
            if snapshot.applied.is_none() {
                self.owned = None;
            }
        }
    }

    fn has_saved(&self) -> bool {
        self.owned.is_some()
    }

    fn reset(&mut self) {
        self.owned = None;
    }

    #[allow(dead_code)]
    fn pop_for_disable(&mut self) -> Option<Option<SavedSystemProxy>> {
        self.owned.take().map(|snapshot| match snapshot.previous {
            PreviousProxyState::CapturedOn(saved) => Some(saved),
            PreviousProxyState::CapturedOff(saved) => saved,
        })
    }

    fn plan_disable(&self, current_os_proxy: Option<&SavedSystemProxy>) -> DisableAction {
        let snapshot = match &self.owned {
            None => return DisableAction::NotOwned,
            Some(s) => s,
        };

        if let (Some(applied), Some(current)) = (&snapshot.applied, current_os_proxy) {
            if is_externally_modified(current, applied) {
                return DisableAction::ExternalModificationAvoided;
            }
        }

        match &snapshot.previous {
            PreviousProxyState::CapturedOn(prev) => {
                DisableAction::Apply(RestoreTarget::RestoreConfig(prev.clone()))
            }
            PreviousProxyState::CapturedOff(Some(prev)) => {
                DisableAction::Apply(RestoreTarget::RestoreConfig(prev.clone()))
            }
            PreviousProxyState::CapturedOff(None) => {
                DisableAction::Apply(RestoreTarget::ClearProxy)
            }
        }
    }

    fn commit_disable_success(&mut self) {
        self.owned = None;
    }

    fn execute_disable<R, S>(&mut self, read_current: R, mut set_proxy: S) -> Result<(), String>
    where
        R: FnOnce() -> Option<SavedSystemProxy>,
        S: FnMut(RestoreTarget) -> Result<(), String>,
    {
        if self.owned.is_none() {
            return Ok(());
        }

        let current = read_current().ok_or_else(|| {
            "Failed to read current system proxy; retaining previous settings for retry".to_string()
        })?;
        let action = self.plan_disable(Some(&current));
        match action {
            DisableAction::NotOwned => Ok(()),
            DisableAction::ExternalModificationAvoided => {
                self.commit_disable_success();
                Ok(())
            }
            DisableAction::Apply(target) => {
                set_proxy(target)?;
                self.commit_disable_success();
                Ok(())
            }
        }
    }
}

static PROXY_STATE: Mutex<ProxyStateManager> = Mutex::new(ProxyStateManager { owned: None });

fn capture_before_enable<F>(state: &mut ProxyStateManager, capture: F) -> Result<(), String>
where
    F: FnOnce() -> Option<SavedSystemProxy>,
{
    if !state.has_saved() {
        let previous = capture().ok_or_else(|| {
            "Failed to capture current system proxy; refusing to overwrite it".to_string()
        })?;
        state.record_before_enable(|| Some(previous));
    }
    Ok(())
}

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

/// Captures the current active system proxy settings from the OS before modifying them.
pub fn capture_current_system_proxy() -> Option<SavedSystemProxy> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
        use winreg::RegKey;

        const SUB_KEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Internet Settings";
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(cur_var) = hkcu.open_subkey_with_flags(SUB_KEY, KEY_READ) {
            let enable = cur_var.get_value::<u32, _>("ProxyEnable").unwrap_or(0) == 1;
            let server: Result<String, _> = cur_var.get_value("ProxyServer");
            let bypass: String = cur_var.get_value("ProxyOverride").unwrap_or_default();

            if let Ok(server_str) = server {
                let parts: Vec<&str> = server_str.split(':').collect();
                if parts.len() == 2 {
                    if let Ok(port) = parts[1].parse::<u16>() {
                        return Some(SavedSystemProxy {
                            enable,
                            host: parts[0].to_string(),
                            port,
                            bypass,
                        });
                    }
                }
                return Some(SavedSystemProxy {
                    enable,
                    host: server_str,
                    port: 0,
                    bypass,
                });
            } else if enable {
                return Some(SavedSystemProxy {
                    enable,
                    host: String::new(),
                    port: 0,
                    bypass,
                });
            } else {
                return Some(SavedSystemProxy {
                    enable: false,
                    host: String::new(),
                    port: 0,
                    bypass,
                });
            }
        }
    }

    sysproxy::Sysproxy::get_system_proxy()
        .ok()
        .map(|s| SavedSystemProxy {
            enable: s.enable,
            host: s.host,
            port: s.port,
            bypass: s.bypass,
        })
}

pub fn enable_system_proxy(host: &str, port: u16) -> Result<(), String> {
    {
        let mut state = PROXY_STATE
            .lock()
            .map_err(|_| "Failed to acquire proxy state lock".to_string())?;
        capture_before_enable(&mut state, capture_current_system_proxy)?;
    }

    let bypass = build_system_proxy_bypass();
    let sys = sysproxy::Sysproxy {
        enable: true,
        host: host.to_string(),
        port,
        bypass,
    };
    match sys.set_system_proxy() {
        Ok(()) => {
            if let Ok(mut state) = PROXY_STATE.lock() {
                state.record_applied(host, port);
            }
            Ok(())
        }
        Err(e) => {
            if let Ok(mut state) = PROXY_STATE.lock() {
                state.revert_if_never_applied();
            }
            Err(format!("Failed to set system proxy: {}", e))
        }
    }
}

fn apply_restore_target(target: RestoreTarget) -> Result<(), String> {
    match target {
        RestoreTarget::RestoreConfig(prev) => {
            let sys = sysproxy::Sysproxy {
                enable: prev.enable,
                host: prev.host,
                port: prev.port,
                bypass: prev.bypass,
            };
            sys.set_system_proxy()
                .map_err(|e| format!("Failed to restore previous system proxy: {}", e))
        }
        RestoreTarget::ClearProxy => {
            if let Ok(mut sys) = sysproxy::Sysproxy::get_system_proxy() {
                sys.enable = false;
                sys.set_system_proxy()
                    .map_err(|e| format!("Failed to clear system proxy: {}", e))
            } else {
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
    }
}

pub fn disable_system_proxy() -> Result<(), String> {
    let mut state = match PROXY_STATE.lock() {
        Ok(guard) => guard,
        Err(_) => return Err("Failed to acquire proxy state lock".to_string()),
    };

    state.execute_disable(capture_current_system_proxy, apply_restore_target)
}

pub fn has_saved_system_proxy() -> bool {
    PROXY_STATE.lock().is_ok_and(|s| s.has_saved())
}

pub fn is_system_proxy_owned() -> bool {
    if let Ok(state) = PROXY_STATE.lock() {
        if let Some(ref snapshot) = state.owned {
            return snapshot.applied.is_some();
        }
    }
    false
}

pub fn reset_saved_system_proxy() {
    if let Ok(mut s) = PROXY_STATE.lock() {
        s.reset();
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

    #[test]
    fn test_proxy_state_manager_lifecycle() {
        let mut manager = ProxyStateManager::default();
        assert!(!manager.has_saved());

        // First enable captures snapshot
        manager.record_before_enable(|| {
            Some(SavedSystemProxy {
                enable: true,
                host: "corporate-proxy.local".to_string(),
                port: 8080,
                bypass: "local.corp".to_string(),
            })
        });
        assert!(manager.has_saved());

        // Second enable should NOT overwrite original snapshot
        manager.record_before_enable(|| {
            Some(SavedSystemProxy {
                enable: false,
                host: "overwritten".to_string(),
                port: 1234,
                bypass: "".to_string(),
            })
        });

        let popped = manager.pop_for_disable();
        assert_eq!(
            popped,
            Some(Some(SavedSystemProxy {
                enable: true,
                host: "corporate-proxy.local".to_string(),
                port: 8080,
                bypass: "local.corp".to_string(),
            }))
        );
        assert!(!manager.has_saved());

        // Popping again returns None
        assert_eq!(manager.pop_for_disable(), None);
    }

    #[test]
    fn test_proxy_state_manager_transitions_and_ownership() {
        let mut manager = ProxyStateManager::default();
        // 1. None-owned state
        assert!(!manager.has_saved());
        let action = manager.plan_disable(None);
        assert_eq!(action, DisableAction::NotOwned);

        // Calling execute_disable on none-owned must be a no-op and NOT call setter
        let res = manager.execute_disable(|| None, |_| panic!("setter called on unowned state"));
        assert!(res.is_ok());
        assert!(!manager.has_saved());

        // 2. Captured-off state (prior proxy was disabled or none)
        manager.record_before_enable(|| None);
        manager.record_applied("127.0.0.1", 7890);
        assert!(manager.has_saved());
        match &manager.owned {
            Some(snapshot) => {
                assert!(matches!(
                    snapshot.previous,
                    PreviousProxyState::CapturedOff(None)
                ));
            }
            None => panic!("Expected owned snapshot"),
        }

        // 3. Captured-on state (prior proxy was enabled)
        let mut manager_on = ProxyStateManager::default();
        manager_on.record_before_enable(|| {
            Some(SavedSystemProxy {
                enable: true,
                host: "corporate.proxy".to_string(),
                port: 8080,
                bypass: "local.corp".to_string(),
            })
        });
        manager_on.record_applied("127.0.0.1", 7890);
        assert!(manager_on.has_saved());
        match &manager_on.owned {
            Some(snapshot) => {
                assert!(matches!(
                    snapshot.previous,
                    PreviousProxyState::CapturedOn(_)
                ));
            }
            None => panic!("Expected owned snapshot"),
        }
    }

    #[test]
    fn test_failed_restore_retains_state_for_retry() {
        let mut manager = ProxyStateManager::default();
        let prev_proxy = SavedSystemProxy {
            enable: true,
            host: "corporate.proxy".to_string(),
            port: 8080,
            bypass: "local.corp".to_string(),
        };
        manager.record_before_enable(|| Some(prev_proxy.clone()));
        manager.record_applied("127.0.0.1", 7890);
        assert!(manager.has_saved());

        let current_proxy = Some(SavedSystemProxy {
            enable: true,
            host: "127.0.0.1".to_string(),
            port: 7890,
            bypass: "".to_string(),
        });

        // 1. First attempt fails in setter
        let mut attempts = 0;
        let res = manager.execute_disable(
            || current_proxy.clone(),
            |_target| {
                attempts += 1;
                Err("Simulated WinINet error".to_string())
            },
        );
        assert!(res.is_err());
        assert_eq!(attempts, 1);
        // CRUCIAL: state must be retained so caller can retry!
        assert!(
            manager.has_saved(),
            "Snapshot must be retained after failed restore"
        );

        // 2. Retry succeeds
        let mut restored_target = None;
        let retry_res = manager.execute_disable(
            || current_proxy.clone(),
            |target| {
                attempts += 1;
                restored_target = Some(target);
                Ok(())
            },
        );
        assert!(retry_res.is_ok());
        assert_eq!(attempts, 2);
        assert_eq!(
            restored_target,
            Some(RestoreTarget::RestoreConfig(prev_proxy))
        );
        // After success, state is cleared
        assert!(
            !manager.has_saved(),
            "Snapshot should be cleared after successful restore"
        );
    }

    #[test]
    fn test_failed_current_proxy_read_retains_snapshot() {
        let mut manager = ProxyStateManager::default();
        manager.record_before_enable(|| None);
        manager.record_applied("127.0.0.1", 7890);

        let result = manager.execute_disable(
            || None,
            |_| panic!("must not restore when current proxy cannot be read"),
        );
        assert!(result.is_err());
        assert!(manager.has_saved());
    }

    #[test]
    fn test_repeated_cleanup_is_idempotent() {
        let mut manager = ProxyStateManager::default();
        manager.record_before_enable(|| None);
        manager.record_applied("127.0.0.1", 7890);
        assert!(manager.has_saved());

        let current_proxy = Some(SavedSystemProxy {
            enable: true,
            host: "127.0.0.1".to_string(),
            port: 7890,
            bypass: "".to_string(),
        });

        let mut set_count = 0;
        // First cleanup succeeds
        let res1 = manager.execute_disable(
            || current_proxy.clone(),
            |target| {
                assert_eq!(target, RestoreTarget::ClearProxy);
                set_count += 1;
                Ok(())
            },
        );
        assert!(res1.is_ok());
        assert_eq!(set_count, 1);
        assert!(!manager.has_saved());

        // Second cleanup: unowned, must NOT invoke setter
        let res2 = manager.execute_disable(
            || current_proxy.clone(),
            |_| {
                set_count += 1;
                Ok(())
            },
        );
        assert!(res2.is_ok());
        assert_eq!(
            set_count, 1,
            "Setter must not be called again on unowned state"
        );

        // Third cleanup after reset: idempotent no-op
        manager.reset();
        assert!(!manager.has_saved());
        let res3 = manager.execute_disable(
            || current_proxy.clone(),
            |_| {
                set_count += 1;
                Ok(())
            },
        );
        assert!(res3.is_ok());
        assert_eq!(set_count, 1);
    }

    #[test]
    fn test_external_modification_avoids_overriding() {
        // Scenario A: Third-party tool modified host and port
        let mut manager = ProxyStateManager::default();
        manager.record_before_enable(|| {
            Some(SavedSystemProxy {
                enable: true,
                host: "corporate.proxy".to_string(),
                port: 8080,
                bypass: "local.corp".to_string(),
            })
        });
        manager.record_applied("127.0.0.1", 7890);

        let third_party_proxy = Some(SavedSystemProxy {
            enable: true,
            host: "other-vpn-proxy.local".to_string(),
            port: 9090,
            bypass: "".to_string(),
        });

        let mut setter_invoked = false;
        let res = manager.execute_disable(
            || third_party_proxy,
            |_| {
                setter_invoked = true;
                Ok(())
            },
        );
        assert!(res.is_ok());
        assert!(
            !setter_invoked,
            "Must avoid overriding third-party proxy configuration"
        );
        assert!(
            !manager.has_saved(),
            "Ownership should be released after external change detected"
        );

        // Scenario B: User turned off proxy in OS settings externally
        let mut manager_b = ProxyStateManager::default();
        manager_b.record_before_enable(|| {
            Some(SavedSystemProxy {
                enable: true,
                host: "corporate.proxy".to_string(),
                port: 8080,
                bypass: "local.corp".to_string(),
            })
        });
        manager_b.record_applied("127.0.0.1", 7890);

        let externally_disabled = Some(SavedSystemProxy {
            enable: false,
            host: "127.0.0.1".to_string(),
            port: 7890,
            bypass: "".to_string(),
        });

        setter_invoked = false;
        let res_b = manager_b.execute_disable(
            || externally_disabled,
            |_| {
                setter_invoked = true;
                Ok(())
            },
        );
        assert!(res_b.is_ok());
        assert!(
            !setter_invoked,
            "Must avoid overriding externally disabled proxy"
        );
        assert!(!manager_b.has_saved());

        // Scenario C: Current proxy matches applied proxy -> should restore
        let mut manager_c = ProxyStateManager::default();
        manager_c.record_before_enable(|| {
            Some(SavedSystemProxy {
                enable: true,
                host: "corporate.proxy".to_string(),
                port: 8080,
                bypass: "local.corp".to_string(),
            })
        });
        manager_c.record_applied("127.0.0.1", 7890);

        let matching_proxy = Some(SavedSystemProxy {
            enable: true,
            host: "127.0.0.1".to_string(),
            port: 7890,
            bypass: "".to_string(),
        });

        setter_invoked = false;
        let res_c = manager_c.execute_disable(
            || matching_proxy,
            |target| {
                setter_invoked = true;
                assert_eq!(
                    target,
                    RestoreTarget::RestoreConfig(SavedSystemProxy {
                        enable: true,
                        host: "corporate.proxy".to_string(),
                        port: 8080,
                        bypass: "local.corp".to_string(),
                    })
                );
                Ok(())
            },
        );
        assert!(res_c.is_ok());
        assert!(setter_invoked, "Must restore when not externally modified");
        assert!(!manager_c.has_saved());
    }

    #[test]
    fn test_first_enable_refuses_to_overwrite_uncaptured_proxy() {
        let mut manager = ProxyStateManager::default();
        assert!(capture_before_enable(&mut manager, || None).is_err());
        assert!(!manager.has_saved());

        let original = SavedSystemProxy {
            enable: true,
            host: "corporate.proxy".to_string(),
            port: 8080,
            bypass: "local.corp".to_string(),
        };
        capture_before_enable(&mut manager, || Some(original.clone())).unwrap();
        assert!(manager.has_saved());
        capture_before_enable(&mut manager, || panic!("must retain first snapshot")).unwrap();
        assert_eq!(
            manager.owned.as_ref().unwrap().previous,
            PreviousProxyState::CapturedOn(original)
        );
    }

    #[test]
    fn test_enable_failure_does_not_claim_ownership() {
        let mut manager = ProxyStateManager::default();
        assert!(!manager.has_saved());

        // record_before_enable records capture
        manager.record_before_enable(|| None);
        assert!(manager.has_saved());

        // If setting proxy fails before record_applied:
        manager.revert_if_never_applied();
        assert!(
            !manager.has_saved(),
            "Must revert ownership if never successfully applied"
        );
    }
}
