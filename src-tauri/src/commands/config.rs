use crate::commands::proxy::load_tun_settings;
use crate::config::adapters::adapt;
use crate::config::generator::generate_singbox_config;
use crate::config::validator::validate_singbox_config;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};
use tauri::{Emitter, Manager, State};

pub(crate) static SUBSCRIPTION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct SubscriptionSnapshot {
    profile_id: String,
    profiles_content: String,
    current_config: String,
    outbounds: Vec<crate::config::adapters::SingBoxOutbound>,
    created_at: Instant,
}

fn subscription_snapshots() -> &'static Mutex<Option<(String, SubscriptionSnapshot)>> {
    static SNAPSHOTS: OnceLock<Mutex<Option<(String, SubscriptionSnapshot)>>> = OnceLock::new();
    SNAPSHOTS.get_or_init(|| Mutex::new(None))
}

fn take_subscription_snapshot(
    token: &str,
    profile_id: &str,
    profiles_content: &str,
    current_config_path: &std::path::Path,
) -> Result<Vec<crate::config::adapters::SingBoxOutbound>, String> {
    let mut guard = subscription_snapshots()
        .lock()
        .map_err(|_| "Subscription preview state is unavailable".to_string())?;
    if guard
        .as_ref()
        .is_none_or(|(stored_token, _)| stored_token != token)
    {
        return Err("Subscription preview expired. Preview the update again.".to_string());
    }
    let (_, snapshot) = guard.take().expect("checked preview token");
    drop(guard);
    if snapshot.created_at.elapsed() > Duration::from_secs(300)
        || snapshot.profile_id != profile_id
        || snapshot.profiles_content != profiles_content
        || std::fs::read_to_string(current_config_path).ok().as_deref()
            != Some(snapshot.current_config.as_str())
    {
        return Err("Subscription changed since preview. Preview the update again.".to_string());
    }
    Ok(snapshot.outbounds)
}

/// Metadata returned to the frontend when importing a profile.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMeta {
    pub id: String,
    pub name: String,
    pub imported_at: u64,
    pub node_count: u32,
}

/// The single, fixed "active" config file path: <app_data_dir>/configs/active.json
/// This is always the file sing-box boots from.
pub fn get_active_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    path.push("configs");
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create configs directory: {}", e))?;
    path.push("active.json");
    Ok(path)
}

/// Per-profile config file: <app_data_dir>/configs/{profile_id}.json
pub fn get_profile_config_path(
    app: &tauri::AppHandle,
    profile_id: &str,
) -> Result<PathBuf, String> {
    if !crate::commands::backup::is_valid_profile_id(profile_id) {
        return Err("Invalid profile ID".to_string());
    }
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    path.push("configs");
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create configs directory: {}", e))?;
    path.push(format!("{}.json", profile_id));
    Ok(path)
}

/// Scratch file used to validate a candidate config before committing it.
fn get_temp_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    path.push("configs");
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create configs directory: {}", e))?;
    path.push("active_temp.json");
    Ok(path)
}

/// Files staged beside their destination before a multi-file config update.
struct StagedFile {
    target: PathBuf,
    temporary: PathBuf,
    backup: PathBuf,
    had_original: bool,
    committed: bool,
}

/// Stages related files beside their destinations and restores earlier files if a commit fails.
fn commit_files_transactionally(files: Vec<(PathBuf, Vec<u8>)>) -> Result<(), String> {
    commit_files_transactionally_with(files, |from, to| std::fs::rename(from, to))
}

fn commit_files_transactionally_with<F>(
    files: Vec<(PathBuf, Vec<u8>)>,
    mut rename: F,
) -> Result<(), String>
where
    F: FnMut(&std::path::Path, &std::path::Path) -> std::io::Result<()>,
{
    if files
        .iter()
        .map(|(path, _)| path)
        .collect::<std::collections::HashSet<_>>()
        .len()
        != files.len()
    {
        return Err("Config transaction contains duplicate destination paths".to_string());
    }
    let mut staged = Vec::with_capacity(files.len());
    for (target, contents) in files {
        let name = target
            .file_name()
            .ok_or_else(|| "Invalid transaction target path".to_string())?;
        let suffix = uuid::Uuid::new_v4();
        let temporary = target.with_file_name(format!("{}.{}.tmp", name.to_string_lossy(), suffix));
        let backup = target.with_file_name(format!("{}.{}.bak", name.to_string_lossy(), suffix));
        if let Err(error) = std::fs::write(&temporary, contents) {
            cleanup_staged_files(&staged);
            return Err(format!("Failed to stage {}: {}", target.display(), error));
        }
        let had_original = target.exists();
        if had_original {
            if let Err(error) = std::fs::copy(&target, &backup) {
                let _ = std::fs::remove_file(&temporary);
                cleanup_staged_files(&staged);
                return Err(format!("Failed to preserve previous file: {}", error));
            }
        }
        staged.push(StagedFile {
            had_original,
            target,
            temporary,
            backup,
            committed: false,
        });
    }

    for index in 0..staged.len() {
        if let Err(error) = rename(&staged[index].temporary, &staged[index].target) {
            let rollback_errors = rollback_staged_files(&mut staged[..index]);
            cleanup_staged_files(&staged[index..]);
            let rollback_detail = if rollback_errors.is_empty() {
                String::new()
            } else {
                format!("; rollback incomplete: {}", rollback_errors.join("; "))
            };
            return Err(format!(
                "Failed to commit file: {}{}",
                error, rollback_detail
            ));
        }
        staged[index].committed = true;
    }

    // Backup cleanup is housekeeping: a failure leaves a recoverable backup but does not undo
    // files that have already been committed successfully.
    cleanup_backup_files(&staged);
    Ok(())
}

fn rollback_staged_files(files: &mut [StagedFile]) -> Vec<String> {
    let mut errors = Vec::new();
    for file in files.iter_mut().rev() {
        if file.committed {
            if let Err(error) = std::fs::remove_file(&file.target) {
                errors.push(format!("remove {}: {}", file.target.display(), error));
                continue;
            }
        }
        if file.had_original {
            if let Err(error) = std::fs::rename(&file.backup, &file.target) {
                errors.push(format!("restore {}: {}", file.target.display(), error));
            }
        }
        if let Err(error) = std::fs::remove_file(&file.temporary) {
            if error.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!(
                    "remove staged {}: {}",
                    file.temporary.display(),
                    error
                ));
            }
        }
    }
    errors
}

fn cleanup_staged_files(files: &[StagedFile]) {
    for file in files {
        let _ = std::fs::remove_file(&file.temporary);
        if !file.committed {
            let _ = std::fs::remove_file(&file.backup);
        }
    }
}

fn cleanup_backup_files(files: &[StagedFile]) {
    for file in files.iter().filter(|file| file.had_original) {
        let _ = std::fs::remove_file(&file.backup);
    }
}

async fn commit_and_apply_with_rollback<F, Fut, C>(
    files: Vec<(PathBuf, Vec<u8>)>,
    connected: bool,
    mut apply: F,
    previous_connection_running: C,
) -> Result<(), String>
where
    F: FnMut(bool) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
    C: Fn() -> bool,
{
    let previous = if connected {
        files
            .iter()
            .map(|(path, _)| {
                std::fs::read(path)
                    .map(|contents| (path.clone(), contents))
                    .map_err(|error| format!("Failed to preserve previous config: {}", error))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    commit_files_transactionally(files)?;
    if !connected {
        return Ok(());
    }

    let error = match apply(false).await {
        Ok(()) if previous_connection_running() => return Ok(()),
        Ok(()) => {
            "Configuration apply reported success but the connection is not running".to_string()
        }
        Err(error) => error,
    };
    commit_files_transactionally(previous)
        .map_err(|rollback| format!("{}; previous config restore failed: {}", error, rollback))?;
    if !previous_connection_running() {
        apply(true).await.map_err(|recovery| {
            format!(
                "{}; previous config was restored but connection recovery failed: {}",
                error, recovery
            )
        })?;
        if !previous_connection_running() {
            return Err(format!(
                "{}; previous config was restored but connection recovery did not reconnect",
                error
            ));
        }
    }
    Err(format!(
        "Configuration apply failed; previous files were restored and the connection is running: {}",
        error
    ))
}

fn remove_files_transactionally(paths: Vec<PathBuf>) -> Result<(), String> {
    remove_files_transactionally_with(paths, |from, to| std::fs::rename(from, to))
}

fn remove_files_transactionally_with<F>(paths: Vec<PathBuf>, mut rename: F) -> Result<(), String>
where
    F: FnMut(&std::path::Path, &std::path::Path) -> std::io::Result<()>,
{
    if paths.iter().collect::<std::collections::HashSet<_>>().len() != paths.len() {
        return Err("File removal transaction contains duplicate paths".to_string());
    }

    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for path in paths.into_iter().filter(|path| path.exists()) {
        let name = path
            .file_name()
            .ok_or_else(|| "Invalid file removal path".to_string())?;
        let backup = path.with_file_name(format!(
            "{}.{}.delete",
            name.to_string_lossy(),
            uuid::Uuid::new_v4()
        ));
        if let Err(error) = rename(&path, &backup) {
            let mut rollback_errors = Vec::new();
            for (original, saved) in moved.iter().rev() {
                if let Err(restore_error) = std::fs::rename(saved, original) {
                    rollback_errors.push(format!(
                        "restore {}: {}",
                        original.display(),
                        restore_error
                    ));
                }
            }
            let detail = if rollback_errors.is_empty() {
                String::new()
            } else {
                format!("; rollback incomplete: {}", rollback_errors.join("; "))
            };
            return Err(format!(
                "Failed to stage deletion of {}: {}{}",
                path.display(),
                error,
                detail
            ));
        }
        moved.push((path, backup));
    }

    // Once every target has been moved out of place, deletion is committed. Backup cleanup is
    // best-effort so a cleanup error cannot turn a successful removal into a misleading failure.
    for (_, backup) in moved {
        let _ = std::fs::remove_file(backup);
    }
    Ok(())
}

/// Reads proxyMode / dnsAddress / wifiSharing from settings.json (best-effort).
fn read_basic_settings(app: &tauri::AppHandle) -> (String, Option<String>, bool) {
    let mut proxy_mode = "system".to_string();
    let mut dns: Option<String> = None;
    let mut wifi = false;
    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("settings.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(m) = val.get("proxyMode").and_then(|v| v.as_str()) {
                        proxy_mode = m.to_string();
                    }
                    if let Some(d) = val.get("dnsAddress").and_then(|v| v.as_str()) {
                        if !d.trim().is_empty() {
                            dns = Some(d.to_string());
                        }
                    }
                    if let Some(w) = val.get("wifiSharing").and_then(|v| v.as_bool()) {
                        wifi = w;
                    }
                }
            }
        }
    }
    (proxy_mode, dns, wifi)
}

fn sanitize_subscription_outbounds(
    mut outbounds: Vec<crate::config::adapters::SingBoxOutbound>,
    outbound_groups: &[crate::config::rules::OutboundGroup],
) -> Vec<crate::config::adapters::SingBoxOutbound> {
    let group_tags: std::collections::HashSet<&str> =
        outbound_groups.iter().map(|g| g.tag.as_str()).collect();
    for ob in &mut outbounds {
        let lower = ob.tag.trim().to_ascii_lowercase();
        let is_reserved = matches!(lower.as_str(), "proxy" | "direct" | "block" | "dns");
        if is_reserved || group_tags.contains(ob.tag.as_str()) {
            ob.tag = format!("{}_node", ob.tag.trim());
        }
    }
    outbounds
}

