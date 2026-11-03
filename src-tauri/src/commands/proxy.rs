use crate::config::generator::TunSettings;
use crate::constants::*;
use std::path::Path;
use std::sync::Arc;
use std::sync::OnceLock;
use tauri::Emitter;
use tauri::Manager;
use tauri::State;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::sync::oneshot;
use tokio::sync::Mutex as AsyncMutex;

pub(crate) fn get_toggle_lock() -> &'static AsyncMutex<()> {
    static LOCK: OnceLock<AsyncMutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| AsyncMutex::new(()))
}

pub fn load_tun_settings(app: &tauri::AppHandle) -> TunSettings {
    let state = app.state::<crate::state::ProxyState>();
    let s = state.get_settings();
    TunSettings {
        auto_route: s.tun_auto_route,
        auto_redirect: s.tun_auto_redirect,
        strict_route: s.tun_strict_route,
        stack: s.tun_stack,
        mtu: s.tun_mtu,
        endpoint_independent_nat: s.tun_endpoint_independent_nat,
        sniff_enabled: s.sniff_enabled,
        sniff_http: s.sniff_http,
        sniff_tls: s.sniff_tls,
        sniff_quic: s.sniff_quic,
        sniff_override_destination: s.sniff_override_destination,
        dns_mode: s.dns_mode,
        fakeip_range: s.fakeip_range,
        primary_dns: s.primary_dns,
        fallback_dns: s.fallback_dns,
        bypass_lan: s.bypass_lan,
        final_outbound: s.final_outbound,
        use_separate_ports: s.use_separate_ports,
        http_port: s.http_port,
        socks_port: s.socks_port,
        dns_caching: s.dns_caching,
        dns_strategy: s.dns_strategy.clone(),
        direct_dns: s.direct_dns.clone(),
        fakeip_filter: s.fakeip_filter.clone(),
        dns_leak_protection: s.dns_leak_protection,
        multiplex: s.multiplex,
    }
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TrafficStats {
    pub upload_bytes: u64,
    pub download_bytes: u64,
    pub upload_speed: u64,
    pub download_speed: u64,
    pub active_connections: u32,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStateSerialized {
    pub status: crate::state::ConnectionStatus,
    pub http_port: u16,
    pub socks_port: u16,
    pub mixed_port: u16,
    pub tun_enabled: bool,
    pub effective_proxy_mode: Option<String>,
    pub uptime: u64,
    pub core_pid: Option<u32>,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
}

#[cfg(target_os = "windows")]
pub fn get_process_metrics(pid: u32) -> (u64, f32) {
    #[repr(C)]
    struct PROCESS_MEMORY_COUNTERS {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    extern "system" {
        fn K32GetProcessMemoryInfo(
            h_process: isize,
            ppsmem_counters: *mut PROCESS_MEMORY_COUNTERS,
            cb: u32,
        ) -> i32;
        fn OpenProcess(dw_desired_access: u32, b_inherit_handle: i32, dw_process_id: u32) -> isize;
        fn CloseHandle(h_object: isize) -> i32;
        fn GetProcessTimes(
            h_process: isize,
            lp_creation_time: *mut u64,
            lp_exit_time: *mut u64,
            lp_kernel_time: *mut u64,
            lp_user_time: *mut u64,
        ) -> i32;
    }

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
    const PROCESS_VM_READ: u32 = 0x0010;

    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
    let handle = if handle == 0 {
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) }
    } else {
        handle
    };

    if handle == 0 {
        return (0, 0.0);
    }

    let mut mem_counters = PROCESS_MEMORY_COUNTERS {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };

    let mem_ok = unsafe { K32GetProcessMemoryInfo(handle, &mut mem_counters, mem_counters.cb) };

    let memory_bytes = if mem_ok != 0 {
        mem_counters.working_set_size as u64
    } else {
        0
    };

    let mut creation_time = 0u64;
    let mut exit_time = 0u64;
    let mut kernel_time = 0u64;
    let mut user_time = 0u64;

    let times_ok = unsafe {
        GetProcessTimes(
            handle,
            &mut creation_time,
            &mut exit_time,
            &mut kernel_time,
            &mut user_time,
        )
    };
    unsafe { CloseHandle(handle) };

    let cpu_percent = if times_ok != 0 {
        let total_proc_time_100ns = kernel_time.saturating_add(user_time);
        static LAST_CPU: std::sync::Mutex<Option<(u32, std::time::Instant, u64)>> =
            std::sync::Mutex::new(None);
        let now = std::time::Instant::now();
        let mut guard = LAST_CPU.lock().unwrap();
        if let Some((prev_pid, prev_instant, prev_proc_time)) = *guard {
            if prev_pid == pid && now > prev_instant && total_proc_time_100ns >= prev_proc_time {
                let delta_proc_100ns = total_proc_time_100ns - prev_proc_time;
                let elapsed_100ns = (now.duration_since(prev_instant).as_nanos() / 100) as u64;
                *guard = Some((pid, now, total_proc_time_100ns));
                if elapsed_100ns > 0 {
                    let num_cpus = std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(1) as f64;
                    let pct =
                        ((delta_proc_100ns as f64) / (elapsed_100ns as f64) / num_cpus) * 100.0;
                    (pct as f32).clamp(0.0, 100.0)
                } else {
                    0.0
                }
            } else {
                *guard = Some((pid, now, total_proc_time_100ns));
                0.0
            }
        } else {
            *guard = Some((pid, now, total_proc_time_100ns));
            0.0
        }
    } else {
        0.0
    };

    (memory_bytes, cpu_percent)
}

#[cfg(target_os = "linux")]
fn parse_linux_process_times(stat: &str) -> Option<(u64, u64)> {
    // The comm field may contain spaces and parentheses, so find its closing delimiter first.
    let fields: Vec<_> = stat
        .get(stat.rfind(')')? + 1..)?
        .split_whitespace()
        .collect();
    let user_ticks = fields.get(11)?.parse::<u64>().ok()?;
    let system_ticks = fields.get(12)?.parse::<u64>().ok()?;
    let start_ticks = fields.get(19)?.parse::<u64>().ok()?;
    Some((user_ticks.saturating_add(system_ticks), start_ticks))
}

#[cfg(target_os = "linux")]
pub fn get_process_metrics(pid: u32) -> (u64, f32) {
    let status = match std::fs::read_to_string(format!("/proc/{pid}/status")) {
        Ok(status) => status,
        Err(_) => return (0, 0.0),
    };
    let memory_bytes = status
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmRSS:")?
                .split_whitespace()
                .next()?
                .parse::<u64>()
                .ok()
        })
        .unwrap_or(0)
        .saturating_mul(1024);
    let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat,
        Err(_) => return (memory_bytes, 0.0),
    };
    let Some((ticks, start_ticks)) = parse_linux_process_times(&stat) else {
        return (memory_bytes, 0.0);
    };
    let ticks_per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if ticks_per_second <= 0 {
        return (memory_bytes, 0.0);
    }
    static LAST_CPU: std::sync::Mutex<Option<(u32, u64, std::time::Instant, u64)>> =
        std::sync::Mutex::new(None);
    let now = std::time::Instant::now();
    let mut last = LAST_CPU.lock().unwrap();
    let cpu_percent = match *last {
        Some((last_pid, last_start, at, last_ticks))
            if last_pid == pid && last_start == start_ticks && ticks >= last_ticks =>
        {
            let seconds = now.duration_since(at).as_secs_f64();
            let cores = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1);
            if seconds > 0.0 {
                ((ticks - last_ticks) as f64 / ticks_per_second as f64 / seconds / cores as f64
                    * 100.0)
                    .clamp(0.0, 100.0) as f32
            } else {
                0.0
            }
        }
        _ => 0.0,
    };
    *last = Some((pid, start_ticks, now, ticks));
    (memory_bytes, cpu_percent)
}

