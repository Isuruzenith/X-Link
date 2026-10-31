use std::collections::{HashMap, HashSet};
use std::time::SystemTime;
use tauri::{Emitter, Manager};

const RETRY_DELAYS_MS: [u64; 4] = [5 * 60_000, 15 * 60_000, 30 * 60_000, 60 * 60_000];

#[derive(Default)]
pub struct SubscriptionRetryState {
    failures: HashMap<String, (u64, usize)>,
}

impl SubscriptionRetryState {
    pub fn ready(&self, profile_id: &str, now_ms: u64) -> bool {
        self.failures
            .get(profile_id)
            .is_none_or(|(retry_at, _)| now_ms >= *retry_at)
    }

    pub fn record_failure(&mut self, profile_id: &str, now_ms: u64) {
        let count = self.failures.get(profile_id).map_or(0, |(_, count)| *count);
        let delay = RETRY_DELAYS_MS[count.min(RETRY_DELAYS_MS.len() - 1)];
        self.failures.insert(
            profile_id.to_string(),
            (now_ms.saturating_add(delay), count.saturating_add(1)),
        );
    }

    pub fn record_success(&mut self, profile_id: &str) {
        self.failures.remove(profile_id);
    }

    pub fn clear(&mut self) {
        self.failures.clear();
    }

    pub fn retain_profiles(&mut self, ids: impl IntoIterator<Item = String>) {
        let ids: HashSet<String> = ids.into_iter().collect();
        self.failures.retain(|id, _| ids.contains(id));
    }
}

pub async fn run_due_updates(
    app: tauri::AppHandle,
    retries: &mut SubscriptionRetryState,
) -> Result<Vec<String>, String> {
    let mut profiles_path = app.path().app_data_dir().map_err(|e| e.to_string())?;
    profiles_path.push("profiles.json");
    if !profiles_path.exists() {
        retries.clear();
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&profiles_path)
        .map_err(|e| format!("Failed to read profiles: {e}"))?;
    let profiles: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse profiles: {e}"))?;
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let all_ids = profiles
        .get("profiles")
        .and_then(|p| p.as_array())
        .into_iter()
        .flat_map(|profiles| profiles.iter())
        .filter_map(|p| p.get("id")?.as_str().map(str::to_string));
    retries.retain_profiles(all_ids);
    let due = crate::commands::config::find_due_subscription_profiles(&profiles, now_ms);
    let mut refreshed = Vec::new();
    for (profile_id, profile_name) in due {
        if !retries.ready(&profile_id, now_ms) {
            continue;
        }
        // refresh_subscription serializes native manual and scheduled refreshes under its lock.
        match crate::commands::config::refresh_subscription(app.clone(), profile_id.clone()).await {
            Ok(meta) => {
                retries.record_success(&profile_id);
                let _ = app.emit(
                    "subscription-auto-updated",
                    serde_json::json!({
                        "profileId": profile_id,
                        "profileName": meta.name,
                        "nodeCount": meta.node_count,
                    }),
                );
                refreshed.push(profile_id);
            }
            Err(_) => {
                retries.record_failure(&profile_id, now_ms);
                // Never publish a URL or token contained in a native error to frontend logs.
                let _ = app.emit(
                    "subscription-update-error",
                    serde_json::json!({
                        "profileId": profile_id,
                        "profileName": profile_name,
                        "error": "Subscription refresh failed; will retry later",
                    }),
                );
                log::warn!("Scheduled subscription update failed; retry delayed");
            }
        }
    }
    Ok(refreshed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_refresh_uses_bounded_backoff_and_success_resets_it() {
        let mut state = SubscriptionRetryState::default();
        assert!(state.ready("one", 0));
        state.record_failure("one", 100);
        assert!(!state.ready("one", 300_099));
        assert!(state.ready("one", 300_100));
        state.record_failure("one", 300_100);
        assert!(!state.ready("one", 1_200_099));
        assert!(state.ready("one", 1_200_100));
        for i in 0..5 {
            state.record_failure("one", 2_000_000 + i * 4_000_000);
        }
        assert!(!state.ready("one", 18_000_000 + 3_599_999));
        assert!(state.ready("one", 18_000_000 + 3_600_000));
        state.record_success("one");
        assert!(state.ready("one", 0));
    }

    #[test]
    fn retries_are_per_profile_and_deleted_profiles_are_removed() {
        let mut state = SubscriptionRetryState::default();
        state.record_failure("one", 0);
        state.record_failure("two", 0);
        assert!(state.ready("three", 0));
        state.retain_profiles(["two".to_string()]);
        assert!(state.ready("one", 0));
        assert!(!state.ready("two", 0));
    }
}
