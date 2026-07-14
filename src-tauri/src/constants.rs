// === Connection Self-Healing ===
pub const MAX_FALLBACK_ATTEMPTS: usize = 5;
pub const MAX_CONFIG_PATCH_ATTEMPTS: usize = 3;
pub const FALLBACK_RETRY_DELAY_MS: u64 = 500;

// === Health Check ===
pub const HEALTH_CHECK_URL: &str = "https://www.gstatic.com/generate_204";
pub const HEALTH_CHECK_TIMEOUT_SECS: u64 = 10;

// === Sidecar Lifecycle ===
pub const STARTUP_WAIT_MS: u64 = 1500;
pub const SOCKET_RELEASE_DELAY_WINDOWS_MS: u64 = 600;
pub const SOCKET_RELEASE_DELAY_OTHER_MS: u64 = 300;
pub const STALE_PROCESS_KILL_DELAY_MS: u64 = 200;
pub const SHUTDOWN_GRACE_PERIOD_MS: u64 = 600;

// === Traffic Monitor ===
pub const TRAFFIC_POLL_INTERVAL_SECS: u64 = 1;
pub const TRAFFIC_MAX_CONSECUTIVE_FAILURES: u32 = 5;

// === Fallback DNS Bootstrap ===
pub const FALLBACK_DNS_PRIMARY: &str = "1.1.1.1";
pub const FALLBACK_DNS_SECONDARY: &str = "8.8.8.8";
