use crate::commands::proxy::load_tun_settings;
use crate::config::adapters::adapt;
use crate::config::generator::generate_singbox_config;
use crate::config::validator::validate_singbox_config;
use std::path::PathBuf;
use std::time::SystemTime;
use tauri::{Manager, State};

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
    let outbounds = adapt(&bytes)?;
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

    // 3. Fetch remote subscription content via bounded HTTPS client with SSRF protection
    let bytes = crate::config::subscription::fetch_subscription_content(&sub_url).await?;

    // 4. Adapt/parse subscription content into outbounds
    let outbounds = adapt(&bytes)?;
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

    // 7. Determine selected node tag: preserve existing tag if still present; otherwise fallback to first valid tag
    let valid_tags: Vec<String> = outbounds.iter().map(|o| o.tag.clone()).collect();

    let new_selected_tag: Option<String> = match existing_selected_tag.as_ref() {
        Some(tag) if valid_tags.contains(tag) => Some(tag.clone()),
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
                    if ob.get("type").and_then(|t| t.as_str()) == Some("selector") {
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
}

/// Returns the customizable proxy outbounds (i.e. not selector/direct/block/dns)
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

    let outbounds = config
        .get("outbounds")
        .and_then(|o| o.as_array())
        .ok_or_else(|| "No outbounds array found in configuration".to_string())?;

    let mut nodes = Vec::new();
    for outbound in outbounds {
        let outbound_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if outbound_type != "selector"
            && outbound_type != "direct"
            && outbound_type != "block"
            && outbound_type != "dns"
        {
            nodes.push(outbound.clone());
        }
    }
    Ok(nodes)
}

/// Returns the outbound the "proxy" selector currently defaults to (i.e. the
/// node that will actually be used to connect), falling back to the first
/// customizable outbound if no default has been set yet.
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

    let outbounds = config
        .get("outbounds")
        .and_then(|o| o.as_array())
        .ok_or_else(|| "No outbounds array found in configuration".to_string())?;

    if let Some(selector) = outbounds
        .iter()
        .find(|o| o.get("type").and_then(|t| t.as_str()) == Some("selector"))
    {
        if let Some(default_tag) = selector.get("default").and_then(|d| d.as_str()) {
            if let Some(found) = outbounds
                .iter()
                .find(|o| o.get("tag").and_then(|t| t.as_str()) == Some(default_tag))
            {
                return Ok(found.clone());
            }
        }
    }

    for outbound in outbounds {
        let outbound_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if outbound_type != "selector"
            && outbound_type != "direct"
            && outbound_type != "block"
            && outbound_type != "dns"
        {
            return Ok(outbound.clone());
        }
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
    let selector = outbounds
        .iter()
        .find(|o| o.get("type").and_then(|t| t.as_str()) == Some("selector"))?;
    selector
        .get("default")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string())
}

/// Edits an existing node's connection parameters in-place (used by the node
/// editor). If connected, validates and cold-restarts sing-box with the updated config.
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

    let target_tag = new_outbound
        .get("tag")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());
    let mut target_idx = None;
    let mut old_tag: Option<String> = None;

    {
        let outbounds = config
            .get_mut("outbounds")
            .and_then(|o| o.as_array_mut())
            .ok_or_else(|| "No outbounds array found in configuration".to_string())?;

        // Prefer an exact tag match
        for (idx, outbound) in outbounds.iter().enumerate() {
            let t = outbound.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if t == "selector" || t == "direct" || t == "block" || t == "dns" {
                continue;
            }
            let tag = outbound.get("tag").and_then(|v| v.as_str());
            if target_tag.as_deref() == tag {
                target_idx = Some(idx);
                old_tag = tag.map(|s| s.to_string());
                break;
            }
        }

        // Fall back to the first customizable outbound (single-node configs)
        if target_idx.is_none() {
            for (idx, outbound) in outbounds.iter().enumerate() {
                let t = outbound.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if t != "selector" && t != "direct" && t != "block" && t != "dns" {
                    target_idx = Some(idx);
                    old_tag = outbound
                        .get("tag")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    break;
                }
            }
        }
    }

    let target_idx =
        target_idx.ok_or_else(|| "No matching outbound found to update".to_string())?;

    {
        let outbounds = config
            .get_mut("outbounds")
            .and_then(|o| o.as_array_mut())
            .unwrap();
        outbounds[target_idx] = new_outbound.clone();
    }

    // If the tag changed, keep the selector's outbound list / default in sync
    if let (Some(old), Some(new)) = (old_tag, new_outbound.get("tag").and_then(|t| t.as_str())) {
        if old != new {
            if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
                for outbound in outbounds.iter_mut() {
                    if outbound.get("type").and_then(|t| t.as_str()) == Some("selector") {
                        if let Some(list) =
                            outbound.get_mut("outbounds").and_then(|o| o.as_array_mut())
                        {
                            for item in list.iter_mut() {
                                if item.as_str() == Some(old.as_str()) {
                                    *item = serde_json::Value::String(new.to_string());
                                }
                            }
                        }
                        if outbound.get("default").and_then(|d| d.as_str()) == Some(old.as_str()) {
                            outbound["default"] = serde_json::Value::String(new.to_string());
                        }
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
                && !matches!(
                    outbound.get("type").and_then(|value| value.as_str()),
                    Some("selector" | "direct" | "block" | "dns")
                )
        }) {
            return Err(format!("Selected node not found in profile {}", profile_id));
        }
        for outbound in outbounds {
            if outbound.get("type").and_then(|value| value.as_str()) == Some("selector") {
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

    let outbounds = config
        .get("outbounds")
        .and_then(|o| o.as_array())
        .ok_or_else(|| "No outbounds array found in profile config".to_string())?;

    let mut nodes = Vec::new();
    for outbound in outbounds {
        let outbound_type = outbound.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if outbound_type != "selector"
            && outbound_type != "direct"
            && outbound_type != "block"
            && outbound_type != "dns"
        {
            nodes.push(outbound.clone());
        }
    }
    Ok(nodes)
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

    // 2. Fetch the file from the URL
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build http client: {}", e))?;

    let response = client
        .get(&url)
        .header("User-Agent", "xlink-client")
        .send()
        .await
        .map_err(|e| format!("Failed to download rule set: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Server returned status error: {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response bytes: {}", e))?;

    // 3. Save it to app_data_dir/rule_set/<tag>.<extension>
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut rule_set_dir = app_data_dir.clone();
    rule_set_dir.push("rule_set");
    std::fs::create_dir_all(&rule_set_dir)
        .map_err(|e| format!("Failed to create rule_set directory: {}", e))?;

    let extension = if format == "binary" { "srs" } else { "json" };
    let mut dest_path = rule_set_dir;
    dest_path.push(format!("{}.{}", tag, extension));

    std::fs::write(&dest_path, &bytes)
        .map_err(|e| format!("Failed to write rule-set cache file: {}", e))?;

    Ok(())
}