/// Imports a config from raw pasted text/clipboard content, or from a local file path.
/// Always writes to the single fixed "active" config slot -- importing a new config
/// overwrites whatever was previously active, there is no profile list to manage.
#[tauri::command]
pub async fn import_config(
    app: tauri::AppHandle,
    state: State<'_, crate::state::ProxyState>,
    content: Option<String>,
    file_path: Option<String>,
    name: String,
    profile_id: Option<String>,
) -> Result<ProfileMeta, String> {
    // 1. Read raw bytes from whichever source was provided
    let bytes: Vec<u8> = if let Some(fp) = file_path {
        std::fs::read(&fp).map_err(|e| format!("Failed to read file: {}", e))?
    } else if let Some(c) = content {
        let trimmed = c.trim();
        if trimmed.starts_with("http://") {
            return Err(
                "Insecure HTTP subscription URLs are not supported. Only HTTPS is allowed for security."
                    .to_string(),
            );
        } else if trimmed.starts_with("https://") {
            // Native SSRF-safe bounded fetch for HTTPS subscription URLs
            crate::config::subscription::fetch_subscription_content(trimmed)
                .await
                .map_err(|e| format!("Failed to fetch subscription: {}", e))?
        } else if trimmed.starts_with("ftp://") || trimmed.starts_with("file://") {
            return Err(
                "Unsupported URL scheme. Only HTTPS subscription URLs are supported.".to_string(),
            );
        } else {
            c.into_bytes()
        }
    } else {
        return Err("No content or file path provided".to_string());
    };

    // 2. Adapt configuration via the established format-detection engine
    //    (raw URIs, Clash YAML, sing-box JSON, base64 subscription blobs, etc.)
    let outbound_groups = crate::config::rules::load_outbound_groups_from_file(&app);
    let outbounds = sanitize_subscription_outbounds(adapt(&bytes)?, &outbound_groups);
    if outbounds.is_empty() {
        return Err("Import contains no usable proxy nodes.".to_string());
    }
    let node_count = outbounds.len() as u32;

    // 3. Generate sing-box JSON config based on active settings.json
    let (proxy_mode, dns, wifi) = read_basic_settings(&app);
    let listen_address = if wifi {
        "0.0.0.0".to_string()
    } else {
        "127.0.0.1".to_string()
    };
    let dns_address = crate::config::resolve_dns_address(dns.as_deref());
    let tun_settings = load_tun_settings(&app);
    let mixed_port = state.get_settings().mixed_port;
    let (user_rules, rule_sets) = crate::config::rules::load_routing_rules_from_file(&app);
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let host_dns = crate::os::get_network_manager()
        .get_system_dns_servers()
        .await
        .unwrap_or_default();

    let generated_config = generate_singbox_config(
        mixed_port,
        outbounds,
        &proxy_mode,
        &dns_address,
        &listen_address,
        &tun_settings,
        None,
        &user_rules,
        &rule_sets,
        &app_data_dir,
        &outbound_groups,
        &host_dns,
    )?;

    // 4. Early guard: validate generated JSON has required structure before
    //    invoking the expensive sing-box binary validator
    {
        let parsed: serde_json::Value = serde_json::from_str(&generated_config)
            .map_err(|e| format!("Generated config is not valid JSON: {}", e))?;
        if parsed
            .get("outbounds")
            .and_then(|o| o.as_array())
            .is_none_or(|a| a.is_empty())
        {
            return Err("Config has no outbounds — nothing to connect to.".to_string());
        }
    }

    // 5. Write to a temporary file and validate with the real sing-box binary
    //    before committing -- this guarantees we never overwrite a working
    //    config with a broken one.
    let temp_path = get_temp_config_path(&app)?;
    std::fs::write(&temp_path, &generated_config)
        .map_err(|e| format!("Failed to write temp config: {}", e))?;

    let validation_res = validate_singbox_config(&app, &temp_path).await;
    let _ = std::fs::remove_file(&temp_path);
    validation_res?;

    // 6. Determine the profile ID — use provided one, or generate a new UUID
    let pid = profile_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // 7. Write to per-profile config AND the active slot
    let profile_path = get_profile_config_path(&app, &pid)?;
    let active_path = get_active_config_path(&app)?;
    commit_files_transactionally(vec![
        (profile_path, generated_config.as_bytes().to_vec()),
        (active_path, generated_config.as_bytes().to_vec()),
    ])?;

    let imported_at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    Ok(ProfileMeta {
        id: pid,
        name,
        imported_at,
        node_count,
    })
}

/// Refreshes a subscription profile in-place from its remote HTTPS subscription URL.
/// Retains profile ID, name, URL, and import origin.
/// Updates nodes, count, and lastUpdated only after fetch, parse, config generation,
/// and real sing-box validation succeed.
/// Preserves the selected node tag if it still exists; otherwise selects a valid fallback.
/// Refreshing a non-active profile does not overwrite active.json or interrupt the active connection.
/// Refreshing an active connected profile uses the controlled validated cold-restart path and restores
/// the previous files and connection if applying the refreshed profile fails.
/// On any failure (fetch, parse, validate), previous disk config and profile metadata remain intact.
#[tauri::command]
pub async fn refresh_subscription(
    app: tauri::AppHandle,
    profile_id: String,
) -> Result<ProfileMeta, String> {
    refresh_subscription_internal(app, profile_id, None).await
}

#[tauri::command]
pub async fn apply_previewed_subscription_refresh(
    app: tauri::AppHandle,
    profile_id: String,
    preview_token: String,
) -> Result<ProfileMeta, String> {
    refresh_subscription_internal(app, profile_id, Some(preview_token)).await
}

async fn refresh_subscription_internal(
    app: tauri::AppHandle,
    profile_id: String,
    preview_token: Option<String>,
) -> Result<ProfileMeta, String> {
    let _lock = SUBSCRIPTION_LOCK.lock().await;
    if !crate::commands::backup::is_valid_profile_id(&profile_id) {
        return Err("Invalid profile ID".to_string());
    }
    // 1. Check whether the target profile is active and whether a live restart is required.
    let active_id = get_active_profile_id(&app)?;
    let is_active = active_id.as_deref() == Some(&profile_id);
    let state = app.state::<crate::state::ProxyState>();
    let is_connected =
        state.get_status() == crate::state::ConnectionStatus::Connected && state.is_running();
    let restart_connected_proxy = is_active && is_connected;
    let effective_mode = state
        .effective_proxy_mode
        .lock()
        .map_err(|error| format!("state_lock_poisoned: {}", error))?
        .clone();

    // 2. Read profiles.json to find profile and its subscriptionUrl
    let mut profiles_path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    profiles_path.push("profiles.json");

    if !profiles_path.exists() {
        return Err("profiles.json does not exist".to_string());
    }

    let profiles_content = std::fs::read_to_string(&profiles_path)
        .map_err(|e| format!("Failed to read profiles.json: {}", e))?;
    let mut profiles_json: serde_json::Value = serde_json::from_str(&profiles_content)
        .map_err(|e| format!("Failed to parse profiles.json: {}", e))?;

    let profiles_arr = profiles_json
        .get_mut("profiles")
        .and_then(|p| p.as_array_mut())
        .ok_or_else(|| "No profiles found in profiles.json".to_string())?;

    let profile_idx = profiles_arr
        .iter()
        .position(|p| p.get("id").and_then(|id| id.as_str()) == Some(&profile_id))
        .ok_or_else(|| format!("Profile not found with ID: {}", profile_id))?;

    let profile_val = &profiles_arr[profile_idx];
    let profile_name = profile_val
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("Subscription")
        .to_string();
    let profile_imported_at = profile_val
        .get("importedAt")
        .and_then(|i| i.as_u64())
        .unwrap_or(0);
    let existing_selected_tag = profile_val
        .get("selectedNodeTag")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());

    let sub_url = profile_val
        .get("subscriptionUrl")
        .and_then(|u| u.as_str())
        .ok_or_else(|| "Profile does not have a subscription URL".to_string())?
        .to_string();

    let outbound_groups = crate::config::rules::load_outbound_groups_from_file(&app);

    // A confirmed preview applies the exact fetched nodes, not a second remote response.
    let raw_outbounds = if let Some(token) = preview_token {
        let current_path = get_profile_config_path(&app, &profile_id)?;
        take_subscription_snapshot(&token, &profile_id, &profiles_content, &current_path)?
    } else {
        let bytes = crate::config::subscription::fetch_subscription_content(&sub_url).await?;
        adapt(&bytes)?
    };
    let outbounds = sanitize_subscription_outbounds(raw_outbounds, &outbound_groups);
    if outbounds.is_empty() {
        return Err("Subscription contains no usable proxy nodes.".to_string());
    }
    let node_count = outbounds.len() as u32;

    // 5. Generate candidate sing-box configuration
    let (saved_proxy_mode, dns, wifi) = read_basic_settings(&app);
    let proxy_mode = if restart_connected_proxy {
        effective_mode
            .as_deref()
            .unwrap_or(&saved_proxy_mode)
            .to_string()
    } else {
        saved_proxy_mode
    };
    let listen_address = if wifi {
        "0.0.0.0".to_string()
    } else {
        "127.0.0.1".to_string()
    };
    let dns_address = crate::config::resolve_dns_address(dns.as_deref());
    let mut tun_settings = load_tun_settings(&app);
    if restart_connected_proxy {
        if let Ok(settings) = state.settings.lock() {
            tun_settings.use_separate_ports = settings.use_separate_ports;
            tun_settings.http_port = settings.http_port;
            tun_settings.socks_port = settings.socks_port;
        }
    }
    let mixed_port = state.get_settings().mixed_port;
    let (user_rules, rule_sets) = crate::config::rules::load_routing_rules_from_file(&app);
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let host_dns = crate::os::get_network_manager()
        .get_system_dns_servers()
        .await
        .unwrap_or_default();

    let generated_config = generate_singbox_config(
        mixed_port,
        outbounds.clone(),
        &proxy_mode,
        &dns_address,
        &listen_address,
        &tun_settings,
        None,
        &user_rules,
        &rule_sets,
        &app_data_dir,
        &outbound_groups,
        &host_dns,
    )?;

    // 6. Early structural guard
    {
        let parsed: serde_json::Value = serde_json::from_str(&generated_config)
            .map_err(|e| format!("Generated config is not valid JSON: {}", e))?;
        if parsed
            .get("outbounds")
            .and_then(|o| o.as_array())
            .is_none_or(|a| a.is_empty())
        {
            return Err("Config has no outbounds — nothing to connect to.".to_string());
        }
    }

    // 7. Determine selected node tag: preserve existing tag if still present in nodes or outbound groups; otherwise fallback to first valid tag
    let valid_tags: Vec<String> = outbounds.iter().map(|o| o.tag.clone()).collect();
    let group_tags: Vec<String> = outbound_groups.iter().map(|g| g.tag.clone()).collect();

    let new_selected_tag: Option<String> = match existing_selected_tag.as_ref() {
        Some(tag) if valid_tags.contains(tag) || group_tags.contains(tag) => Some(tag.clone()),
        _ => valid_tags.first().cloned(),
    };

    let previous_selected_tag = if restart_connected_proxy {
        get_selected_outbound_tag(&app)
    } else {
        None
    };
    let mut final_config = generated_config;
    if let Some(ref tag) = new_selected_tag {
        if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&final_config) {
            if let Some(obs) = parsed.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
                for ob in obs.iter_mut() {
                    if ob.get("type").and_then(|t| t.as_str()) == Some("selector")
                        && ob.get("tag").and_then(|t| t.as_str()) == Some("proxy")
                    {
                        ob["default"] = serde_json::Value::String(tag.clone());
                    }
                }
            }
            if let Ok(s) = serde_json::to_string_pretty(&parsed) {
                final_config = s;
            }
        }
    }

    // Validate the exact final config, including the selected-node default, before committing.
    let temp_path = get_temp_config_path(&app)?;
    std::fs::write(&temp_path, &final_config)
        .map_err(|error| format!("Failed to write temporary refreshed config: {}", error))?;
    let validation = validate_singbox_config(&app, &temp_path).await;
    let _ = std::fs::remove_file(&temp_path);
    validation
        .map_err(|error| format!("Validation of refreshed configuration failed: {}", error))?;

    // 9. Prepare metadata and stage every file before changing the current profile/config.
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let target_profile = &mut profiles_arr[profile_idx];
    target_profile["nodeCount"] = serde_json::Value::Number(node_count.into());
    target_profile["lastUpdated"] = serde_json::Value::Number(now_ms.into());
    target_profile["selectedNodeTag"] = match &new_selected_tag {
        Some(t) => serde_json::Value::String(t.clone()),
        None => serde_json::Value::Null,
    };
    let updated_profiles = serde_json::to_vec_pretty(&profiles_json)
        .map_err(|e| format!("Failed to serialize profiles.json: {}", e))?;

    let profile_path = get_profile_config_path(&app, &profile_id)?;
    let mut files = vec![(profile_path, final_config.as_bytes().to_vec())];
    if is_active {
        files.push((
            get_active_config_path(&app)?,
            final_config.as_bytes().to_vec(),
        ));
    }
    files.push((profiles_path.clone(), updated_profiles));

    commit_and_apply_with_rollback(
        files,
        restart_connected_proxy,
        |restoring| {
            crate::commands::proxy::try_reload_proxy_config_with_mode(
                &app,
                if restoring {
                    previous_selected_tag.clone()
                } else {
                    new_selected_tag.clone()
                },
                effective_mode.clone(),
            )
        },
        || state.get_status() == crate::state::ConnectionStatus::Connected && state.is_running(),
    )
    .await?;

    Ok(ProfileMeta {
        id: profile_id,
        name: profile_name,
        imported_at: profile_imported_at,
        node_count,
    })
}

