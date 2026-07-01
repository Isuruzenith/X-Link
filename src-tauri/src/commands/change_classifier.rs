#[allow(dead_code)]
pub enum ReloadStrategy {
    /// Patch in-memory via Clash API — no restart
    HotReload,
    /// Full stop + start, re-apply OS proxy
    FullRestart,
}

#[allow(dead_code)]
pub fn classify_change(old: &serde_json::Value, new: &serde_json::Value) -> ReloadStrategy {
    let inbounds_changed = old.get("inbounds") != new.get("inbounds");

    if inbounds_changed {
        // Port change or TUN mode flip — must do a full restart
        return ReloadStrategy::FullRestart;
    }

    // Route, outbound, DNS changes can be hot-reloaded via Clash API
    ReloadStrategy::HotReload
}