#[cfg(target_os = "macos")]
fn parse_macos_process_metrics(output: &str, cores: usize) -> Option<(u64, f32)> {
    let mut fields = output.split_whitespace();
    let resident_kb = fields.next()?.parse::<u64>().ok()?;
    let cpu_per_core = fields.next()?.parse::<f32>().ok()?;
    if !cpu_per_core.is_finite() || cpu_per_core < 0.0 || fields.next().is_some() {
        return None;
    }
    Some((
        resident_kb.saturating_mul(1024),
        (cpu_per_core / cores.max(1) as f32).clamp(0.0, 100.0),
    ))
}

#[cfg(target_os = "macos")]
pub fn get_process_metrics(pid: u32) -> (u64, f32) {
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "rss=", "-o", "%cpu="])
        .output();
    let Ok(output) = output else { return (0, 0.0) };
    if !output.status.success() {
        return (0, 0.0);
    }
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    parse_macos_process_metrics(&String::from_utf8_lossy(&output.stdout), cores).unwrap_or((0, 0.0))
}

// Global variable to keep track of connection start time
static CONNECTION_START_TIME: std::sync::Mutex<Option<std::time::Instant>> =
    std::sync::Mutex::new(None);

fn spawn_singbox_sidecar(
    app: &tauri::AppHandle,
    state: &crate::state::ProxyState,
    config_path: &Path,
    session_id: &str,
) -> Result<oneshot::Receiver<Option<i32>>, String> {
    // Defense-in-depth: check cached sidecar version compatibility before spawning
    if let Ok(guard) = state.singbox_version.lock() {
        if let Some(ref cached) = *guard {
            crate::commands::system::check_singbox_version_compatibility(cached)
                .map_err(|e| format!("Proxy execution blocked: {}", e))?;
        }
    }

    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let _ = std::fs::create_dir_all(&app_data_dir);

    let (mut rx, child) = app
        .shell()
        .sidecar("sing-box")
        .map_err(|e| format!("Failed to initialize sidecar: {}", e))
        .and_then(|s| {
            s.args([
                "run",
                "-c",
                config_path.to_str().unwrap(),
                "-D",
                app_data_dir.to_str().unwrap(),
            ])
            .env("ENABLE_DEPRECATED_GEOSITE", "true")
            .env("ENABLE_DEPRECATED_GEOIP", "true")
            .env("ENABLE_DEPRECATED_LEGACY_DNS_FAKEIP_OPTIONS", "true")
            .env("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS", "true")
            .env("ENABLE_DEPRECATED_OUTBOUND_DNS_RULE_ITEM", "true")
            .spawn()
            .map_err(|e| format!("spawn_failed: {}", e))
        })?;

    {
        let mut process_lock = state
            .child_process
            .lock()
            .map_err(|e| format!("state_lock_poisoned: {}", e))?;
        *process_lock = Some(child);
    }

    let app_for_logs = app.clone();
    let (term_tx, term_rx) = oneshot::channel::<Option<i32>>();
    let term_tx = Arc::new(tokio::sync::Mutex::new(Some(term_tx)));
    let session_id_clone = session_id.to_string();

    tokio::spawn(async move {
        let state_for_logs = app_for_logs.state::<crate::state::ProxyState>();
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                    let text = String::from_utf8_lossy(&line).to_string();
                    state_for_logs.push_log(text.clone());
                    let _ = app_for_logs.emit("sing-box-log", text);
                }
                CommandEvent::Terminated(payload) => {
                    let mut guard = term_tx.lock().await;
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(payload.code);
                    }

                    let is_active =
                        if let Ok(active_id_guard) = state_for_logs.active_session_id.lock() {
                            active_id_guard.as_ref() == Some(&session_id_clone)
                        } else {
                            false
                        };

                    if is_active {
                        let _ = app_for_logs.emit("sing-box-terminated", payload.code);
                        state_for_logs.set_status(
                            &app_for_logs,
                            crate::state::ConnectionStatus::Disconnected,
                        );
                        *CONNECTION_START_TIME.lock().unwrap() = None;
                        crate::tray::perform_clean_cleanup(&app_for_logs);
                        crate::tray::update_tray(&app_for_logs);
                    } else {
                        state_for_logs.push_log(format!("[System] Orphaned sing-box process (session {}) exited with code {:?}.", session_id_clone, payload.code));
                    }

                    break;
                }
                _ => {}
            }
        }
    });

    Ok(term_rx)
}

async fn wait_for_startup_or_exit(
    term_rx: &mut oneshot::Receiver<Option<i32>>,
) -> Result<(), String> {
    tokio::time::sleep(std::time::Duration::from_millis(STARTUP_WAIT_MS)).await;
    match term_rx.try_recv() {
        Ok(code) => Err(format!(
            "spawn_failed: exited early during startup with code {:?}",
            code
        )),
        Err(oneshot::error::TryRecvError::Closed) => {
            Err("spawn_failed: child process terminated unexpectedly during startup".into())
        }
        Err(oneshot::error::TryRecvError::Empty) => Ok(()),
    }
}

fn health_check_port(settings: &crate::state::UserSettings) -> u16 {
    if settings.use_separate_ports {
        settings.http_port
    } else {
        settings.mixed_port
    }
}

fn mode_for_tun_support(requested: &str, tun_supported: bool) -> &str {
    if requested == "tun" && !tun_supported {
        "system"
    } else {
        requested
    }
}

#[cfg(test)]
mod startup_tests {
    use super::{health_check_port, mode_for_tun_support};
    use crate::state::UserSettings;

    #[test]
    fn health_checks_use_the_configured_http_listener() {
        let mut settings = UserSettings::default();
        assert_eq!(health_check_port(&settings), settings.mixed_port);
        settings.use_separate_ports = true;
        assert_eq!(health_check_port(&settings), settings.http_port);
    }

    #[test]
    fn unsupported_tun_falls_back_without_changing_other_modes() {
        assert_eq!(mode_for_tun_support("tun", false), "system");
        assert_eq!(mode_for_tun_support("tun", true), "tun");
        assert_eq!(mode_for_tun_support("system", false), "system");
    }

