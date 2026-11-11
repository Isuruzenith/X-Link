// === Health Check ===
pub const HEALTH_CHECK_URL: &str = "https://www.gstatic.com/generate_204";
pub const HEALTH_CHECK_FALLBACK_URL: &str = "https://cp.cloudflare.com/generate_204";
pub const HEALTH_CHECK_TIMEOUT_SECS: u64 = 10;

// === Sidecar Lifecycle ===
pub const STARTUP_WAIT_MS: u64 = 1500;
pub const STALE_PROCESS_KILL_DELAY_MS: u64 = 200;
pub const SHUTDOWN_GRACE_PERIOD_MS: u64 = 600;

// === Traffic Monitor ===
pub const TRAFFIC_POLL_INTERVAL_SECS: u64 = 1;
pub const TRAFFIC_MAX_CONSECUTIVE_FAILURES: u32 = 5;

