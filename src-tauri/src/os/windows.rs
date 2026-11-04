use crate::config::generator::TunSettings;
use crate::os::{PlatformNetworkManager, TunSupportStatus};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::AppHandle;

pub struct WindowsNetworkManager;

impl WindowsNetworkManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsNetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn find_wintun_dll() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("wintun.dll"));
            candidates.push(dir.join("binaries").join("wintun.dll"));
            if let Some(parent) = dir.parent() {
                candidates.push(parent.join("binaries").join("wintun.dll"));
                if let Some(grandparent) = parent.parent() {
                    candidates.push(grandparent.join("binaries").join("wintun.dll"));
                }
            }
        }
    }
    candidates.push(PathBuf::from("binaries/wintun.dll"));
    candidates.push(PathBuf::from("src-tauri/binaries/wintun.dll"));
    candidates.into_iter().find(|p| p.exists())
}

/// Assigns a spawned sidecar process to a Windows Job Object configured with
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` so the kernel terminates `sing-box`
/// immediately if X-Link crashes or exits unexpectedly.
pub fn assign_process_to_job_object(pid: u32) -> Result<(), String> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    static JOB_HANDLE: OnceLock<isize> = OnceLock::new();

    let job_raw = *JOB_HANDLE.get_or_init(|| unsafe {
        let Ok(job) = CreateJobObjectW(None, None) else {
            return 0;
        };
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let _ = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        job.0 as isize
    });

    if job_raw == 0 {
        return Err("Failed to initialize Windows Job Object".to_string());
    }

    unsafe {
        let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
            .map_err(|e| format!("OpenProcess for Job Object failed: {}", e))?;
        let job = HANDLE(job_raw as *mut std::ffi::c_void);
        let assign_res = AssignProcessToJobObject(job, process);
        let _ = CloseHandle(process);
        assign_res.map_err(|e| format!("AssignProcessToJobObject failed: {}", e))
    }
}

pub fn discover_windows_dns_servers() -> Vec<IpAddr> {
    use std::net::{Ipv4Addr, Ipv6Addr};
    use windows::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST,
        GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6,
    };

    unsafe {
        let flags = GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST;
        let mut buf_len: u32 = 16384;
        let mut buffer: Vec<u8> = vec![0u8; buf_len as usize];

        let mut ret = GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            flags,
            None,
            Some(buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
            &mut buf_len,
        );

        if ret == ERROR_BUFFER_OVERFLOW.0 {
            buffer.resize(buf_len as usize, 0);
            ret = GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                flags,
                None,
                Some(buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut buf_len,
            );
        }

        if ret != 0 {
            return Vec::new();
        }

        let mut adapters = Vec::new();
        let mut curr_ptr = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;

        while !curr_ptr.is_null() {
            let entry = &*curr_ptr;

            let name = if !entry.AdapterName.0.is_null() {
                windows::core::PCSTR(entry.AdapterName.0)
                    .to_string()
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let description = if !entry.Description.0.is_null() {
                entry.Description.to_string().unwrap_or_default()
            } else {
                String::new()
            };

            let if_type = entry.IfType;
            let is_physical = if_type == 6 || if_type == 71;
            let is_operational = entry.OperStatus.0 == 1;

            let mut has_ipv4_gateway = false;
            let mut has_ipv6_gateway = false;
            let mut gw_ptr = entry.FirstGatewayAddress;
            while !gw_ptr.is_null() {
                let gw_entry = &*gw_ptr;
                let sock_ptr = gw_entry.Address.lpSockaddr;
                if !sock_ptr.is_null() {
                    let family = (*sock_ptr).sa_family;
                    if family == AF_INET {
                        has_ipv4_gateway = true;
                    } else if family == AF_INET6 {
                        has_ipv6_gateway = true;
                    }
                }
                gw_ptr = gw_entry.Next;
            }

            let mut dns_servers = Vec::new();
            let mut dns_ptr = entry.FirstDnsServerAddress;
            while !dns_ptr.is_null() {
                let dns_entry = &*dns_ptr;
                let sock_ptr = dns_entry.Address.lpSockaddr;
                if !sock_ptr.is_null() {
                    let family = (*sock_ptr).sa_family;
                    if family == AF_INET {
                        let sin = &*(sock_ptr as *const SOCKADDR_IN);
                        let ip_bytes = sin.sin_addr.S_un.S_addr.to_ne_bytes();
                        dns_servers.push(IpAddr::V4(Ipv4Addr::from(ip_bytes)));
                    } else if family == AF_INET6 {
                        let sin6 = &*(sock_ptr as *const SOCKADDR_IN6);
                        let ip_bytes = sin6.sin6_addr.u.Byte;
                        dns_servers.push(IpAddr::V6(Ipv6Addr::from(ip_bytes)));
                    }
                }
                dns_ptr = dns_entry.Next;
            }

            adapters.push(crate::config::DetectedAdapter {
                name,
                description,
                is_physical,
                is_operational,
                has_ipv4_gateway,
                has_ipv6_gateway,
                dns_servers,
            });

            curr_ptr = entry.Next;
        }

        let mut results = Vec::new();
        if let Some(best) = crate::config::select_best_dns_server(&adapters) {
            results.push(best);
        }
        for adapter in &adapters {
            if adapter.is_operational
                && !adapter.name.to_lowercase().contains("x-link")
                && !adapter.description.to_lowercase().contains("wintun")
            {
                for ip in &adapter.dns_servers {
                    if !ip.is_loopback() && !ip.is_unspecified() && !results.contains(ip) {
                        results.push(*ip);
                    }
                }
            }
        }
        results
    }
}

#[async_trait::async_trait]
impl PlatformNetworkManager for WindowsNetworkManager {
    async fn configure_system_proxy(&self, port: u16, _bypass_local: bool) -> Result<(), String> {
        crate::os::enable_system_proxy_internal("127.0.0.1", port)
    }

    async fn restore_system_proxy(&self) -> Result<(), String> {
        self.restore_system_proxy_sync()
    }

    fn restore_system_proxy_sync(&self) -> Result<(), String> {
        crate::os::disable_system_proxy_internal()
    }

    async fn check_tun_support(&self) -> Result<TunSupportStatus, String> {
        let driver_present = find_wintun_dll().is_some();
        let elevated = crate::os::is_elevated();
        Ok(crate::os::resolve_windows_tun_status(
            elevated,
            driver_present,
        ))
    }

    async fn provision_tun_interface(&self, _settings: &TunSettings) -> Result<String, String> {
        if find_wintun_dll().is_none() {
            return Err("wintun.dll driver is missing from application binaries".to_string());
        }
        if !crate::os::is_elevated() {
            return Err("Administrator privileges are required for Windows TUN mode".to_string());
        }
        Ok("X-Link".to_string())
    }

    async fn request_platform_elevation(&self, app_handle: &AppHandle) -> Result<(), String> {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

        let exe_path =
            std::env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;

        let mut verb: Vec<u16> = std::ffi::OsStr::new("runas").encode_wide().collect();
        verb.push(0);

        let mut file: Vec<u16> = exe_path.as_os_str().encode_wide().collect();
        file.push(0);

        let handoff = format!("--elevation-handoff={}", std::process::id());
        let mut params: Vec<u16> = std::ffi::OsStr::new(&handoff).encode_wide().collect();
        params.push(0);

        unsafe {
            let result = ShellExecuteW(
                None,
                PCWSTR(verb.as_ptr()),
                PCWSTR(file.as_ptr()),
                PCWSTR(params.as_ptr()),
                None,
                SW_SHOW,
            );

            if result.0 as isize <= 32 {
                return Err("Elevation request denied or failed".to_string());
            }
        }

        crate::tray::perform_clean_cleanup(app_handle);
        std::process::exit(0);
    }

    async fn get_system_dns_servers(&self) -> Result<Vec<IpAddr>, String> {
        Ok(discover_windows_dns_servers())
    }
}