    #[test]
    fn test_get_process_metrics_self() {
        let current_pid = std::process::id();
        let (mem, cpu) = super::get_process_metrics(current_pid);
        assert!(
            mem > 0,
            "Current process memory should be > 0 bytes, got {}",
            mem
        );
        assert!(
            (0.0..=100.0).contains(&cpu),
            "CPU percent should be 0-100%, got {}",
            cpu
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_cpu_parser_handles_spaces_in_process_name_and_rejects_short_stat() {
        let mut fields = vec!["0"; 22];
        fields[11] = "14";
        fields[12] = "6";
        fields[19] = "123";
        let stat = format!("42 (my (proxy) process) {}", fields.join(" "));
        assert_eq!(super::parse_linux_process_times(&stat), Some((20, 123)));
        assert_eq!(super::parse_linux_process_times("42 (process) S 1"), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_metrics_parser_normalizes_cpu_and_rejects_invalid_values() {
        assert_eq!(
            super::parse_macos_process_metrics("2048 50.0", 2),
            Some((2097152, 25.0))
        );
        assert_eq!(super::parse_macos_process_metrics("2048 -2", 2), None);
        assert_eq!(super::parse_macos_process_metrics("garbage", 2), None);
    }
}

async fn probe_via_http_proxy(port: u16) -> Result<(), String> {
    let proxy = reqwest::Proxy::all(format!("http://127.0.0.1:{}", port))
        .map_err(|e| format!("HTTP proxy probe setup failed: {}", e))?;
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(HEALTH_CHECK_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("HTTP proxy probe setup failed: {}", e))?;

    client
        .get(HEALTH_CHECK_URL)
        .send()
        .await
        .map_err(|e| {
            format!(
                "HTTP proxy probe failed: node/outbound/config did not carry HTTPS traffic ({})",
                e
            )
        })?
        .error_for_status()
        .map(|_| ())
        .map_err(|e| format!("HTTP proxy probe failed: upstream returned {}", e))
}

async fn probe_via_tun_route() -> Result<(), String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(HEALTH_CHECK_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("TUN probe client failed: {}", e))?;

    client
        .get(HEALTH_CHECK_URL)
        .send()
        .await
        .map_err(|e| {
            format!(
                "TUN probe failed: OS traffic did not traverse TUN successfully ({})",
                e
            )
        })?
        .error_for_status()
        .map(|_| ())
        .map_err(|e| format!("TUN probe failed: upstream returned {}", e))
}

async fn run_startup_health_checks(port: u16, proxy_mode: &str) -> Result<(), String> {
    probe_via_http_proxy(port).await?;
    if proxy_mode == "tun" {
        probe_via_tun_route().await?;
    }
    Ok(())
}

fn patch_config_for_fallback(config_path: &std::path::Path, attempt: usize) -> Result<(), String> {
    let content = std::fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read active config for fallback patching: {}", e))?;
    let mut config_val: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse active config for fallback patching: {}", e))?;

    match attempt {
        1 => {
            // Attempt 2 (attempt = 1): Firefox uTLS fingerprint
            if let Some(outbounds) = config_val
                .get_mut("outbounds")
                .and_then(|o| o.as_array_mut())
            {
                for outbound in outbounds {
                    let ob_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if ob_type == "vless" || ob_type == "vmess" || ob_type == "trojan" {
                        if let Some(tls) = outbound.get_mut("tls").and_then(|t| t.as_object_mut()) {
                            if let Some(utls) = tls.get_mut("utls").and_then(|u| u.as_object_mut())
                            {
                                utls.insert("enabled".to_string(), serde_json::Value::Bool(true));
                                utls.insert(
                                    "fingerprint".to_string(),
                                    serde_json::json!("firefox"),
                                );
                            }
                        }
                    }
                }
            }
        }
        2 => {
            // Attempt 3 (attempt = 2): Disable uTLS completely (Native Go TLS)
            if let Some(outbounds) = config_val
                .get_mut("outbounds")
                .and_then(|o| o.as_array_mut())
            {
                for outbound in outbounds {
                    let ob_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if ob_type == "vless" || ob_type == "vmess" || ob_type == "trojan" {
                        if let Some(tls) = outbound.get_mut("tls").and_then(|t| t.as_object_mut()) {
                            if let Some(utls) = tls.get_mut("utls").and_then(|u| u.as_object_mut())
                            {
                                utls.insert("enabled".to_string(), serde_json::Value::Bool(false));
                            }
                        }
                    }
                }
            }
        }
        3 => {
            // Attempt 4 (attempt = 3): DNS Fallback (Switch DNS rules resolving proxy domains to direct query via public bootstrap dns)
            // First, add public DNS servers detoured direct using modern sing-box typed format
            if let Some(dns) = config_val.get_mut("dns").and_then(|d| d.as_object_mut()) {
                if let Some(servers) = dns.get_mut("servers").and_then(|s| s.as_array_mut()) {
                    // Update local-dns to point to fallback public DNS
                    for server in servers.iter_mut() {
                        if server.get("tag").and_then(|t| t.as_str()) == Some("local-dns") {
                            *server = crate::config::generator::parse_dns_server(
                                "local-dns",
                                FALLBACK_DNS_PRIMARY,
                                Some("direct"),
                            );
                        }
                    }

                    // Add bootstrap-dns tags if not already present
                    if !servers.iter().any(|s| {
                        s.get("tag").and_then(|t| t.as_str()) == Some("fallback-bootstrap-1")
                    }) {
                        servers.push(crate::config::generator::parse_dns_server(
                            "fallback-bootstrap-1",
                            FALLBACK_DNS_PRIMARY,
                            Some("direct"),
                        ));
                        servers.push(crate::config::generator::parse_dns_server(
                            "fallback-bootstrap-2",
                            FALLBACK_DNS_SECONDARY,
                            Some("direct"),
                        ));
                    }
                }

                // Point proxy domain resolution rule to our fallback public DNS instead of local-dns
                if let Some(rules) = dns.get_mut("rules").and_then(|r| r.as_array_mut()) {
                    for rule in rules {
                        // Find rule containing domain list
                        if rule.get("domain").is_some() {
                            rule["server"] = serde_json::json!("fallback-bootstrap-1");
                        }
                    }
                }
            }

            // Ensure route.default_domain_resolver points to the fallback DNS
            if let Some(route) = config_val.get_mut("route").and_then(|r| r.as_object_mut()) {
                route.insert(
                    "default_domain_resolver".to_string(),
                    serde_json::json!("fallback-bootstrap-1"),
                );
            }

            // Explicitly set domain_resolver on outbound proxy nodes so node dialling uses fallback DNS
            if let Some(outbounds) = config_val
                .get_mut("outbounds")
                .and_then(|o| o.as_array_mut())
            {
                for outbound in outbounds {
                    let ob_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if ob_type != "direct" && ob_type != "block" && ob_type != "dns" {
                        outbound["domain_resolver"] = serde_json::json!("fallback-bootstrap-1");
                    }
                }
            }

            // Ensure fallback DNS IPs are excluded from TUN interface routing
            if let Some(inbounds) = config_val
                .get_mut("inbounds")
                .and_then(|i| i.as_array_mut())
            {
                for inbound in inbounds {
                    if inbound.get("type").and_then(|t| t.as_str()) == Some("tun") {
                        if let Some(excludes) = inbound
                            .get_mut("route_exclude_address")
                            .and_then(|e| e.as_array_mut())
                        {
                            let fb1 = serde_json::json!(format!("{}/32", FALLBACK_DNS_PRIMARY));
                            let fb2 = serde_json::json!(format!("{}/32", FALLBACK_DNS_SECONDARY));
                            if !excludes.contains(&fb1) {
                                excludes.push(fb1);
                            }
                            if !excludes.contains(&fb2) {
                                excludes.push(fb2);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    std::fs::write(
        config_path,
        serde_json::to_string_pretty(&config_val).unwrap(),
    )
    .map_err(|e| format!("Failed to write fallback config: {}", e))?;
    Ok(())
}

fn stop_startup_child(app: &tauri::AppHandle, state: &crate::state::ProxyState) {
    if let Ok(mut session_lock) = state.active_session_id.lock() {
        *session_lock = None;
    }
    if let Ok(mut process_lock) = state.child_process.lock() {
        if let Some(child) = process_lock.take() {
            let _ = child.kill();
        }
    }
    let _ = crate::os::disable_system_proxy();
    *CONNECTION_START_TIME.lock().unwrap() = None;
    crate::tray::update_tray(app);
}

#[tauri::command]
pub fn get_proxy_status(
    state: State<'_, crate::state::ProxyState>,
) -> Result<ProxyStateSerialized, String> {
    let status = state.get_status();
    let uptime = if let Some(start) = *CONNECTION_START_TIME.lock().unwrap() {
        start.elapsed().as_secs()
    } else {
        0
    };

    let settings = state.get_settings();
    let effective_proxy_mode = state
        .effective_proxy_mode
        .lock()
        .map_err(|e| format!("state_lock_poisoned: {}", e))?
        .clone();
    let tun_enabled = status == crate::state::ConnectionStatus::Connected
        && effective_proxy_mode.as_deref() == Some("tun");

    let core_pid = if status == crate::state::ConnectionStatus::Connected {
        state
            .child_process
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|child| child.pid()))
    } else {
        None
    };

    let (memory_bytes, cpu_percent) = if let Some(pid) = core_pid {
        get_process_metrics(pid)
    } else {
        (0, 0.0)
    };

    Ok(ProxyStateSerialized {
        status,
        http_port: settings.http_port,
        socks_port: settings.socks_port,
        mixed_port: settings.mixed_port,
        tun_enabled,
        effective_proxy_mode,
        uptime,
        core_pid,
        memory_bytes,
        cpu_percent,
    })
}

#[tauri::command]
pub async fn get_traffic_stats(
    state: State<'_, crate::state::ProxyState>,
) -> Result<TrafficStats, String> {
    if !state.is_running() {
        return Ok(TrafficStats {
            upload_bytes: 0,
            download_bytes: 0,
            upload_speed: 0,
            download_speed: 0,
            active_connections: 0,
        });
    }

    Ok(TrafficStats {
        upload_bytes: *state.upload_bytes.lock().map_err(|e| e.to_string())?,
        download_bytes: *state.download_bytes.lock().map_err(|e| e.to_string())?,
        upload_speed: *state.upload_speed.lock().map_err(|e| e.to_string())?,
        download_speed: *state.download_speed.lock().map_err(|e| e.to_string())?,
        active_connections: *state.active_connections.lock().map_err(|e| e.to_string())?,
    })
}

pub async fn prepare_and_patch_config(
    app: &tauri::AppHandle,
    state: &crate::state::ProxyState,
    selected_outbound_tag: Option<&str>,
    mode_override: Option<&str>,
) -> Result<(std::path::PathBuf, String, u16, String), String> {
    // Reload settings from disk and update cache
    let settings = state.reload_settings(app)?;

    let dns_address =
        crate::config::resolve_dns_address(if settings.dns_address.trim().is_empty() {
            None
        } else {
            Some(&settings.dns_address)
        });
    let wifi_sharing = settings.wifi_sharing;
    let api_port = settings.api_port;
    let api_secret = settings.api_secret.clone();
    let requested_mode = mode_override.unwrap_or(&settings.proxy_mode);
    let tun_supported = crate::commands::system::check_tun_support(app.clone());
    let proxy_mode = mode_for_tun_support(requested_mode, tun_supported).to_string();
    if requested_mode == "tun" && proxy_mode == "system" {
        state.push_log(
            "[System] TUN privileges are unavailable. Using System Proxy for this session."
                .to_string(),
        );
    }

    let listen_address = if wifi_sharing { "0.0.0.0" } else { "127.0.0.1" };
    let config_path = crate::commands::config::get_active_config_path(app)?;

    // Write a default valid config if the file doesn't exist
    if !config_path.exists() {
        let mixed_port = settings.mixed_port;
        let default_config = format!(
            r#"{{
  "log": {{
    "level": "info",
    "timestamp": true
  }},
  "inbounds": [
    {{
      "type": "mixed",
      "tag": "mixed-in",
      "listen": "127.0.0.1",
      "listen_port": {}
    }}
  ],
  "outbounds": [
    {{ "type": "direct", "tag": "direct" }}
  ],
  "route": {{
    "rules": [
      {{ "ip_is_private": true, "outbound": "direct" }}
    ],
    "final": "direct",
    "auto_detect_interface": true
  }}
}}"#,
            mixed_port
        );
        std::fs::write(&config_path, default_config)
            .map_err(|e| format!("Failed to write default config: {}", e))?;
    }

    // Get the active profile ID and load its outbounds instead of reading the stale/fallback active.json
    let active_profile_id =
        match crate::commands::config::get_active_profile_id(app) {
            Ok(Some(id)) => id,
            _ => return Err(
                "No active profile selected. Please select or import a profile before connecting."
                    .to_string(),
            ),
        };

    let profile_nodes =
        crate::commands::config::get_profile_outbounds(app.clone(), active_profile_id)?;

    // Guard: If there are no nodes in the active profile, return an error
    if profile_nodes.is_empty() {
        return Err(
            "No server profiles found. Please import a profile before connecting.".to_string(),
        );
    }

    let node_outbounds: Vec<crate::config::adapters::SingBoxOutbound> =
        serde_json::from_value(serde_json::Value::Array(profile_nodes))
            .map_err(|e| format!("Failed to deserialize profile nodes: {}", e))?;

    let tun_settings = load_tun_settings(app);
    let mixed_port = settings.mixed_port;
    let (user_rules, rule_sets) = crate::config::rules::load_routing_rules_from_file(app);
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    // Generate a clean sing-box config from scratch
    let generated_config = crate::config::generator::generate_singbox_config(
        mixed_port,
        node_outbounds,
        &proxy_mode,
        &dns_address,
        listen_address,
        &tun_settings,
        selected_outbound_tag,
        &user_rules,
        &rule_sets,
        &app_data_dir,
    )?;

    let mut final_config_val: serde_json::Value = serde_json::from_str(&generated_config)
        .map_err(|e| format!("Generated config is invalid JSON: {}", e))?;

    // Inject Clash API experimental settings if enabled and authenticated
    if let Some(clash_api) = build_clash_api_config(
        settings.api_enabled,
        settings.api_port,
        &settings.api_secret,
        settings.api_cors,
    ) {
        let mut exp = final_config_val
            .get("experimental")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        exp["clash_api"] = clash_api;
        final_config_val["experimental"] = exp;
    } else if let Some(exp) = final_config_val
        .get_mut("experimental")
        .and_then(|e| e.as_object_mut())
    {
        exp.remove("clash_api");
        if exp.is_empty() {
            final_config_val
                .as_object_mut()
                .unwrap()
                .remove("experimental");
        }
    }

    // Safe Overwrite (Transactional Write): Validate first
    let temp_config_path = config_path.with_extension("json.tmp");
    std::fs::write(
        &temp_config_path,
        serde_json::to_string_pretty(&final_config_val).unwrap(),
    )
    .map_err(|e| format!("Failed to write temporary config: {}", e))?;

    // Perform validation check
    if let Err(err) =
        crate::config::validator::validate_singbox_config(app, &temp_config_path).await
    {
        let _ = std::fs::remove_file(&temp_config_path);
        return Err(format!("Configuration check failed: {}", err));
    }

    // Overwrite the original active.json only if validation passes
    std::fs::rename(&temp_config_path, &config_path)
        .map_err(|e| format!("Failed to finalize active config write: {}", e))?;

    Ok((config_path, proxy_mode, api_port, api_secret))
}

/// Pure controller policy: builds the sing-box `experimental.clash_api` configuration.
///
/// Security constraints:
/// 1. Controller is never enabled without authentication (requires a non-empty `api_secret`).
/// 2. If `api_enabled` is false, returns `None` (controller is not started).
/// 3. CORS wildcard (`*`) is never set when `api_cors` is false.
pub fn build_clash_api_config(
    api_enabled: bool,
    api_port: u16,
    api_secret: &str,
    api_cors: bool,
) -> Option<serde_json::Value> {
    if !api_enabled {
        return None;
    }

    let trimmed_secret = api_secret.trim();
    if trimmed_secret.is_empty() {
        // Controller must never start enabled without auth
        return None;
    }

    let mut clash_api = serde_json::json!({
        "external_controller": format!("127.0.0.1:{}", api_port),
        "secret": trimmed_secret,
    });

    if api_cors {
        clash_api["access_control_allow_origin"] = serde_json::json!(["*"]);
    }

    Some(clash_api)
}

#[allow(dead_code)]
async fn hot_reload_via_api(
    config_path: &std::path::Path,
    api_port: u16,
    api_secret: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let path_str = config_path.to_string_lossy();
    let mut req = client
        .put(format!("http://127.0.0.1:{}/configs", api_port))
        .json(&serde_json::json!({ "path": path_str, "force": false }));
    if !api_secret.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_secret));
    }
    req.send()
        .await
        .map_err(|e| format!("Hot reload API call failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Hot reload rejected by sing-box: {}", e))?;
    Ok(())
}

async fn switch_selector_node_via_api(
    tag: &str,
    api_port: u16,
    api_secret: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let mut req = client
        .put(format!("http://127.0.0.1:{}/proxies/proxy", api_port))
        .json(&serde_json::json!({ "name": tag }));
    if !api_secret.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_secret));
    }
    req.send()
        .await
        .map_err(|e| format!("Selector switch HTTP request failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Selector switch rejected by sing-box API: {}", e))?;
    Ok(())
}

