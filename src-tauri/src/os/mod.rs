pub fn enable_system_proxy(host: &str, port: u16) -> Result<(), String> {
    let bypass = "localhost;127.*;10.*;172.16.*;192.168.*".to_string();
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