/// Evaluates a profiles JSON structure and returns list of (profile_id, profile_name) due for auto-update.
pub fn find_due_subscription_profiles(
    profiles_json: &serde_json::Value,
    now_ms: u64,
) -> Vec<(String, String)> {
    let profiles = match profiles_json.get("profiles").and_then(|p| p.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    let mut due = Vec::new();
    for p in profiles {
        let p_type = p.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if p_type != "subscription" {
            continue;
        }
        let sub_url = p
            .get("subscriptionUrl")
            .and_then(|u| u.as_str())
            .unwrap_or("");
        if sub_url.trim().is_empty() {
            continue;
        }
        let interval_mins = p
            .get("autoUpdateInterval")
            .and_then(|i| i.as_u64())
            .unwrap_or(0);
        if interval_mins == 0 {
            continue;
        }
        let last_updated = p.get("lastUpdated").and_then(|i| i.as_u64()).unwrap_or(0);
        let interval_ms = interval_mins.saturating_mul(60 * 1000);
        if now_ms.saturating_sub(last_updated) >= interval_ms {
            let id = p
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string();
            let name = p
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("Subscription")
                .to_string();
            if !id.is_empty() {
                due.push((id, name));
            }
        }
    }
    due
}

/// Background task helper to evaluate and refresh all subscription profiles due for auto-update.
#[tauri::command]
pub async fn run_subscription_auto_updates(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let mut profiles_path = match app.path().app_data_dir() {
        Ok(p) => p,
        Err(_) => return Ok(Vec::new()),
    };
    profiles_path.push("profiles.json");
    if !profiles_path.exists() {
        return Ok(Vec::new());
    }

    let content = match std::fs::read_to_string(&profiles_path) {
        Ok(c) => c,
        Err(_) => return Ok(Vec::new()),
    };
    let profiles_json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };

    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let due_profiles = find_due_subscription_profiles(&profiles_json, now_ms);
    let mut refreshed = Vec::new();

    for (profile_id, profile_name) in due_profiles {
        match refresh_subscription(app.clone(), profile_id.clone()).await {
            Ok(meta) => {
                let _ = app.emit(
                    "subscription-auto-updated",
                    serde_json::json!({
                        "profileId": profile_id,
                        "profileName": meta.name,
                        "nodeCount": meta.node_count
                    }),
                );
                refreshed.push(profile_id);
            }
            Err(err) => {
                let _ = app.emit(
                    "subscription-update-error",
                    serde_json::json!({
                        "profileId": profile_id,
                        "profileName": profile_name,
                        "error": err
                    }),
                );
            }
        }
    }

    Ok(refreshed)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDiffPreview {
    pub profile_id: String,
    pub profile_name: String,
    pub old_node_count: usize,
    pub new_node_count: usize,
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub retained_node_count: usize,
    pub current_selected_tag: Option<String>,
    pub is_selected_node_preserved: bool,
    pub fallback_selected_tag: Option<String>,
    pub preview_token: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub node_count: usize,
    pub node_types: std::collections::HashMap<String, usize>,
    pub sample_nodes: Vec<String>,
}

pub fn calculate_subscription_diff(
    profile_id: &str,
    profile_name: &str,
    current_tags: &[String],
    current_selected_tag: Option<&str>,
    new_tags: &[String],
) -> SubscriptionDiffPreview {
    let old_set: std::collections::HashSet<&str> =
        current_tags.iter().map(|s| s.as_str()).collect();
    let new_set: std::collections::HashSet<&str> = new_tags.iter().map(|s| s.as_str()).collect();

    let added_nodes: Vec<String> = new_tags
        .iter()
        .filter(|t| !old_set.contains(t.as_str()))
        .cloned()
        .collect();

    let removed_nodes: Vec<String> = current_tags
        .iter()
        .filter(|t| !new_set.contains(t.as_str()))
        .cloned()
        .collect();

    let retained_node_count = current_tags
        .iter()
        .filter(|t| new_set.contains(t.as_str()))
        .count();

    let is_selected_node_preserved = match current_selected_tag {
        Some(tag) => new_set.contains(tag),
        None => true,
    };

    let fallback_selected_tag = if !is_selected_node_preserved {
        new_tags.first().cloned()
    } else {
        current_selected_tag.map(|s| s.to_string())
    };

    SubscriptionDiffPreview {
        profile_id: profile_id.to_string(),
        profile_name: profile_name.to_string(),
        old_node_count: current_tags.len(),
        new_node_count: new_tags.len(),
        added_nodes,
        removed_nodes,
        retained_node_count,
        current_selected_tag: current_selected_tag.map(|s| s.to_string()),
        is_selected_node_preserved,
        fallback_selected_tag,
        preview_token: None,
    }
}

/// Previews the diff (added, removed, retained nodes) resulting from refreshing a subscription profile,
/// without modifying disk configurations.
#[tauri::command]
pub async fn preview_subscription_refresh(
    app: tauri::AppHandle,
    profile_id: String,
) -> Result<SubscriptionDiffPreview, String> {
    let _lock = SUBSCRIPTION_LOCK.lock().await;
    if !crate::commands::backup::is_valid_profile_id(&profile_id) {
        return Err("Invalid profile ID".to_string());
    }
    let mut profiles_path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    profiles_path.push("profiles.json");

    if !profiles_path.exists() {
        return Err("profiles.json does not exist".to_string());
    }

    let profiles_content = std::fs::read_to_string(&profiles_path)
        .map_err(|e| format!("Failed to read profiles.json: {}", e))?;
    let profiles_json: serde_json::Value = serde_json::from_str(&profiles_content)
        .map_err(|e| format!("Failed to parse profiles.json: {}", e))?;

    let profiles_arr = profiles_json
        .get("profiles")
        .and_then(|p| p.as_array())
        .ok_or_else(|| "No profiles found in profiles.json".to_string())?;

    let profile_val = profiles_arr
        .iter()
        .find(|p| p.get("id").and_then(|id| id.as_str()) == Some(&profile_id))
        .ok_or_else(|| format!("Profile not found with ID: {}", profile_id))?;

    let profile_name = profile_val
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("Subscription")
        .to_string();

    let sub_url = profile_val
        .get("subscriptionUrl")
        .and_then(|u| u.as_str())
        .ok_or_else(|| "Profile does not have a subscription URL".to_string())?;

    let current_selected_tag = profile_val.get("selectedNodeTag").and_then(|t| t.as_str());

    let current_config_path = get_profile_config_path(&app, &profile_id)?;
    let content = std::fs::read_to_string(&current_config_path)
        .map_err(|error| format!("Failed to read current profile configuration: {}", error))?;
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|error| format!("Failed to parse current profile configuration: {}", error))?;
    parsed
        .get("outbounds")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "Current profile configuration has no outbounds.".to_string())?;
    let current_tags: Vec<String> = extract_nodes_from_config(&parsed)
        .iter()
        .filter_map(|node| node.get("tag")?.as_str().map(str::to_string))
        .collect();

    let bytes = crate::config::subscription::fetch_subscription_content(sub_url).await?;
    let new_outbounds = adapt(&bytes)?;
    if new_outbounds.is_empty() {
        return Err("Subscription contains no usable proxy nodes.".to_string());
    }

    let new_tags: Vec<String> = new_outbounds.iter().map(|o| o.tag.clone()).collect();
    if std::fs::read_to_string(&profiles_path).ok().as_deref() != Some(profiles_content.as_str())
        || std::fs::read_to_string(&current_config_path)
            .ok()
            .as_deref()
            != Some(content.as_str())
    {
        return Err("Subscription changed during preview. Preview the update again.".to_string());
    }
    let mut preview = calculate_subscription_diff(
        &profile_id,
        &profile_name,
        &current_tags,
        current_selected_tag,
        &new_tags,
    );
    let token = uuid::Uuid::new_v4().to_string();
    let mut snapshots = subscription_snapshots()
        .lock()
        .map_err(|_| "Subscription preview state is unavailable".to_string())?;
    // Keep one bounded preview at a time; a later preview invalidates an earlier confirmation.
    *snapshots = Some((
        token.clone(),
        SubscriptionSnapshot {
            profile_id,
            profiles_content,
            current_config: content,
            outbounds: new_outbounds,
            created_at: Instant::now(),
        },
    ));
    preview.preview_token = Some(token);
    Ok(preview)
}

/// Previews parsed nodes and types from a raw configuration or subscription URL before import.
#[tauri::command]
pub async fn preview_import_config(content: String) -> Result<ImportPreview, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("Content is empty".to_string());
    }

    if trimmed.starts_with("http://") {
        return Err("Insecure HTTP subscription URLs are not supported. Only HTTPS is allowed for security.".to_string());
    }
    if trimmed.starts_with("ftp://") || trimmed.starts_with("file://") {
        return Err(
            "Unsupported URL scheme. Only HTTPS subscription URLs are supported.".to_string(),
        );
    }
    let bytes = if trimmed.starts_with("https://") {
        crate::config::subscription::fetch_subscription_content(trimmed).await?
    } else {
        if trimmed.len() > 10 * 1024 * 1024 {
            return Err("Preview content exceeds the 10 MB limit.".to_string());
        }
        trimmed.as_bytes().to_vec()
    };

    let outbounds = adapt(&bytes)?;
    let mut node_types: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut sample_nodes = Vec::new();

    for (idx, ob) in outbounds.iter().enumerate() {
        *node_types.entry(ob.outbound_type.clone()).or_insert(0) += 1;
        if idx < 10 {
            sample_nodes.push(ob.tag.clone());
        }
    }

    Ok(ImportPreview {
        node_count: outbounds.len(),
        node_types,
        sample_nodes,
    })
}