/// Switch the active node via the Clash API without restarting sing-box.
/// Falls back to a full config reload if the API call fails or the API is disabled.
#[tauri::command]
pub async fn switch_node_hot(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::ProxyState>,
    tag: String,
) -> Result<(), String> {
    let settings = state.get_settings();
    let mode_override = state
        .effective_proxy_mode
        .lock()
        .ok()
        .and_then(|mode| mode.clone());
    let api_port = settings.api_port;
    let api_secret = settings.api_secret.clone();
    let api_enabled = settings.api_enabled && !api_secret.trim().is_empty();

    if state.get_status() == crate::state::ConnectionStatus::Connected && api_enabled {
        let (config_path, _, _, _) = match prepare_and_patch_config(
            &app,
            &state,
            Some(&tag),
            mode_override.as_deref(),
        )
        .await
        {
            Ok(res) => res,
            Err(e) => {
                state.push_log(format!("[System] Config patch failed: {}", e));
                try_reload_proxy_config(&app, &state, Some(tag.clone())).await?;
                if let Ok(mut lock) = state.selected_outbound_tag.lock() {
                    *lock = Some(tag);
                }
                return Ok(());
            }
        };

        // Try hot-reloading the configuration first so the running sing-box gets the updated active.json
        if let Err(e) = hot_reload_via_api(&config_path, api_port, &api_secret).await {
            state.push_log(format!(
                "[System] API config reload failed ({}), falling back to full reload.",
                e
            ));
            try_reload_proxy_config(&app, &state, Some(tag.clone())).await?;
            if let Ok(mut lock) = state.selected_outbound_tag.lock() {
                *lock = Some(tag);
            }
            return Ok(());
        }

        // Try switching the selector group
        match switch_selector_node_via_api(&tag, api_port, &api_secret).await {
            Ok(_) => {
                if let Ok(mut lock) = state.selected_outbound_tag.lock() {
                    *lock = Some(tag.clone());
                }
                state.push_log(format!("[System] Hot-switched selector to node: {}", tag));
                return Ok(());
            }
            Err(e) => {
                state.push_log(format!(
                    "[System] Selector switch API failed ({}), falling back to full reload.",
                    e
                ));
            }
        }
    }

    try_reload_proxy_config(&app, &state, Some(tag.clone())).await?;
    if let Ok(mut lock) = state.selected_outbound_tag.lock() {
        *lock = Some(tag);
    }
    Ok(())
}

pub fn try_reload_proxy_config<'a>(
    app: &'a tauri::AppHandle,
    state: &'a crate::state::ProxyState,
    selected_outbound_tag: Option<String>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
    let mode_override = state
        .effective_proxy_mode
        .lock()
        .ok()
        .and_then(|mode| mode.clone());
    try_reload_proxy_config_with_mode(app, selected_outbound_tag, mode_override)
}

pub fn try_reload_proxy_config_with_mode<'a>(
    app: &'a tauri::AppHandle,
    selected_outbound_tag: Option<String>,
    mode_override: Option<String>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
    let app_handle = app.clone();
    let state_handle = app.state::<crate::state::ProxyState>();
    let tag = selected_outbound_tag;

    Box::pin(async move {
        let _ = prepare_and_patch_config(
            &app_handle,
            &state_handle,
            tag.as_deref(),
            mode_override.as_deref(),
        )
        .await?;

        // Stop the proxy
        toggle_proxy(app_handle.clone(), state_handle.clone(), false, None, None).await?;

        // Give it a brief moment to shut down gracefully
        tokio::time::sleep(std::time::Duration::from_millis(SHUTDOWN_GRACE_PERIOD_MS)).await;

        // Start the proxy
        toggle_proxy(app_handle, state_handle, true, tag, mode_override)
            .await
            .map(|_| ())
    })
}

#[tauri::command]
pub async fn reload_active_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::ProxyState>,
    selected_outbound_tag: Option<String>,
) -> Result<(), String> {
    try_reload_proxy_config(&app, &state, selected_outbound_tag).await
}