#[cfg(test)]
mod transaction_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn test_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!("xlink-config-txn-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn transaction_commits_all_staged_files() {
        let dir = test_dir();
        let first = dir.join("first.json");
        let second = dir.join("second.json");
        std::fs::write(&first, b"old-first").unwrap();

        commit_files_transactionally(vec![
            (first.clone(), b"new-first".to_vec()),
            (second.clone(), b"new-second".to_vec()),
        ])
        .unwrap();

        assert_eq!(std::fs::read(&first).unwrap(), b"new-first");
        assert_eq!(std::fs::read(&second).unwrap(), b"new-second");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_find_due_subscription_profiles() {
        let now_ms = 1_000_000_000_000u64;
        let one_hour_ago = now_ms - (61 * 60 * 1000);
        let half_hour_ago = now_ms - (30 * 60 * 1000);

        let data = serde_json::json!({
            "profiles": [
                {
                    "id": "p1",
                    "name": "Due Profile",
                    "type": "subscription",
                    "subscriptionUrl": "https://example.com/sub1",
                    "autoUpdateInterval": 60,
                    "lastUpdated": one_hour_ago
                },
                {
                    "id": "p2",
                    "name": "Not Due Yet",
                    "type": "subscription",
                    "subscriptionUrl": "https://example.com/sub2",
                    "autoUpdateInterval": 60,
                    "lastUpdated": half_hour_ago
                },
                {
                    "id": "p3",
                    "name": "Disabled Auto Update",
                    "type": "subscription",
                    "subscriptionUrl": "https://example.com/sub3",
                    "autoUpdateInterval": 0,
                    "lastUpdated": one_hour_ago
                },
                {
                    "id": "p4",
                    "name": "Manual Profile",
                    "type": "manual",
                    "autoUpdateInterval": 60,
                    "lastUpdated": one_hour_ago
                },
                {
                    "id": "p5",
                    "name": "Empty URL Profile",
                    "type": "subscription",
                    "subscriptionUrl": "",
                    "autoUpdateInterval": 60,
                    "lastUpdated": one_hour_ago
                }
            ]
        });

        let due = find_due_subscription_profiles(&data, now_ms);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].0, "p1");
        assert_eq!(due[0].1, "Due Profile");
    }

    #[test]
    fn test_convert_wireguard_outbound_to_endpoint_and_extract() {
        let wg_outbound = serde_json::json!({
            "type": "wireguard",
            "tag": "wg-test",
            "server": "1.2.3.4",
            "server_port": 51820,
            "private_key": "privkey123",
            "peer_public_key": "pubkey456",
            "local_address": ["10.0.0.2/32"],
            "pre_shared_key": "psk789",
            "mtu": 1420
        });

        let ep = convert_wireguard_outbound_to_endpoint(&wg_outbound);
        assert_eq!(ep["type"], "wireguard");
        assert_eq!(ep["tag"], "wg-test");
        assert_eq!(ep["private_key"], "privkey123");
        assert_eq!(ep["address"][0], "10.0.0.2/32");
        assert_eq!(ep["peers"][0]["address"], "1.2.3.4");
        assert_eq!(ep["peers"][0]["port"], 51820);
        assert_eq!(ep["peers"][0]["public_key"], "pubkey456");

        let config = serde_json::json!({
            "outbounds": [
                { "type": "selector", "tag": "proxy", "outbounds": ["vless-1", "wg-test"] },
                { "type": "vless", "tag": "vless-1", "server": "example.com", "server_port": 443 },
                { "type": "direct", "tag": "direct" }
            ],
            "endpoints": [
                ep
            ]
        });

        let extracted = extract_nodes_from_config(&config);
        assert_eq!(extracted.len(), 2);
        assert_eq!(extracted[0]["tag"], "vless-1");
        assert_eq!(extracted[1]["tag"], "wg-test");
        assert_eq!(extracted[1]["type"], "wireguard");
        assert_eq!(extracted[1]["server"], "1.2.3.4");
        assert_eq!(extracted[1]["server_port"], 51820);
    }

    #[test]
    fn test_delete_node_transforms_config_correctly() {
        let mut config = serde_json::json!({
            "outbounds": [
                { "type": "selector", "tag": "proxy", "outbounds": ["vless-1", "wg-test"], "default": "vless-1" },
                { "type": "vless", "tag": "vless-1", "server": "example.com", "server_port": 443 },
                { "type": "direct", "tag": "direct" }
            ],
            "endpoints": [
                { "type": "wireguard", "tag": "wg-test" }
            ]
        });

        // 1. Delete vless-1: selector default switches to wg-test
        let tag = "vless-1";
        if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
            outbounds.retain(|o| o.get("tag").and_then(|v| v.as_str()) != Some(tag));
            for outbound in outbounds.iter_mut() {
                if outbound.get("type").and_then(|t| t.as_str()) == Some("selector") {
                    let mut first_candidate = "direct".to_string();
                    if let Some(list) = outbound.get_mut("outbounds").and_then(|o| o.as_array_mut())
                    {
                        list.retain(|v| v.as_str() != Some(tag));
                        if let Some(first) = list.iter().find_map(|v| v.as_str()) {
                            first_candidate = first.to_string();
                        }
                    }
                    if outbound.get("default").and_then(|d| d.as_str()) == Some(tag) {
                        outbound["default"] = serde_json::Value::String(first_candidate);
                    }
                }
            }
        }

        let selector = config["outbounds"][0].clone();
        assert_eq!(selector["default"], "wg-test");
        assert_eq!(selector["outbounds"], serde_json::json!(["wg-test"]));
        assert_eq!(config["outbounds"].as_array().unwrap().len(), 2);

        // 2. Delete wg-test: endpoints removed and selector outbounds falls back to ["direct"]
        let tag2 = "wg-test";
        if let Some(endpoints) = config.get_mut("endpoints").and_then(|e| e.as_array_mut()) {
            endpoints.retain(|ep| ep.get("tag").and_then(|v| v.as_str()) != Some(tag2));
        }
        if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
            for outbound in outbounds.iter_mut() {
                if outbound.get("type").and_then(|t| t.as_str()) == Some("selector") {
                    let mut first_candidate = "direct".to_string();
                    if let Some(list) = outbound.get_mut("outbounds").and_then(|o| o.as_array_mut())
                    {
                        list.retain(|v| v.as_str() != Some(tag2));
                        if list.is_empty() {
                            list.push(serde_json::Value::String("direct".to_string()));
                        }
                        if let Some(first) = list.iter().find_map(|v| v.as_str()) {
                            first_candidate = first.to_string();
                        }
                    }
                    if outbound.get("default").and_then(|d| d.as_str()) == Some(tag2) {
                        outbound["default"] = serde_json::Value::String(first_candidate);
                    }
                }
            }
        }
        assert_eq!(config["endpoints"].as_array().unwrap().len(), 0);
        let selector2 = config["outbounds"][0].clone();
        assert_eq!(selector2["default"], "direct");
        assert_eq!(selector2["outbounds"], serde_json::json!(["direct"]));
    }

    #[test]
    fn transaction_restores_originals_when_later_commit_fails() {
        let dir = test_dir();
        let first = dir.join("first.json");
        let second = dir.join("second.json");
        std::fs::write(&first, b"old-first").unwrap();
        std::fs::write(&second, b"old-second").unwrap();
        let renames = AtomicUsize::new(0);

        let result = commit_files_transactionally_with(
            vec![
                (first.clone(), b"new-first".to_vec()),
                (second.clone(), b"new-second".to_vec()),
            ],
            |from, to| {
                if renames.fetch_add(1, Ordering::SeqCst) == 1 {
                    Err(std::io::Error::other("simulated rename failure"))
                } else {
                    std::fs::rename(from, to)
                }
            },
        );

        assert!(result.unwrap_err().contains("simulated rename failure"));
        assert_eq!(std::fs::read(&first).unwrap(), b"old-first");
        assert_eq!(std::fs::read(&second).unwrap(), b"old-second");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn transaction_reports_missing_backup_during_rollback() {
        let dir = test_dir();
        let first = dir.join("first.json");
        let second = dir.join("second.json");
        std::fs::write(&first, b"old-first").unwrap();
        std::fs::write(&second, b"old-second").unwrap();
        let mut renames = 0;

        let error = commit_files_transactionally_with(
            vec![
                (first.clone(), b"new-first".to_vec()),
                (second.clone(), b"new-second".to_vec()),
            ],
            |from, to| {
                renames += 1;
                if renames == 2 {
                    let backup = std::fs::read_dir(&dir)
                        .unwrap()
                        .filter_map(Result::ok)
                        .map(|entry| entry.path())
                        .find(|path| {
                            path.file_name()
                                .unwrap()
                                .to_string_lossy()
                                .starts_with("first.json.")
                                && path.extension().is_some_and(|ext| ext == "bak")
                        })
                        .unwrap();
                    std::fs::remove_file(backup).unwrap();
                    Err(std::io::Error::other("simulated second rename failure"))
                } else {
                    std::fs::rename(from, to)
                }
            },
        )
        .unwrap_err();

        assert!(error.contains("rollback incomplete"));
        assert!(error.contains("restore"));
        assert_eq!(std::fs::read(&second).unwrap(), b"old-second");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn transaction_rejects_duplicate_destinations_without_changing_files() {
        let dir = test_dir();
        let target = dir.join("profile.json");
        std::fs::write(&target, b"unchanged").unwrap();
        let result = commit_files_transactionally(vec![
            (target.clone(), b"one".to_vec()),
            (target.clone(), b"two".to_vec()),
        ]);
        assert!(result.unwrap_err().contains("duplicate destination"));
        assert_eq!(std::fs::read(&target).unwrap(), b"unchanged");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn profile_switch_validates_selected_node_before_committing() {
        let config = r#"{
            "outbounds": [
                {"type": "selector", "tag": "proxy", "outbounds": ["node-a", "node-b"], "default": "node-a"},
                {"type": "vless", "tag": "node-a"},
                {"type": "vless", "tag": "node-b"}
            ]
        }"#;

        let switched = prepare_profile_switch("profile-1", Some("node-b"), config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&switched).unwrap();
        assert_eq!(parsed["outbounds"][0]["default"], "node-b");
        assert!(prepare_profile_switch("profile-1", Some("missing"), config).is_err());
        assert!(prepare_profile_switch("profile-1", Some("proxy"), config).is_err());
        assert!(prepare_profile_switch("profile-1", None, "{\"outbounds\":[]}").is_ok());
        assert!(prepare_profile_switch("profile-1", None, "{}").is_err());
    }

    #[test]
    fn profile_switch_preserves_existing_selector_when_no_tag_is_provided() {
        let config = r#"{"outbounds":[{"type":"selector","tag":"proxy","outbounds":["node-a","node-b"],"default":"node-b"},{"type":"vless","tag":"node-a"},{"type":"vless","tag":"node-b"}]}"#;
        let switched = prepare_profile_switch("profile-1", None, config).unwrap();
        assert_eq!(switched, config);
    }

    #[tokio::test]
    async fn profile_switch_commits_config_and_active_id_together() {
        let dir = test_dir();
        let active = dir.join("active.json");
        let profiles = dir.join("profiles.json");
        std::fs::write(&active, b"old-active").unwrap();
        std::fs::write(&profiles, br#"{"activeProfileId":"old"}"#).unwrap();

        commit_and_apply_with_rollback(
            vec![
                (active.clone(), b"{\"outbounds\":[]}".to_vec()),
                (profiles.clone(), br#"{"activeProfileId":"next"}"#.to_vec()),
            ],
            false,
            |_| async { panic!("disconnected switch must not restart") },
            || false,
        )
        .await
        .unwrap();

        assert_eq!(std::fs::read(&active).unwrap(), b"{\"outbounds\":[]}");
        let metadata: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&profiles).unwrap()).unwrap();
        assert_eq!(metadata["activeProfileId"], "next");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn connected_apply_keeps_candidate_after_success() {
        let dir = test_dir();
        let active = dir.join("active.json");
        let profiles = dir.join("profiles.json");
        std::fs::write(&active, b"old-active").unwrap();
        std::fs::write(&profiles, b"old-metadata").unwrap();
        let calls = AtomicUsize::new(0);

        commit_and_apply_with_rollback(
            vec![
                (active.clone(), b"new-active".to_vec()),
                (profiles.clone(), b"new-metadata".to_vec()),
            ],
            true,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                assert_eq!(std::fs::read(&active).unwrap(), b"new-active");
                assert_eq!(std::fs::read(&profiles).unwrap(), b"new-metadata");
                async { Ok(()) }
            },
            || true,
        )
        .await
        .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(std::fs::read(&active).unwrap(), b"new-active");
        assert_eq!(std::fs::read(&profiles).unwrap(), b"new-metadata");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn connected_apply_failure_before_stop_restores_files_without_restarting() {
        let dir = test_dir();
        let active = dir.join("active.json");
        let profiles = dir.join("profiles.json");
        std::fs::write(&active, b"old-active").unwrap();
        std::fs::write(&profiles, b"old-metadata").unwrap();
        let calls = AtomicUsize::new(0);

        let error = commit_and_apply_with_rollback(
            vec![
                (active.clone(), b"new-active".to_vec()),
                (profiles.clone(), b"new-metadata".to_vec()),
            ],
            true,
            |_| {
                assert_eq!(calls.fetch_add(1, Ordering::SeqCst), 0);
                async { Err("validation failed".to_string()) }
            },
            || true,
        )
        .await
        .unwrap_err();

        assert!(error.contains("validation failed"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(std::fs::read(&active).unwrap(), b"old-active");
        assert_eq!(std::fs::read(&profiles).unwrap(), b"old-metadata");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn connected_apply_failure_after_stop_recovers_previous_connection() {
        use std::sync::atomic::AtomicBool;
        let dir = test_dir();
        let active = dir.join("active.json");
        let profiles = dir.join("profiles.json");
        std::fs::write(&active, b"old-active").unwrap();
        std::fs::write(&profiles, b"old-metadata").unwrap();
        let running = AtomicBool::new(true);
        let calls = AtomicUsize::new(0);

        let error = commit_and_apply_with_rollback(
            vec![
                (active.clone(), b"new-active".to_vec()),
                (profiles.clone(), b"new-metadata".to_vec()),
            ],
            true,
            |restoring| {
                if restoring {
                    assert_eq!(calls.fetch_add(1, Ordering::SeqCst), 1);
                    assert_eq!(std::fs::read(&active).unwrap(), b"old-active");
                    assert_eq!(std::fs::read(&profiles).unwrap(), b"old-metadata");
                    running.store(true, Ordering::SeqCst);
                } else {
                    assert_eq!(calls.fetch_add(1, Ordering::SeqCst), 0);
                    running.store(false, Ordering::SeqCst);
                }
                std::future::ready(if restoring {
                    Ok(())
                } else {
                    Err("restart failed".to_string())
                })
            },
            || running.load(Ordering::SeqCst),
        )
        .await
        .unwrap_err();

        assert!(error.contains("restart failed"));
        assert!(running.load(Ordering::SeqCst));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(std::fs::read(&active).unwrap(), b"old-active");
        assert_eq!(std::fs::read(&profiles).unwrap(), b"old-metadata");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn connected_apply_reports_recovery_failure_and_keeps_previous_files() {
        use std::sync::atomic::AtomicBool;
        let dir = test_dir();
        let active = dir.join("active.json");
        std::fs::write(&active, b"old-active").unwrap();
        let running = AtomicBool::new(true);

        let error = commit_and_apply_with_rollback(
            vec![(active.clone(), b"new-active".to_vec())],
            true,
            |restoring| {
                running.store(false, Ordering::SeqCst);
                async move {
                    if restoring {
                        Err("recovery failed".to_string())
                    } else {
                        Err("restart failed".to_string())
                    }
                }
            },
            || running.load(Ordering::SeqCst),
        )
        .await
        .unwrap_err();

        assert!(error.contains("recovery failed"));
        assert!(!running.load(Ordering::SeqCst));
        assert_eq!(std::fs::read(&active).unwrap(), b"old-active");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn reported_apply_success_without_connection_is_rolled_back() {
        use std::sync::atomic::AtomicBool;
        let dir = test_dir();
        let active = dir.join("active.json");
        std::fs::write(&active, b"old-active").unwrap();
        let running = AtomicBool::new(false);
        let calls = AtomicUsize::new(0);

        let error = commit_and_apply_with_rollback(
            vec![(active.clone(), b"new-active".to_vec())],
            true,
            |restoring| {
                calls.fetch_add(1, Ordering::SeqCst);
                if restoring {
                    assert_eq!(std::fs::read(&active).unwrap(), b"old-active");
                    running.store(true, Ordering::SeqCst);
                }
                async { Ok(()) }
            },
            || running.load(Ordering::SeqCst),
        )
        .await
        .unwrap_err();

        assert!(error.contains("not running"));
        assert!(running.load(Ordering::SeqCst));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(std::fs::read(&active).unwrap(), b"old-active");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn transactional_removal_keeps_files_when_later_stage_fails() {
        let dir = test_dir();
        let first = dir.join("first.json");
        let second = dir.join("second.json");
        std::fs::write(&first, b"keep-me").unwrap();
        std::fs::write(&second, b"also-keep-me").unwrap();
        let renames = AtomicUsize::new(0);

        let result =
            remove_files_transactionally_with(vec![first.clone(), second.clone()], |from, to| {
                if renames.fetch_add(1, Ordering::SeqCst) == 1 {
                    Err(std::io::Error::other("simulated delete failure"))
                } else {
                    std::fs::rename(from, to)
                }
            });

        assert!(result.unwrap_err().contains("simulated delete failure"));
        assert_eq!(std::fs::read(&first).unwrap(), b"keep-me");
        assert_eq!(std::fs::read(&second).unwrap(), b"also-keep-me");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn transactional_removal_commits_profile_and_final_active_files() {
        let dir = test_dir();
        let profile = dir.join("profile.json");
        let active = dir.join("active.json");
        std::fs::write(&profile, b"profile").unwrap();
        std::fs::write(&active, b"active").unwrap();

        remove_files_transactionally(vec![profile.clone(), active.clone()]).unwrap();

        assert!(!profile.exists());
        assert!(!active.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_safe_rule_set_paths_destination_safety() {
        let dir = test_dir();
        let (dest_path, temp_path) = safe_rule_set_paths(&dir, "geoip-cn", "binary").unwrap();
        assert_eq!(dest_path, dir.join("geoip-cn.srs"));
        assert_eq!(dest_path.parent().unwrap(), dir);
        assert!(dest_path.starts_with(&dir));
        assert_eq!(temp_path.parent().unwrap(), dir);
        assert!(temp_path.starts_with(&dir));
        assert!(temp_path.to_string_lossy().ends_with(".tmp"));

        let (dest_json, _) = safe_rule_set_paths(&dir, "custom-rules", "source").unwrap();
        assert_eq!(dest_json, dir.join("custom-rules.json"));

        assert!(safe_rule_set_paths(&dir, "../evil", "binary").is_err());
        assert!(safe_rule_set_paths(&dir, "CON", "binary").is_err());
        assert!(safe_rule_set_paths(&dir, "", "binary").is_err());
        assert!(safe_rule_set_paths(&dir, "C:\\evil", "binary").is_err());
        assert!(safe_rule_set_paths(&dir, "valid", "unsupported").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_stage_and_commit_rule_set_no_write_for_invalid() {
        let dir = test_dir();

        // Invalid tags must fail and produce no writes/files
        let bad_tags = [
            "../evil",
            "..\\evil",
            "..",
            "",
            "   ",
            "CON",
            "prn",
            "aux.json",
            "NUL",
            "COM1",
            "C:\\evil",
            "/etc/passwd",
            "foo/bar",
            "foo\\bar",
            "foo:bar",
            "tag|pipe",
            ".hidden",
            "trailing.",
        ];

        for &bad_tag in &bad_tags {
            let res =
                stage_and_commit_rule_set_content(&dir, bad_tag, "binary", b"malicious content");
            assert!(res.is_err(), "Expected error for bad tag: {}", bad_tag);
        }

        // Verify no files were created in dir
        let entries = std::fs::read_dir(&dir).unwrap().collect::<Vec<_>>();
        assert!(
            entries.is_empty(),
            "Directory must be empty after invalid attempts"
        );

        // Valid tag must write successfully and leave no temp files
        let dest =
            stage_and_commit_rule_set_content(&dir, "geoip-cn", "binary", b"valid binary content")
                .unwrap();
        assert!(dest.exists());
        assert_eq!(std::fs::read(&dest).unwrap(), b"valid binary content");

        // Verify only the destination file exists (no leftover .tmp files)
        let entries = std::fs::read_dir(&dir).unwrap().collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].as_ref().unwrap().path(), dest);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_subscription_preview_token_is_one_time_and_detects_changes() {
        let dir = std::env::temp_dir().join(format!("xlink_preview_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let config = dir.join("profile.json");
        std::fs::write(&config, "previous").unwrap();
        let outbound = crate::config::adapters::SingBoxOutbound {
            outbound_type: "vmess".into(),
            tag: "new".into(),
            fields: Default::default(),
        };
        let put_snapshot = |token: &str| {
            *subscription_snapshots().lock().unwrap() = Some((
                token.into(),
                SubscriptionSnapshot {
                    profile_id: "profile".into(),
                    profiles_content: "profiles".into(),
                    current_config: "previous".into(),
                    outbounds: vec![outbound.clone()],
                    created_at: Instant::now(),
                },
            ));
        };
        put_snapshot("one");
        assert!(take_subscription_snapshot("one", "profile", "profiles", &config).is_ok());
        assert!(take_subscription_snapshot("one", "profile", "profiles", &config).is_err());
        put_snapshot("two");
        std::fs::write(&config, "changed").unwrap();
        assert!(take_subscription_snapshot("two", "profile", "profiles", &config).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_calculate_subscription_diff_preserves_selected_node() {
        let current = vec![
            "node-1".to_string(),
            "node-2".to_string(),
            "node-3".to_string(),
        ];
        let incoming = vec![
            "node-2".to_string(),
            "node-3".to_string(),
            "node-4".to_string(),
        ];

        let diff =
            calculate_subscription_diff("sub-1", "My Sub", &current, Some("node-2"), &incoming);
        assert_eq!(diff.added_nodes, vec!["node-4"]);
        assert_eq!(diff.removed_nodes, vec!["node-1"]);
        assert_eq!(diff.retained_node_count, 2);
        assert!(diff.is_selected_node_preserved);
        assert_eq!(diff.fallback_selected_tag, Some("node-2".to_string()));
    }

    #[test]
    fn test_calculate_subscription_diff_handles_removed_selected_node() {
        let current = vec!["node-1".to_string(), "node-2".to_string()];
        let incoming = vec!["node-3".to_string(), "node-4".to_string()];

        let diff =
            calculate_subscription_diff("sub-1", "My Sub", &current, Some("node-1"), &incoming);
        assert_eq!(diff.added_nodes, vec!["node-3", "node-4"]);
        assert_eq!(diff.removed_nodes, vec!["node-1", "node-2"]);
        assert_eq!(diff.retained_node_count, 0);
        assert!(!diff.is_selected_node_preserved);
        assert_eq!(diff.fallback_selected_tag, Some("node-3".to_string()));
    }
}

/// Returns the customizable proxy outbounds and endpoints (i.e. not selector/direct/block/dns)
/// from the active config, for display as selectable server "nodes" in the UI.
#[tauri::command]
pub fn get_config_outbounds(app: tauri::AppHandle) -> Result<Vec<serde_json::Value>, String> {
    let config_path = get_active_config_path(&app)?;
    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    Ok(extract_nodes_from_config(&config))
}

/// Converts a WireGuard outbound node into a sing-box 1.13.14 endpoint object.
pub fn convert_wireguard_outbound_to_endpoint(node: &serde_json::Value) -> serde_json::Value {
    let tag = node
        .get("tag")
        .and_then(|t| t.as_str())
        .unwrap_or("wireguard")
        .to_string();
    let address = if let Some(addr) = node.get("address").or_else(|| node.get("local_address")) {
        if addr.is_array() {
            addr.clone()
        } else if let Some(s) = addr.as_str() {
            serde_json::json!([s])
        } else {
            serde_json::json!(["10.0.0.2/32"])
        }
    } else {
        serde_json::json!(["10.0.0.2/32"])
    };

    let private_key = node
        .get("private_key")
        .or_else(|| node.get("secret_key"))
        .and_then(|k| k.as_str())
        .unwrap_or("")
        .to_string();

    let server_address = node
        .get("server")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    let server_port = node
        .get("server_port")
        .and_then(|p| p.as_u64())
        .unwrap_or(51820) as u16;

    let peer_public_key = node
        .get("peer_public_key")
        .or_else(|| node.get("public_key"))
        .and_then(|k| k.as_str())
        .unwrap_or("")
        .to_string();

    let allowed_ips = if let Some(ips) = node.get("allowed_ips") {
        if ips.is_array() {
            ips.clone()
        } else if let Some(s) = ips.as_str() {
            serde_json::json!([s])
        } else {
            serde_json::json!(["0.0.0.0/0", "::/0"])
        }
    } else {
        serde_json::json!(["0.0.0.0/0", "::/0"])
    };

    let mut peer_obj = serde_json::json!({
        "address": server_address,
        "port": server_port,
        "public_key": peer_public_key,
        "allowed_ips": allowed_ips
    });

    if let Some(reserved) = node.get("reserved") {
        peer_obj["reserved"] = reserved.clone();
    }

    let mut endpoint_obj = serde_json::json!({
        "type": "wireguard",
        "tag": tag,
        "address": address,
        "private_key": private_key,
        "peers": [peer_obj]
    });

    if let Some(mtu) = node.get("mtu").and_then(|m| m.as_u64()) {
        endpoint_obj["mtu"] = serde_json::json!(mtu);
    }

    endpoint_obj
}

/// Extracts all selectable proxy nodes from config (both outbounds and endpoints).
pub fn extract_nodes_from_config(config: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut nodes = Vec::new();
    if let Some(outbounds) = config.get("outbounds").and_then(|o| o.as_array()) {
        for outbound in outbounds {
            let outbound_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if outbound_type != "selector"
                && outbound_type != "urltest"
                && outbound_type != "fallback"
                && outbound_type != "direct"
                && outbound_type != "block"
                && outbound_type != "dns"
            {
                nodes.push(outbound.clone());
            }
        }
    }
    if let Some(endpoints) = config.get("endpoints").and_then(|e| e.as_array()) {
        for ep in endpoints {
            let ep_type = ep.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if ep_type == "wireguard" {
                let mut node = ep.clone();
                if let Some(peer) = ep
                    .get("peers")
                    .and_then(|p| p.as_array())
                    .and_then(|p| p.first())
                {
                    if let Some(addr) = peer.get("address") {
                        node["server"] = addr.clone();
                    }
                    if let Some(port) = peer.get("port") {
                        node["server_port"] = port.clone();
                    }
                    if let Some(pk) = peer.get("public_key") {
                        node["peer_public_key"] = pk.clone();
                    }
                    if let Some(ips) = peer.get("allowed_ips") {
                        node["allowed_ips"] = ips.clone();
                    }
                }
                nodes.push(node);
            }
        }
    }
    nodes
}

/// Returns the outbound the "proxy" selector currently defaults to (i.e. the
/// node or outbound group that will actually be used to connect), falling back
/// to the first customizable outbound if no default has been set yet.
#[tauri::command]
pub fn get_active_outbound(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let config_path = get_active_config_path(&app)?;
    if !config_path.exists() {
        return Err("No active configuration found".to_string());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    let all_nodes = extract_nodes_from_config(&config);
    let outbound_groups = crate::config::rules::load_outbound_groups_from_file(&app);

    if let Some(outbounds) = config.get("outbounds").and_then(|o| o.as_array()) {
        if let Some(selector) = outbounds.iter().find(|o| {
            o.get("type").and_then(|t| t.as_str()) == Some("selector")
                && o.get("tag").and_then(|t| t.as_str()) == Some("proxy")
        }) {
            if let Some(default_tag) = selector.get("default").and_then(|d| d.as_str()) {
                if let Some(found) = all_nodes
                    .iter()
                    .find(|o| o.get("tag").and_then(|t| t.as_str()) == Some(default_tag))
                {
                    return Ok(found.clone());
                }
                if outbound_groups.iter().any(|g| g.tag == default_tag) {
                    if let Some(group_ob) = outbounds
                        .iter()
                        .find(|o| o.get("tag").and_then(|t| t.as_str()) == Some(default_tag))
                    {
                        return Ok(group_ob.clone());
                    }
                }
            }
        }
    }

    if let Some(first) = all_nodes.first() {
        return Ok(first.clone());
    }

    Err("No primary proxy outbound found in configuration".to_string())
}

/// Reads the tag the "proxy" selector currently defaults to, if any.
pub fn get_selected_outbound_tag(app: &tauri::AppHandle) -> Option<String> {
    let config_path = get_active_config_path(app).ok()?;
    if !config_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&config_path).ok()?;
    let config: serde_json::Value = serde_json::from_str(&content).ok()?;
    let outbounds = config.get("outbounds")?.as_array()?;
    let selector = outbounds.iter().find(|o| {
        o.get("type").and_then(|t| t.as_str()) == Some("selector")
            && o.get("tag").and_then(|t| t.as_str()) == Some("proxy")
    })?;
    selector
        .get("default")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string())
}

/// Edits an existing node's connection parameters in-place or adds a new node.
/// If connected, validates and cold-restarts sing-box with the updated config.
#[tauri::command]
pub async fn update_node(
    app: tauri::AppHandle,
    new_outbound: serde_json::Value,
) -> Result<(), String> {
    let config_path = get_active_config_path(&app)?;
    if !config_path.exists() {
        return Err("No active configuration found".to_string());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let mut config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    let is_wireguard = new_outbound.get("type").and_then(|t| t.as_str()) == Some("wireguard");
    let target_tag = new_outbound
        .get("tag")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Tag name is required for node".to_string())?;

    let mut old_tag: Option<String> = None;
    let mut target_outbound_idx = None;
    let mut target_endpoint_idx = None;

    // Search outbounds for exact tag match
    if let Some(outbounds) = config.get("outbounds").and_then(|o| o.as_array()) {
        for (idx, outbound) in outbounds.iter().enumerate() {
            let t = outbound.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if t == "selector"
                || t == "urltest"
                || t == "fallback"
                || t == "direct"
                || t == "block"
                || t == "dns"
            {
                continue;
            }
            if let Some(tag) = outbound.get("tag").and_then(|v| v.as_str()) {
                if tag == target_tag {
                    target_outbound_idx = Some(idx);
                    old_tag = Some(tag.to_string());
                    break;
                }
            }
        }
    }

    // Search endpoints for exact tag match
    if target_outbound_idx.is_none() {
        if let Some(endpoints) = config.get("endpoints").and_then(|e| e.as_array()) {
            for (idx, ep) in endpoints.iter().enumerate() {
                if let Some(tag) = ep.get("tag").and_then(|v| v.as_str()) {
                    if tag == target_tag {
                        target_endpoint_idx = Some(idx);
                        old_tag = Some(tag.to_string());
                        break;
                    }
                }
            }
        }
    }

    if is_wireguard {
        let ep_val = convert_wireguard_outbound_to_endpoint(&new_outbound);
        // If it previously lived in outbounds, remove from outbounds
        if let Some(out_idx) = target_outbound_idx {
            if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
                outbounds.remove(out_idx);
            }
        }
        // Insert or update in endpoints
        if let Some(ep_idx) = target_endpoint_idx {
            let endpoints = config
                .get_mut("endpoints")
                .and_then(|e| e.as_array_mut())
                .unwrap();
            endpoints[ep_idx] = ep_val;
        } else {
            if config.get("endpoints").is_none() {
                config["endpoints"] = serde_json::Value::Array(Vec::new());
            }
            if let Some(arr) = config.get_mut("endpoints").and_then(|e| e.as_array_mut()) {
                arr.push(ep_val);
            }
        }
    } else {
        // If it previously lived in endpoints, remove from endpoints
        if let Some(ep_idx) = target_endpoint_idx {
            if let Some(endpoints) = config.get_mut("endpoints").and_then(|e| e.as_array_mut()) {
                endpoints.remove(ep_idx);
            }
        }
        // Insert or update in outbounds
        if let Some(out_idx) = target_outbound_idx {
            let outbounds = config
                .get_mut("outbounds")
                .and_then(|o| o.as_array_mut())
                .unwrap();
            outbounds[out_idx] = new_outbound.clone();
        } else {
            // New node addition to outbounds
            let outbounds = config
                .get_mut("outbounds")
                .and_then(|o| o.as_array_mut())
                .ok_or_else(|| "No outbounds array found in configuration".to_string())?;
            outbounds.push(new_outbound.clone());
        }
    }

    // Ensure target_tag is in root 'proxy' selector.outbounds
    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        for outbound in outbounds.iter_mut() {
            if outbound.get("type").and_then(|t| t.as_str()) == Some("selector")
                && outbound.get("tag").and_then(|t| t.as_str()) == Some("proxy")
            {
                if let Some(list) = outbound.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
                    if let Some(ref old) = old_tag {
                        if old != &target_tag {
                            for item in list.iter_mut() {
                                if item.as_str() == Some(old.as_str()) {
                                    *item = serde_json::Value::String(target_tag.clone());
                                }
                            }
                        }
                    }
                    if !list.iter().any(|v| v.as_str() == Some(&target_tag)) {
                        let list_len = list.len();
                        list.retain(|v| v.as_str() != Some("direct") || list_len <= 1);
                        list.push(serde_json::Value::String(target_tag.clone()));
                    }
                }
            }
        }
    }

    let pretty_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config to JSON: {}", e))?;

    // Validate before committing
    let temp_path = get_temp_config_path(&app)?;
    std::fs::write(&temp_path, &pretty_config)
        .map_err(|e| format!("Failed to write temp config: {}", e))?;
    let validation_res = validate_singbox_config(&app, &temp_path).await;
    let _ = std::fs::remove_file(&temp_path);
    validation_res?;

    let mut files = vec![(config_path.clone(), pretty_config.as_bytes().to_vec())];
    let active_profile_id = get_active_profile_id(&app)?;
    if let Some(profile_id) = active_profile_id.as_deref() {
        let profile_path = get_profile_config_path(&app, profile_id)?;
        if profile_path.exists() {
            files.push((profile_path, pretty_config.as_bytes().to_vec()));
        }

        // Update profiles.json nodeCount
        let mut profiles_path = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        profiles_path.push("profiles.json");
        if profiles_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&profiles_path) {
                if let Ok(mut p_json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let total_nodes = extract_nodes_from_config(&config).len() as u32;
                    if let Some(arr) = p_json.get_mut("profiles").and_then(|p| p.as_array_mut()) {
                        if let Some(target) = arr
                            .iter_mut()
                            .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(profile_id))
                        {
                            target["nodeCount"] = serde_json::Value::Number(total_nodes.into());
                            let now_ms = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            target["lastUpdated"] = serde_json::Value::Number(now_ms.into());
                            if let Ok(serialized) = serde_json::to_vec_pretty(&p_json) {
                                files.push((profiles_path, serialized));
                            }
                        }
                    }
                }
            }
        }
    }

    // Preserve the current runtime mode and previous durable configs until a replacement
    // process has passed the normal validated startup checks.
    let state = app.state::<crate::state::ProxyState>();
    let connected =
        state.get_status() == crate::state::ConnectionStatus::Connected && state.is_running();
    let mode_override = state
        .effective_proxy_mode
        .lock()
        .map_err(|error| format!("state_lock_poisoned: {}", error))?
        .clone();
    let previous_selected_tag = get_selected_outbound_tag(&app);
    let updated_selected_tag = config
        .get("outbounds")
        .and_then(|outbounds| outbounds.as_array())
        .and_then(|outbounds| {
            outbounds.iter().find(|outbound| {
                outbound.get("type").and_then(|value| value.as_str()) == Some("selector")
                    && outbound.get("tag").and_then(|value| value.as_str()) == Some("proxy")
            })
        })
        .and_then(|selector| selector.get("default"))
        .and_then(|default| default.as_str())
        .map(str::to_string);
    commit_and_apply_with_rollback(
        files,
        connected,
        |restoring| {
            crate::commands::proxy::try_reload_proxy_config_with_mode(
                &app,
                if restoring {
                    previous_selected_tag.clone()
                } else {
                    updated_selected_tag.clone()
                },
                mode_override.clone(),
            )
        },
        || state.get_status() == crate::state::ConnectionStatus::Connected && state.is_running(),
    )
    .await
}

/// Deletes an existing node from the active/profile configuration by its tag.
/// If connected, validates and restarts/reloads sing-box with the updated config.
#[tauri::command]
pub async fn delete_node(app: tauri::AppHandle, tag: String) -> Result<(), String> {
    let config_path = get_active_config_path(&app)?;
    if !config_path.exists() {
        return Err("No active configuration found".to_string());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let mut config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    let mut removed = false;

    // 1. Remove from outbounds
    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        let initial_len = outbounds.len();
        outbounds.retain(|o| {
            let t = o.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if t == "selector"
                || t == "urltest"
                || t == "fallback"
                || t == "direct"
                || t == "block"
                || t == "dns"
            {
                return true;
            }
            o.get("tag").and_then(|v| v.as_str()) != Some(&tag)
        });
        if outbounds.len() < initial_len {
            removed = true;
        }
    }

    // 2. Remove from endpoints (e.g. WireGuard)
    if let Some(endpoints) = config.get_mut("endpoints").and_then(|e| e.as_array_mut()) {
        let initial_len = endpoints.len();
        endpoints.retain(|ep| ep.get("tag").and_then(|v| v.as_str()) != Some(&tag));
        if endpoints.len() < initial_len {
            removed = true;
        }
    }

    if !removed {
        return Err(format!(
            "Node with tag '{}' not found in configuration",
            tag
        ));
    }

    // 3. Remove from selector outbounds list
    let mut next_selected_tag: Option<String> = None;
    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        for outbound in outbounds.iter_mut() {
            if outbound.get("type").and_then(|t| t.as_str()) == Some("selector")
                && outbound.get("tag").and_then(|t| t.as_str()) == Some("proxy")
            {
                let mut first_candidate = "direct".to_string();
                if let Some(list) = outbound.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
                    list.retain(|v| v.as_str() != Some(&tag));
                    if list.is_empty() {
                        list.push(serde_json::Value::String("direct".to_string()));
                    }
                    if let Some(first) = list.iter().find_map(|v| v.as_str()) {
                        first_candidate = first.to_string();
                    }
                }
                let is_current_default =
                    outbound.get("default").and_then(|d| d.as_str()) == Some(&tag);
                if is_current_default {
                    outbound["default"] = serde_json::Value::String(first_candidate.clone());
                    next_selected_tag = if first_candidate != "direct" {
                        Some(first_candidate)
                    } else {
                        None
                    };
                } else if let Some(def) = outbound.get("default").and_then(|d| d.as_str()) {
                    next_selected_tag = Some(def.to_string());
                }
            }
        }
    }

    let pretty_config = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config to JSON: {}", e))?;

    // 4. Validate before committing
    let temp_path = get_temp_config_path(&app)?;
    std::fs::write(&temp_path, &pretty_config)
        .map_err(|e| format!("Failed to write temp config: {}", e))?;
    let validation_res = validate_singbox_config(&app, &temp_path).await;
    let _ = std::fs::remove_file(&temp_path);
    validation_res?;

    // 5. Stage files transactionally
    let mut files = vec![(config_path.clone(), pretty_config.as_bytes().to_vec())];
    let active_profile_id = get_active_profile_id(&app)?;
    if let Some(profile_id) = active_profile_id.as_deref() {
        let profile_path = get_profile_config_path(&app, profile_id)?;
        if profile_path.exists() {
            files.push((profile_path, pretty_config.as_bytes().to_vec()));
        }

        let mut profiles_path = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        profiles_path.push("profiles.json");
        if profiles_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&profiles_path) {
                if let Ok(mut p_json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let total_nodes = extract_nodes_from_config(&config).len() as u32;
                    if let Some(arr) = p_json.get_mut("profiles").and_then(|p| p.as_array_mut()) {
                        if let Some(target) = arr
                            .iter_mut()
                            .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(profile_id))
                        {
                            target["nodeCount"] = serde_json::Value::Number(total_nodes.into());
                            if target.get("selectedNodeTag").and_then(|t| t.as_str()) == Some(&tag)
                            {
                                target["selectedNodeTag"] = match &next_selected_tag {
                                    Some(t) => serde_json::Value::String(t.clone()),
                                    None => serde_json::Value::Null,
                                };
                            }
                            let now_ms = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            target["lastUpdated"] = serde_json::Value::Number(now_ms.into());
                            if let Ok(serialized) = serde_json::to_vec_pretty(&p_json) {
                                files.push((profiles_path, serialized));
                            }
                        }
                    }
                }
            }
        }
    }

    let state = app.state::<crate::state::ProxyState>();
    let connected =
        state.get_status() == crate::state::ConnectionStatus::Connected && state.is_running();
    let mode_override = state
        .effective_proxy_mode
        .lock()
        .map_err(|error| format!("state_lock_poisoned: {}", error))?
        .clone();
    let previous_selected_tag = get_selected_outbound_tag(&app);

    commit_and_apply_with_rollback(
        files,
        connected,
        |restoring| {
            crate::commands::proxy::try_reload_proxy_config_with_mode(
                &app,
                if restoring {
                    previous_selected_tag.clone()
                } else {
                    next_selected_tag.clone()
                },
                mode_override.clone(),
            )
        },
        || state.get_status() == crate::state::ConnectionStatus::Connected && state.is_running(),
    )
    .await
}

/// Creates an empty manual profile with no initial proxy nodes.
/// Produces a valid sing-box fallback configuration (with direct fallback),
/// registers the profile in profiles.json as type "manual", and sets it active if needed.
#[tauri::command]
pub async fn create_empty_profile(
    app: tauri::AppHandle,
    state: State<'_, crate::state::ProxyState>,
    name: String,
) -> Result<ProfileMeta, String> {
    let profile_name = if name.trim().is_empty() {
        "Manual Profile".to_string()
    } else {
        name.trim().to_string()
    };

    let profile_id = uuid::Uuid::new_v4().to_string();

    // 1. Generate empty sing-box config
    let (proxy_mode, dns, wifi) = read_basic_settings(&app);
    let listen_address = if wifi { "0.0.0.0" } else { "127.0.0.1" };
    let dns_address = crate::config::resolve_dns_address(dns.as_deref());
    let tun_settings = load_tun_settings(&app);
    let mixed_port = state.get_settings().mixed_port;
    let (user_rules, rule_sets) = crate::config::rules::load_routing_rules_from_file(&app);
    let outbound_groups = crate::config::rules::load_outbound_groups_from_file(&app);
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let host_dns = crate::os::get_network_manager()
        .get_system_dns_servers()
        .await
        .unwrap_or_default();

    let generated_config = generate_singbox_config(
        mixed_port,
        Vec::new(),
        &proxy_mode,
        &dns_address,
        listen_address,
        &tun_settings,
        None,
        &user_rules,
        &rule_sets,
        &app_data_dir,
        &outbound_groups,
        &host_dns,
    )?;

    // 2. Validate empty config
    let temp_path = get_temp_config_path(&app)?;
    std::fs::write(&temp_path, &generated_config)
        .map_err(|e| format!("Failed to write temp config: {}", e))?;
    let validation = validate_singbox_config(&app, &temp_path).await;
    let _ = std::fs::remove_file(&temp_path);
    validation.map_err(|e| format!("Validation of empty config failed: {}", e))?;

    // 3. Write profile config
    let profile_path = get_profile_config_path(&app, &profile_id)?;
    std::fs::write(&profile_path, generated_config.as_bytes())
        .map_err(|e| format!("Failed to write profile config: {}", e))?;

    // 4. Update profiles.json
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let mut profiles_path = app_data_dir;
    profiles_path.push("profiles.json");

    let mut profiles_json = if profiles_path.exists() {
        let content = std::fs::read_to_string(&profiles_path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|_| serde_json::json!({ "profiles": [] }))
    } else {
        serde_json::json!({ "profiles": [] })
    };

    let new_profile_entry = serde_json::json!({
        "id": profile_id,
        "name": profile_name,
        "type": "manual",
        "importedAt": now_ms,
        "lastUpdated": now_ms,
        "nodeCount": 0,
        "selectedNodeTag": null
    });

    let is_first_profile = profiles_json
        .get("profiles")
        .and_then(|p| p.as_array())
        .is_none_or(|arr| arr.is_empty());

    if let Some(arr) = profiles_json
        .get_mut("profiles")
        .and_then(|p| p.as_array_mut())
    {
        arr.push(new_profile_entry);
    } else {
        profiles_json["profiles"] = serde_json::json!([new_profile_entry]);
    }

    if is_first_profile {
        profiles_json["activeProfileId"] = serde_json::Value::String(profile_id.clone());
        let active_path = get_active_config_path(&app)?;
        let _ = std::fs::write(&active_path, generated_config.as_bytes());
    }

    let updated_bytes = serde_json::to_vec_pretty(&profiles_json)
        .map_err(|e| format!("Failed to serialize profiles.json: {}", e))?;
    std::fs::write(&profiles_path, updated_bytes)
        .map_err(|e| format!("Failed to write profiles.json: {}", e))?;

    Ok(ProfileMeta {
        id: profile_id,
        name: profile_name,
        imported_at: now_ms,
        node_count: 0,
    })
}

/// Deletes the per-profile config file for a given profile ID.
#[tauri::command]
pub fn delete_profile_config(app: tauri::AppHandle, profile_id: String) -> Result<(), String> {
    let profile_path = get_profile_config_path(&app, &profile_id)?;
    let active_path = get_active_config_path(&app)?;
    let profiles_path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("profiles.json");
    let is_active = get_active_profile_id(&app)?.as_deref() == Some(profile_id.as_str());
    if is_active {
        return Err(
            "Cannot delete the active profile config before selecting a replacement".to_string(),
        );
    }

    let profiles_json = if profiles_path.exists() {
        let content = std::fs::read_to_string(&profiles_path)
            .map_err(|e| format!("Failed to read profiles.json: {}", e))?;
        serde_json::from_str::<serde_json::Value>(&content)
            .map_err(|e| format!("Failed to parse profiles.json: {}", e))?
    } else {
        serde_json::Value::Null
    };
    let profiles_empty = profiles_json
        .get("profiles")
        .and_then(|profiles| profiles.as_array())
        .is_none_or(|profiles| profiles.is_empty());
    let no_active_profile = profiles_json
        .get("activeProfileId")
        .and_then(|value| value.as_str())
        .is_none();

    let mut paths = vec![profile_path];
    if profiles_empty && no_active_profile {
        paths.push(active_path);
    }
    remove_files_transactionally(paths)
}

fn prepare_profile_switch(
    profile_id: &str,
    selected_node_tag: Option<&str>,
    profile_config: &str,
) -> Result<String, String> {
    let final_config = if let Some(tag) = selected_node_tag {
        let mut config: serde_json::Value = serde_json::from_str(profile_config)
            .map_err(|e| format!("Failed to parse profile config: {}", e))?;
        let outbounds = config
            .get_mut("outbounds")
            .and_then(|outbounds| outbounds.as_array_mut())
            .ok_or_else(|| "No outbounds array found in profile config".to_string())?;
        if !outbounds.iter().any(|outbound| {
            outbound.get("tag").and_then(|value| value.as_str()) == Some(tag)
                && outbound.get("tag").and_then(|value| value.as_str()) != Some("proxy")
                && !matches!(
                    outbound.get("type").and_then(|value| value.as_str()),
                    Some("direct" | "block" | "dns")
                )
        }) {
            return Err(format!("Selected node not found in profile {}", profile_id));
        }
        for outbound in outbounds {
            if outbound.get("type").and_then(|value| value.as_str()) == Some("selector")
                && outbound.get("tag").and_then(|value| value.as_str()) == Some("proxy")
            {
                outbound["default"] = serde_json::Value::String(tag.to_string());
            }
        }
        serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?
    } else {
        profile_config.to_string()
    };

    let parsed: serde_json::Value = serde_json::from_str(&final_config)
        .map_err(|e| format!("Failed to parse profile config: {}", e))?;
    let outbounds = parsed
        .get("outbounds")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "No outbounds array found in profile config".to_string())?;
    if let Some(tag) = selected_node_tag {
        if !outbounds
            .iter()
            .any(|outbound| outbound.get("tag").and_then(|value| value.as_str()) == Some(tag))
        {
            return Err(format!("Selected node not found in profile {}", profile_id));
        }
    }
    Ok(final_config)
}

/// Switches the active config to a different profile by copying its config to the active slot.
/// If connected, validates and cold-restarts the proxy, restoring the prior profile on failure.
#[tauri::command]
pub async fn switch_profile(
    app: tauri::AppHandle,
    profile_id: String,
    selected_node_tag: Option<String>,
) -> Result<(), String> {
    let profile_path = get_profile_config_path(&app, &profile_id)?;
    if !profile_path.exists() {
        return Err(format!(
            "Profile config file not found for ID: {}",
            profile_id
        ));
    }

    let config_content = std::fs::read_to_string(&profile_path)
        .map_err(|e| format!("Failed to read profile config: {}", e))?;

    let final_config =
        prepare_profile_switch(&profile_id, selected_node_tag.as_deref(), &config_content)?;

    let active_path = get_active_config_path(&app)?;
    let profiles_path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("profiles.json");
    let state = app.state::<crate::state::ProxyState>();
    let connected =
        state.get_status() == crate::state::ConnectionStatus::Connected && state.is_running();
    let previous_selected_tag = get_selected_outbound_tag(&app);
    let mode_override = state
        .effective_proxy_mode
        .lock()
        .map_err(|error| format!("state_lock_poisoned: {}", error))?
        .clone();

    let new_selected_tag = selected_node_tag.or_else(|| {
        serde_json::from_str::<serde_json::Value>(&final_config)
            .ok()
            .and_then(|config| {
                config
                    .get("outbounds")
                    .and_then(|outbounds| outbounds.as_array())
                    .and_then(|outbounds| {
                        outbounds.iter().find(|outbound| {
                            outbound.get("type").and_then(|value| value.as_str())
                                == Some("selector")
                                && outbound.get("tag").and_then(|value| value.as_str())
                                    == Some("proxy")
                        })
                    })
                    .and_then(|selector| selector.get("default"))
                    .and_then(|default| default.as_str())
                    .map(str::to_string)
            })
    });
    let mut metadata: serde_json::Value = if profiles_path.exists() {
        serde_json::from_slice(
            &std::fs::read(&profiles_path)
                .map_err(|error| format!("Failed to read profiles.json: {}", error))?,
        )
        .map_err(|error| format!("Failed to parse profiles.json: {}", error))?
    } else {
        serde_json::json!({})
    };
    metadata["activeProfileId"] = serde_json::Value::String(profile_id);
    let metadata = serde_json::to_vec_pretty(&metadata)
        .map_err(|error| format!("Failed to serialize profiles.json: {}", error))?;

    commit_and_apply_with_rollback(
        vec![
            (active_path, final_config.into_bytes()),
            (profiles_path, metadata),
        ],
        connected,
        |restoring| {
            crate::commands::proxy::try_reload_proxy_config_with_mode(
                &app,
                if restoring {
                    previous_selected_tag.clone()
                } else {
                    new_selected_tag.clone()
                },
                mode_override.clone(),
            )
        },
        || state.get_status() == crate::state::ConnectionStatus::Connected && state.is_running(),
    )
    .await
}

/// Returns the outbound nodes from a specific profile's config file.
#[tauri::command]
pub fn get_profile_outbounds(
    app: tauri::AppHandle,
    profile_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let config_path = get_profile_config_path(&app, &profile_id)?;
    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read profile config: {}", e))?;
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse profile config: {}", e))?;

    Ok(extract_nodes_from_config(&config))
}

pub fn get_active_profile_id(app: &tauri::AppHandle) -> Result<Option<String>, String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    path.push("profiles.json");
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read profiles.json: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse profiles.json: {}", e))?;
    if let Some(val) = json.get("activeProfileId") {
        if let Some(s) = val.as_str() {
            return Ok(Some(s.to_string()));
        }
        if let Some(s) = val.get("value").and_then(|v| v.as_str()) {
            return Ok(Some(s.to_string()));
        }
    }
    Ok(None)
}

/// Resolves and validates safe destination and temporary paths for a rule-set file within `rule_set_dir`.
/// Guarantees that neither path can escape the target `rule_set_dir` directory.
pub fn safe_rule_set_paths(
    rule_set_dir: &std::path::Path,
    tag: &str,
    format: &str,
) -> Result<(PathBuf, PathBuf), String> {
    crate::config::rules::validate_rule_set_tag(tag)?;

    let extension = match format {
        "binary" => "srs",
        "source" | "json" => "json",
        _ => return Err(format!("Unsupported rule-set format: {}", format)),
    };
    let filename = format!("{}.{}", tag, extension);
    let dest_path = rule_set_dir.join(&filename);

    let temp_filename = format!("{}.{}.{}.tmp", tag, extension, uuid::Uuid::new_v4());
    let temp_path = rule_set_dir.join(&temp_filename);

    for path in [&dest_path, &temp_path] {
        if !path.starts_with(rule_set_dir) {
            return Err("Rule-set destination path escapes rule_set directory".to_string());
        }
        if path.parent() != Some(rule_set_dir) {
            return Err(
                "Rule-set destination path is not an immediate child of rule_set directory"
                    .to_string(),
            );
        }
        for comp in path.components() {
            if matches!(comp, std::path::Component::ParentDir) {
                return Err(
                    "Rule-set destination path contains parent directory traversal".to_string(),
                );
            }
        }
    }

    Ok((dest_path, temp_path))
}

/// Atomically stages and commits downloaded rule-set bytes to disk after verifying path safety.
pub fn stage_and_commit_rule_set_content(
    rule_set_dir: &std::path::Path,
    tag: &str,
    format: &str,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    let (dest_path, temp_path) = safe_rule_set_paths(rule_set_dir, tag, format)?;

    std::fs::create_dir_all(rule_set_dir)
        .map_err(|e| format!("Failed to create rule_set directory: {}", e))?;

    std::fs::write(&temp_path, bytes)
        .map_err(|e| format!("Failed to write rule-set temporary file: {}", e))?;

    if let Err(e) = std::fs::rename(&temp_path, &dest_path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("Failed to commit rule-set file: {}", e));
    }

    Ok(dest_path)
}

#[tauri::command]
pub async fn update_rule_set(
    app: tauri::AppHandle,
    rule_set_id: String,
    url: String,
) -> Result<(), String> {
    // 1. Load rule sets from file to find tag and format
    let (_, rule_sets) = crate::config::rules::load_routing_rules_from_file(&app);
    let rule_set = rule_sets
        .iter()
        .find(|rs| rs.id == rule_set_id)
        .ok_or_else(|| format!("Rule set with ID '{}' not found", rule_set_id))?;

    let tag = &rule_set.tag;
    let format = &rule_set.format;

    // Validate tag and destination safety BEFORE network fetch and BEFORE filesystem writes
    crate::config::rules::validate_rule_set_tag(tag)?;

    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut rule_set_dir = app_data_dir;
    rule_set_dir.push("rule_set");
    safe_rule_set_paths(&rule_set_dir, tag, format)?;

    // 2. Fetch the file securely using SSRF-safe bounded fetcher
    let bytes = crate::config::subscription::fetch_subscription_content(&url)
        .await
        .map_err(|e| format!("Failed to download rule set: {}", e))?;

    if bytes.is_empty() {
        return Err("Downloaded rule set is empty".to_string());
    }

    // 3. Format validation
    if format == "source" || format == "json" {
        serde_json::from_slice::<serde_json::Value>(&bytes)
            .map_err(|e| format!("Rule set JSON format validation failed: {}", e))?;
    } else if format == "binary" && bytes.len() < 8 {
        return Err("Binary rule set file is truncated or invalid".to_string());
    }

    // 4. Atomically stage and commit to app_data_dir/rule_set/<tag>.<extension>
    stage_and_commit_rule_set_content(&rule_set_dir, tag, format, &bytes)?;

    Ok(())
}