#[tauri::command]
pub async fn toggle_proxy(
    app: tauri::AppHandle,
    state: State<'_, crate::state::ProxyState>,
    start: bool,
    selected_outbound_tag: Option<String>,
    mode_override: Option<String>,
) -> Result<String, String> {
    let _guard = get_toggle_lock().lock().await;
    if let Some(ref mode) = mode_override {
        if mode != "system" && mode != "tun" {
            return Err("Invalid proxy mode override".to_string());
        }
    }

    // ── STOP PATH ────────────────────────────────────────────────────────────
    if !start {
        {
            let mut process_lock = state
                .child_process
                .lock()
                .map_err(|e| format!("state_lock_poisoned: {}", e))?;
            if let Some(child) = process_lock.take() {
                child
                    .kill()
                    .map_err(|e| format!("Failed to stop owned proxy process: {}", e))?;
            }
        }

        // Only the child owned by this X-Link session may be terminated.
        // An unrelated sing-box process must never be killed by image name.

        // Clear system proxy before reporting a successful stop.
        crate::os::disable_system_proxy()?;
        state.set_status(&app, crate::state::ConnectionStatus::Disconnected);

        // Clear global state
        if let Ok(mut session_lock) = state.active_session_id.lock() {
            *session_lock = None;
        }
        *CONNECTION_START_TIME.lock().unwrap() = None;

        // Reset bandwidth metrics
        if let Ok(mut up_s) = state.upload_speed.lock() {
            *up_s = 0;
        }
        if let Ok(mut down_s) = state.download_speed.lock() {
            *down_s = 0;
        }
        if let Ok(mut conn) = state.active_connections.lock() {
            *conn = 0;
        }

        // Sync native system tray
        crate::tray::update_tray(&app);

        // Revert active.json to a safe direct-only fallback config
        if let Ok(config_path) = crate::commands::config::get_active_config_path(&app) {
            let mixed_port = state.get_settings().mixed_port;
            let default_config = format!(
                r#"{{
  "log": {{
    "level": "info",
    "timestamp": true
  }},
  "inbounds": [
    {{
      "type": "mixed",
      "tag": "mixed-in",
      "listen": "127.0.0.1",
      "listen_port": {}
    }}
  ],
  "outbounds": [
    {{ "type": "direct", "tag": "direct" }}
  ],
  "route": {{
    "rules": [
      {{ "ip_is_private": true, "outbound": "direct" }}
    ],
    "final": "direct",
    "auto_detect_interface": true
  }}
}}"#,
                mixed_port
            );
            let _ = std::fs::write(&config_path, default_config);
        }

        return Ok("stopped".into());
    }

    // ── START PATH ───────────────────────────────────────────────────────────
    // If already connected, only a node-switch (different selected_outbound_tag)
    // warrants action -- otherwise this is a no-op.
    if start && state.get_status() == crate::state::ConnectionStatus::Connected {
        let current_tag = crate::commands::config::get_selected_outbound_tag(&app);
        if selected_outbound_tag.is_some() && selected_outbound_tag != current_tag {
            let result = try_reload_proxy_config(&app, &state, selected_outbound_tag.clone()).await;
            match result {
                Ok(_) => {
                    state.push_log("[System] Dynamic node switch succeeded.".to_string());
                    return Ok("started".into());
                }
                Err(e) => {
                    state.push_log(format!(
                        "[System] Dynamic node switch failed: {}. Falling back to clean restart...",
                        e
                    ));
                }
            }
        } else {
            return Ok("started".into());
        }
    }

    // Verify sidecar binary compatibility before transitioning status or running proxy
    crate::commands::system::ensure_sidecar_compatible(&app)
        .await
        .map_err(|e| format!("Proxy startup aborted: {}", e))?;

    state.set_status(&app, crate::state::ConnectionStatus::Connecting);
    crate::tray::update_tray(&app);
    // Clean up a previously tracked child only; a port owned by another
    // application must be reported as a conflict, not forcibly cleared.
    let mut killed_stale = false;
    {
        let mut process_lock = state
            .child_process
            .lock()
            .map_err(|e| format!("state_lock_poisoned: {}", e))?;
        if let Some(child) = process_lock.take() {
            if let Err(error) = child.kill() {
                state.set_status(&app, crate::state::ConnectionStatus::Disconnected);
                return Err(format!(
                    "Failed to stop stale owned proxy process: {}",
                    error
                ));
            }
            killed_stale = true;
        }
    }

    if killed_stale {
        let _ = crate::os::disable_system_proxy();
        *CONNECTION_START_TIME.lock().unwrap() = None;
        tokio::time::sleep(std::time::Duration::from_millis(
            STALE_PROCESS_KILL_DELAY_MS,
        ))
        .await;
    }

    let (config_path, resolved_proxy_mode, api_port, api_secret) = match prepare_and_patch_config(
        &app,
        &state,
        selected_outbound_tag.as_deref(),
        mode_override.as_deref(),
    )
    .await
    {
        Ok(res) => res,
        Err(e) => {
            state.set_status(&app, crate::state::ConnectionStatus::Disconnected);
            crate::tray::update_tray(&app);
            return Err(e);
        }
    };

    let mut current_mode = resolved_proxy_mode.clone();
    let mut attempt = 0;
    let mut last_error = String::new();
    let mut connected_successfully = false;

    // Helper helper to apply TUN compatibility in-place
    let apply_tun_compatibility_in_place = |path: &std::path::Path| -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read active config for TUN compatibility: {}", e))?;
        let mut config_val = serde_json::from_str::<serde_json::Value>(&content)
            .map_err(|e| format!("Failed to parse active config for TUN compatibility: {}", e))?;
        crate::config::apply_tun_compatibility_profile(&mut config_val);
        std::fs::write(path, serde_json::to_string_pretty(&config_val).unwrap())
            .map_err(|e| format!("Failed to write TUN compatibility fallback config: {}", e))?;
        Ok(())
    };

    while attempt < MAX_FALLBACK_ATTEMPTS {
        if attempt > 0 {
            // Apply fallback patch for current attempt
            if attempt <= MAX_CONFIG_PATCH_ATTEMPTS {
                if let Err(e) = patch_config_for_fallback(&config_path, attempt) {
                    state.push_log(format!("[System] Fallback patching failed: {}", e));
                }
            } else if attempt == (MAX_FALLBACK_ATTEMPTS - 1) && current_mode == "tun" {
                state.push_log(
                    "[System] Retrying with Windows TUN compatibility profile...".to_string(),
                );
                if let Err(e) = apply_tun_compatibility_in_place(&config_path) {
                    state.push_log(format!("[System] TUN compatibility patching failed: {}", e));
                }
            }
        }

        let session_id = uuid::Uuid::new_v4().to_string();
        {
            if let Ok(mut session_lock) = state.active_session_id.lock() {
                *session_lock = Some(session_id.clone());
            }
        }

        #[cfg(target_os = "windows")]
        if current_mode == "tun" {
            crate::copy_wintun_dll_to_sidecar_dir(&app);
        }

        let mut term_rx = match spawn_singbox_sidecar(&app, &state, &config_path, &session_id) {
            Ok(rx) => rx,
            Err(e) => {
                last_error = e.clone();
                state.push_log(format!(
                    "[System] Attempt {} failed to spawn: {}",
                    attempt + 1,
                    e
                ));
                attempt += 1;
                tokio::time::sleep(std::time::Duration::from_millis(FALLBACK_RETRY_DELAY_MS)).await;
                continue;
            }
        };

        if let Err(e) = wait_for_startup_or_exit(&mut term_rx).await {
            stop_startup_child(&app, &state);
            last_error = e.clone();
            state.push_log(format!(
                "[System] Attempt {} process exited early: {}",
                attempt + 1,
                e
            ));
            attempt += 1;
            tokio::time::sleep(std::time::Duration::from_millis(FALLBACK_RETRY_DELAY_MS)).await;
            continue;
        }

        let port = health_check_port(&state.get_settings());
        match run_startup_health_checks(port, &current_mode).await {
            Ok(_) => {
                state.push_log("[System] Connection verified successfully.".to_string());
                connected_successfully = true;
                break;
            }
            Err(probe_err) => {
                stop_startup_child(&app, &state);
                last_error = probe_err.clone();
                state.push_log(format!(
                    "[System] Attempt {} probe failed: {}",
                    attempt + 1,
                    probe_err
                ));

                attempt += 1;
                if attempt < MAX_FALLBACK_ATTEMPTS {
                    match attempt {
                        1 => state.push_log(
                            "[System] Retrying with Firefox TLS fingerprint fallback..."
                                .to_string(),
                        ),
                        2 => state.push_log(
                            "[System] Retrying with Native TLS (uTLS disabled) fallback..."
                                .to_string(),
                        ),
                        3 => state.push_log(
                            "[System] Retrying with Public DNS bootstrap fallback...".to_string(),
                        ),
                        _ => {}
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(FALLBACK_RETRY_DELAY_MS))
                        .await;
                }
            }
        }
    }

    if !connected_successfully {
        if resolved_proxy_mode == "tun" {
            state.push_log("[System] TUN mode failed completely after all fallback attempts. Rolling back to System Proxy mode...".to_string());

            // Switch mode dynamically
            current_mode = "system".to_string();

            // The fallback has the same ownership and status rollback guarantees as a normal attempt.
            let fallback_result: Result<(), String> = async {
                let (retry_config_path, _, _, _) = prepare_and_patch_config(
                    &app,
                    &state,
                    selected_outbound_tag.as_deref(),
                    Some("system"),
                )
                .await?;

                let session_id = uuid::Uuid::new_v4().to_string();
                if let Ok(mut session_lock) = state.active_session_id.lock() {
                    *session_lock = Some(session_id.clone());
                }

                let mut term_rx =
                    spawn_singbox_sidecar(&app, &state, &retry_config_path, &session_id)
                        .map_err(|e| format!("System proxy fallback spawn failed: {}", e))?;
                wait_for_startup_or_exit(&mut term_rx)
                    .await
                    .map_err(|e| format!("System proxy fallback process exited: {}", e))?;

                let port = health_check_port(&state.get_settings());
                run_startup_health_checks(port, &current_mode)
                    .await
                    .map_err(|e| format!("System proxy fallback health check failed: {}", e))
            }
            .await;

            if let Err(error) = fallback_result {
                stop_startup_child(&app, &state);
                state.set_status(&app, crate::state::ConnectionStatus::Disconnected);
                crate::tray::update_tray(&app);
                return Err(error);
            }

            state.push_log("[System] Connected successfully in System Proxy mode.".to_string());
        } else {
            state.set_status(&app, crate::state::ConnectionStatus::Disconnected);
            crate::tray::update_tray(&app);
            return Err(format!(
                "Connection failed after {} fallback attempts: {}",
                MAX_FALLBACK_ATTEMPTS, last_error
            ));
        }
    }

    // Spawn background traffic monitoring task only when API controller is active
    let is_api_active = state.get_settings().api_enabled && !api_secret.trim().is_empty();
    if is_api_active {
        let app_handle = app.clone();
        let api_secret_clone = api_secret.clone();
        tokio::spawn(async move {
            let state_clone = app_handle.state::<crate::state::ProxyState>();
            let client = reqwest::Client::new();
            let url = format!("http://127.0.0.1:{}/connections", api_port);
            let mut prev_upload = 0u64;
            let mut prev_download = 0u64;
            let mut consecutive_failures = 0;

            loop {
                if !state_clone.is_running() {
                    break;
                }

                let mut req = client.get(&url);
                if !api_secret_clone.is_empty() {
                    req = req.header("Authorization", format!("Bearer {}", api_secret_clone));
                }

                match req.send().await {
                    Ok(resp) => {
                        #[derive(serde::Deserialize)]
                        struct ConnectionsResponse {
                            #[serde(rename = "uploadTotal")]
                            upload_total: u64,
                            #[serde(rename = "downloadTotal")]
                            download_total: u64,
                            connections: Vec<serde_json::Value>,
                        }

                        if let Ok(data) = resp.json::<ConnectionsResponse>().await {
                            consecutive_failures = 0;
                            let mut up_speed = 0u64;
                            let mut down_speed = 0u64;

                            if prev_upload > 0 && data.upload_total >= prev_upload {
                                up_speed = data.upload_total - prev_upload;
                            }
                            if prev_download > 0 && data.download_total >= prev_download {
                                down_speed = data.download_total - prev_download;
                            }

                            prev_upload = data.upload_total;
                            prev_download = data.download_total;

                            if let Ok(mut up_b) = state_clone.upload_bytes.lock() {
                                *up_b = data.upload_total;
                            }
                            if let Ok(mut down_b) = state_clone.download_bytes.lock() {
                                *down_b = data.download_total;
                            }
                            if let Ok(mut up_s) = state_clone.upload_speed.lock() {
                                *up_s = up_speed;
                            }
                            if let Ok(mut down_s) = state_clone.download_speed.lock() {
                                *down_s = down_speed;
                            }
                            if let Ok(mut conn) = state_clone.active_connections.lock() {
                                *conn = data.connections.len() as u32;
                            }

                            // Update tray tooltip with live speed
                            crate::tray::update_tray_tooltip(&app_handle);
                        } else {
                            consecutive_failures += 1;
                        }
                    }
                    Err(_) => {
                        consecutive_failures += 1;
                    }
                }

                if consecutive_failures > TRAFFIC_MAX_CONSECUTIVE_FAILURES {
                    break;
                }

                tokio::time::sleep(std::time::Duration::from_secs(TRAFFIC_POLL_INTERVAL_SECS))
                    .await;
            }

            if let Ok(mut up_s) = state_clone.upload_speed.lock() {
                *up_s = 0;
            };
            if let Ok(mut down_s) = state_clone.download_speed.lock() {
                *down_s = 0;
            };
            if let Ok(mut conn) = state_clone.active_connections.lock() {
                *conn = 0;
            };

            // Reset tooltip on disconnect
            crate::tray::update_tray_tooltip(&app_handle);
        });
    } else {
        if let Ok(mut up_s) = state.upload_speed.lock() {
            *up_s = 0;
        }
        if let Ok(mut down_s) = state.download_speed.lock() {
            *down_s = 0;
        }
        if let Ok(mut conn) = state.active_connections.lock() {
            *conn = 0;
        }
        crate::tray::update_tray_tooltip(&app);
    }

    // Enable system proxy if the final connection mode is 'system' (either by default or as a fallback)
    if current_mode == "system" {
        let port = health_check_port(&state.get_settings());
        if let Err(e) = crate::os::enable_system_proxy("127.0.0.1", port) {
            crate::tray::perform_clean_cleanup(&app);
            state.set_status(&app, crate::state::ConnectionStatus::Disconnected);
            crate::tray::update_tray(&app);
            return Err(e);
        }
    }

    // Set connection global states
    if let Ok(mut mode) = state.effective_proxy_mode.lock() {
        *mode = Some(current_mode.clone());
    }
    state.set_status(&app, crate::state::ConnectionStatus::Connected);
    *CONNECTION_START_TIME.lock().unwrap() = Some(std::time::Instant::now());

    // Sync native system tray
    crate::tray::update_tray(&app);

    Ok("started".into())
}

#[tauri::command]
pub async fn get_active_connections(
    state: State<'_, crate::state::ProxyState>,
) -> Result<serde_json::Value, String> {
    let settings = state.get_settings();
    if !state.is_running() || !settings.api_enabled || settings.api_secret.trim().is_empty() {
        return Ok(serde_json::json!({ "connections": [] }));
    }
    let api_port = settings.api_port;
    let api_secret = settings.api_secret.clone();
    let url = format!("http://127.0.0.1:{}/connections", api_port);

    let client = reqwest::Client::new();
    let req = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_secret));

    match req.send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                Ok(json)
            } else {
                Err("Failed to parse connections JSON".to_string())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn close_connection(
    id: String,
    state: State<'_, crate::state::ProxyState>,
) -> Result<(), String> {
    let settings = state.get_settings();
    if !state.is_running() || !settings.api_enabled || settings.api_secret.trim().is_empty() {
        return Ok(());
    }
    let api_port = settings.api_port;
    let api_secret = settings.api_secret.clone();
    let url = format!("http://127.0.0.1:{}/connections/{}", api_port, id);

    let client = reqwest::Client::new();
    let req = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", api_secret));

    match req.send().await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn close_all_connections(
    state: State<'_, crate::state::ProxyState>,
) -> Result<(), String> {
    let settings = state.get_settings();
    if !state.is_running() || !settings.api_enabled || settings.api_secret.trim().is_empty() {
        return Ok(());
    }
    let api_port = settings.api_port;
    let api_secret = settings.api_secret.clone();
    let url = format!("http://127.0.0.1:{}/connections", api_port);

    let client = reqwest::Client::new();
    let req = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", api_secret));

    match req.send().await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_config_for_fallback_dns_attempt_3() {
        let temp_dir = std::env::temp_dir();
        let config_path =
            temp_dir.join(format!("test_patch_fallback_{}.json", uuid::Uuid::new_v4()));

        let initial_config = serde_json::json!({
            "dns": {
                "servers": [
                    { "tag": "local-dns", "type": "local", "detour": "direct" },
                    { "tag": "remote-dns", "type": "https", "server": "https://1.1.1.1/dns-query", "detour": "proxy" }
                ],
                "rules": [
                    { "domain": ["zoomconnector.duckdns.org"], "server": "local-dns" }
                ]
            },
            "inbounds": [
                {
                    "type": "tun",
                    "tag": "tun-in",
                    "route_exclude_address": ["192.168.1.1/32"]
                }
            ],
            "outbounds": [
                {
                    "type": "vless",
                    "tag": "proxy-node",
                    "server": "zoomconnector.duckdns.org",
                    "server_port": 443
                },
                {
                    "type": "direct",
                    "tag": "direct"
                }
            ],
            "route": {
                "default_domain_resolver": "local-dns"
            }
        });

        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&initial_config).unwrap(),
        )
        .unwrap();

        // Run Attempt 3 (DNS Fallback)
        let result = patch_config_for_fallback(&config_path, 3);
        assert!(result.is_ok(), "patch_config_for_fallback should succeed");

        let patched_content = std::fs::read_to_string(&config_path).unwrap();
        let patched: serde_json::Value = serde_json::from_str(&patched_content).unwrap();

        let _ = std::fs::remove_file(&config_path);

        // 1. Verify fallback servers exist with modern format
        let servers = patched["dns"]["servers"].as_array().unwrap();
        let fb1 = servers
            .iter()
            .find(|s| s["tag"] == "fallback-bootstrap-1")
            .unwrap();
        assert_eq!(fb1["type"], "udp");
        assert_eq!(fb1["server"], FALLBACK_DNS_PRIMARY);
        assert_eq!(fb1["server_port"], 53);
        assert_eq!(fb1["detour"], "direct");
        assert!(
            fb1.get("address").is_none(),
            "Must not use legacy address field"
        );

        // 2. Verify local-dns is updated to fallback public DNS
        let local_dns = servers.iter().find(|s| s["tag"] == "local-dns").unwrap();
        assert_eq!(local_dns["type"], "udp");
        assert_eq!(local_dns["server"], FALLBACK_DNS_PRIMARY);

        // 3. Verify domain rule points to fallback-bootstrap-1
        let rules = patched["dns"]["rules"].as_array().unwrap();
        assert_eq!(rules[0]["server"], "fallback-bootstrap-1");

        // 4. Verify route.default_domain_resolver is updated
        assert_eq!(
            patched["route"]["default_domain_resolver"],
            "fallback-bootstrap-1"
        );

        // 5. Verify outbound proxy node domain_resolver is updated
        let outbounds = patched["outbounds"].as_array().unwrap();
        let proxy_node = outbounds.iter().find(|o| o["tag"] == "proxy-node").unwrap();
        assert_eq!(proxy_node["domain_resolver"], "fallback-bootstrap-1");

        // 6. Verify route_exclude_address contains bootstrap DNS
        let inbounds = patched["inbounds"].as_array().unwrap();
        let tun = &inbounds[0];
        let excludes = tun["route_exclude_address"].as_array().unwrap();
        assert!(excludes
            .iter()
            .any(|e| e == &serde_json::json!(format!("{}/32", FALLBACK_DNS_PRIMARY))));
        assert!(excludes
            .iter()
            .any(|e| e == &serde_json::json!(format!("{}/32", FALLBACK_DNS_SECONDARY))));
    }

    #[test]
    fn test_clash_api_config_disabled() {
        assert!(build_clash_api_config(false, 9090, "secret123", false).is_none());
        assert!(build_clash_api_config(false, 9090, "secret123", true).is_none());
        assert!(build_clash_api_config(false, 9090, "", false).is_none());
    }

    #[test]
    fn test_clash_api_config_empty_or_missing_secret() {
        // Controller must never start enabled without authentication
        assert!(build_clash_api_config(true, 9090, "", false).is_none());
        assert!(build_clash_api_config(true, 9090, "   ", false).is_none());
        assert!(build_clash_api_config(true, 9090, "", true).is_none());
    }

    #[test]
    fn test_clash_api_config_enabled_with_auth() {
        let config = build_clash_api_config(true, 9090, "sec-token-xyz", false).unwrap();
        assert_eq!(config["external_controller"], "127.0.0.1:9090");
        assert_eq!(config["secret"], "sec-token-xyz");
    }

    #[test]
    fn test_clash_api_config_origin_setting() {
        // When api_cors is false: wildcard * must not be present or permitted
        let no_cors = build_clash_api_config(true, 9090, "sec-token-xyz", false).unwrap();
        assert!(
            no_cors.get("access_control_allow_origin").is_none(),
            "CORS wildcard must not be default/permitted when api_cors is false"
        );

        // When api_cors is true: explicitly allows wildcard
        let with_cors = build_clash_api_config(true, 9090, "sec-token-xyz", true).unwrap();
        let origins = with_cors["access_control_allow_origin"].as_array().unwrap();
        assert_eq!(origins, &vec![serde_json::json!("*")]);
    }
}
