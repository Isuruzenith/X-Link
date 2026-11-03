use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::Manager;

/// Supported backup schema version.
pub const BACKUP_SCHEMA_VERSION: u32 = 1;
const MAX_BACKUP_BYTES: usize = 32 * 1024 * 1024;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BackupMetadata {
    pub schema_version: u32,
    pub app_version: String,
    pub exported_at: u64,
    pub platform: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBackupArchive {
    pub metadata: BackupMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profiles: Option<serde_json::Value>,
    pub profile_configs: HashMap<String, String>, // profile_id -> sing-box JSON string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BackupValidationResult {
    pub is_valid: bool,
    pub error: Option<String>,
    pub schema_version: u32,
    pub app_version: String,
    pub exported_at: u64,
    pub profile_count: usize,
    pub profile_names: Vec<String>,
    pub has_settings: bool,
    pub rule_count: usize,
    pub rule_set_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOptions {
    pub restore_settings: bool,
    pub restore_profiles: bool,
    pub restore_routing: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub success: bool,
    pub restored_profiles: usize,
    pub restored_rules: usize,
    pub message: String,
}

/// Accepts only ASCII alphanumeric/hyphen/underscore profile IDs, excluding reserved
/// config names and Windows device names regardless of case.
pub fn is_valid_profile_id(id: &str) -> bool {
    let trimmed = id.trim();
    if trimmed.is_empty() || trimmed.len() > 128 || trimmed != id {
        return false;
    }
    if trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('\0')
    {
        return false;
    }
    let stem = trimmed.to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    stem != "ACTIVE"
        && stem != "ACTIVE_TEMP"
        && !reserved.contains(&stem.as_str())
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Internal validation logic for a candidate backup archive.
pub fn validate_backup_archive_internal(
    archive: &ConfigBackupArchive,
) -> Result<BackupValidationResult, String> {
    if archive.metadata.schema_version != BACKUP_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported backup schema version {}. Maximum supported version is {}.",
            archive.metadata.schema_version, BACKUP_SCHEMA_VERSION
        ));
    }

    let archive_size = serde_json::to_vec(archive)
        .map_err(|error| format!("Failed to measure backup: {}", error))?
        .len();
    if archive_size > MAX_BACKUP_BYTES {
        return Err("Backup exceeds the 32 MB limit.".to_string());
    }

    if let Some(settings) = &archive.settings {
        let object = settings
            .as_object()
            .ok_or_else(|| "Backup settings must be a JSON object.".to_string())?;
        for key in ["proxyMode", "mixedPort", "httpPort", "socksPort", "apiPort"] {
            if let Some(value) = object.get(key) {
                let valid = if key.ends_with("Port") {
                    value
                        .as_u64()
                        .is_some_and(|port| (1..=65535).contains(&port))
                } else {
                    value.is_string()
                };
                if !valid {
                    return Err(format!("Invalid backup setting '{}'.", key));
                }
            }
        }
        let parsed = serde_json::from_value::<crate::state::UserSettings>(settings.clone())
            .map_err(|error| format!("Invalid backup settings: {}", error))?;
        if !["system", "tun"].contains(&parsed.proxy_mode.as_str())
            || !["dark", "light", "system"].contains(
                &object
                    .get("theme")
                    .and_then(|value| value.as_str())
                    .unwrap_or("dark"),
            )
        {
            return Err("Backup contains an unsupported proxy mode or theme.".to_string());
        }
        if parsed.api_enabled && parsed.api_secret.trim().is_empty() {
            return Err("Backup enables the API without an authentication secret.".to_string());
        }
    }

    let mut profile_names = Vec::new();
    let mut profile_ids = HashSet::new();
    let mut normalized_ids = HashSet::new();
    if let Some(profiles_val) = &archive.profiles {
        let profiles = profiles_val
            .get("profiles")
            .and_then(|value| value.as_array())
            .ok_or_else(|| "Backup profiles must contain a profiles array.".to_string())?;
        for (idx, profile) in profiles.iter().enumerate() {
            let id = profile
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| format!("Profile at index {} is missing an 'id' field.", idx))?;
            if !is_valid_profile_id(id) || !normalized_ids.insert(id.to_ascii_uppercase()) {
                return Err(format!(
                    "Profile at index {} has an unsafe or duplicate profile ID: '{}'.",
                    idx, id
                ));
            }
            profile_ids.insert(id.to_string());
            let name = profile
                .get("name")
                .and_then(|value| value.as_str())
                .ok_or_else(|| format!("Profile '{}' is missing a name.", id))?;
            let profile_type = profile
                .get("type")
                .and_then(|value| value.as_str())
                .ok_or_else(|| format!("Profile '{}' is missing a type.", id))?;
            if !["manual", "file", "subscription"].contains(&profile_type) {
                return Err(format!("Profile '{}' has an unsupported type.", id));
            }
            profile_names.push(name.to_string());
        }
        if let Some(active_id) = profiles_val
            .get("activeProfileId")
            .and_then(|value| value.as_str())
        {
            if !profile_ids.contains(active_id) {
                return Err("Backup activeProfileId is not present in profiles.".to_string());
            }
        } else if !profile_ids.is_empty() {
            return Err("Backup with profiles requires an activeProfileId.".to_string());
        }
        if profile_ids.len() != archive.profile_configs.len() {
            return Err(
                "Backup must contain a configuration for every listed profile.".to_string(),
            );
        }
    } else if !archive.profile_configs.is_empty() {
        return Err("Backup configurations require profiles metadata.".to_string());
    }

    let mut config_normalized_ids = HashSet::new();
    for (profile_id, config_str) in &archive.profile_configs {
        if !is_valid_profile_id(profile_id)
            || !config_normalized_ids.insert(profile_id.to_ascii_uppercase())
        {
            return Err(format!(
                "Invalid or duplicate profile ID in configurations: '{}'.",
                profile_id
            ));
        }
        if !profile_ids.contains(profile_id) {
            return Err(format!(
                "Configuration for '{}' is not listed in profiles.",
                profile_id
            ));
        }
        let parsed: serde_json::Value = serde_json::from_str(config_str).map_err(|e| {
            format!(
                "Profile configuration for '{}' is not valid JSON: {}",
                profile_id, e
            )
        })?;
        let outbounds = parsed
            .get("outbounds")
            .and_then(|value| value.as_array())
            .ok_or_else(|| {
                format!(
                    "Profile configuration for '{}' does not contain any outbounds.",
                    profile_id
                )
            })?;
        if outbounds.is_empty() {
            return Err(format!(
                "Profile configuration for '{}' does not contain any outbounds.",
                profile_id
            ));
        }
        for outbound in outbounds {
            if outbound
                .get("type")
                .and_then(|value| value.as_str())
                .is_none()
                || outbound
                    .get("tag")
                    .and_then(|value| value.as_str())
                    .is_none()
            {
                return Err(format!(
                    "Profile configuration for '{}' has an invalid outbound.",
                    profile_id
                ));
            }
        }
    }

    let (rule_count, rule_set_count) = if let Some(routing) = &archive.routing {
        let rules = routing
            .get("rules")
            .and_then(|value| value.as_array())
            .ok_or_else(|| "Backup routing must contain a rules array.".to_string())?;
        let rule_sets = routing
            .get("ruleSets")
            .and_then(|value| value.as_array())
            .ok_or_else(|| "Backup routing must contain a ruleSets array.".to_string())?;
        serde_json::from_value::<Vec<crate::config::rules::RoutingRule>>(serde_json::Value::Array(
            rules.clone(),
        ))
        .map_err(|error| format!("Invalid routing rules: {}", error))?;
        let parsed_sets = serde_json::from_value::<Vec<crate::config::rules::RuleSet>>(
            serde_json::Value::Array(rule_sets.clone()),
        )
        .map_err(|error| format!("Invalid routing rule sets: {}", error))?;
        for rule_set in parsed_sets {
            crate::config::rules::validate_rule_set_tag(&rule_set.tag)?;
            if !["local", "remote"].contains(&rule_set.set_type.as_str())
                || !["binary", "source"].contains(&rule_set.format.as_str())
            {
                return Err(format!(
                    "Rule set '{}' has an unsupported type or format.",
                    rule_set.tag
                ));
            }
            if rule_set.set_type == "remote"
                && rule_set
                    .url
                    .as_deref()
                    .is_none_or(|url| !url.starts_with("https://"))
            {
                return Err(format!(
                    "Remote rule set '{}' requires an HTTPS URL.",
                    rule_set.tag
                ));
            }
        }
        (rules.len(), rule_sets.len())
    } else {
        (0, 0)
    };

    let has_settings = archive.settings.is_some();

    Ok(BackupValidationResult {
        is_valid: true,
        error: None,
        schema_version: archive.metadata.schema_version,
        app_version: archive.metadata.app_version.clone(),
        exported_at: archive.metadata.exported_at,
        profile_count: profile_ids.len(),
        profile_names,
        has_settings,
        rule_count,
        rule_set_count,
    })
}

struct StagedRestore {
    target: PathBuf,
    temporary: PathBuf,
    previous: PathBuf,
    existed: bool,
    remove: bool,
}

fn check_app_data_dir(path: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("Failed to check app data directory: {}", error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("Backup app data destination is not a regular directory.".to_string());
    }
    Ok(())
}

fn check_safe_path(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(format!(
            "Backup destination '{}' is not a regular file.",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to check '{}': {}", path.display(), error)),
    }
}

fn cleanup_staged(staged: &[StagedRestore]) {
    for entry in staged {
        let _ = std::fs::remove_file(&entry.temporary);
        let _ = std::fs::remove_file(&entry.previous);
    }
}

fn rollback_staged(staged: &[StagedRestore], completed: usize) -> Vec<String> {
    let mut errors = Vec::new();
    for entry in staged[..completed].iter().rev() {
        let result = if entry.existed {
            std::fs::rename(&entry.previous, &entry.target)
        } else if entry.remove {
            Ok(())
        } else {
            std::fs::remove_file(&entry.target)
        };
        if let Err(error) = result {
            errors.push(format!(
                "Failed to roll back '{}': {}",
                entry.target.display(),
                error
            ));
        }
    }
    errors
}

fn commit_restore_files_with<F>(
    files: Vec<(PathBuf, Option<Vec<u8>>)>,
    mut rename: F,
) -> Result<(), String>
where
    F: FnMut(&Path, &Path) -> std::io::Result<()>,
{
    if files
        .iter()
        .map(|(path, _)| path)
        .collect::<HashSet<_>>()
        .len()
        != files.len()
    {
        return Err("Backup restore contains duplicate destination paths.".to_string());
    }
    let mut staged = Vec::new();
    for (target, content) in files {
        if let Err(error) = check_safe_path(&target) {
            cleanup_staged(&staged);
            return Err(error);
        }
        let suffix = uuid::Uuid::new_v4();
        let temporary = target.with_extension(format!("{}.tmp", suffix));
        let previous = target.with_extension(format!("{}.bak", suffix));
        if let Some(ref bytes) = content {
            let staged_write = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .and_then(|mut file| file.write_all(bytes));
            if let Err(error) = staged_write {
                let _ = std::fs::remove_file(&temporary);
                cleanup_staged(&staged);
                return Err(format!("Failed to stage '{}': {}", target.display(), error));
            }
        }
        let existed = target.exists();
        if existed {
            let preserve = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&previous)
                .and_then(|mut file| {
                    let mut original = std::fs::File::open(&target)?;
                    std::io::copy(&mut original, &mut file)?;
                    Ok(())
                });
            if let Err(error) = preserve {
                let _ = std::fs::remove_file(&temporary);
                let _ = std::fs::remove_file(&previous);
                cleanup_staged(&staged);
                return Err(format!(
                    "Failed to preserve '{}': {}",
                    target.display(),
                    error
                ));
            }
        }
        staged.push(StagedRestore {
            target,
            temporary,
            previous,
            existed,
            remove: content.is_none(),
        });
    }

    for index in 0..staged.len() {
        if let Err(error) = check_safe_path(&staged[index].target) {
            let rollback_errors = rollback_staged(&staged, index);
            if rollback_errors.is_empty() {
                cleanup_staged(&staged);
            } else {
                for entry in &staged {
                    let _ = std::fs::remove_file(&entry.temporary);
                }
            }
            return Err(format!(
                "{}{}",
                error,
                if rollback_errors.is_empty() {
                    String::new()
                } else {
                    format!("; rollback incomplete: {}", rollback_errors.join("; "))
                }
            ));
        }
        let result = if staged[index].remove {
            if staged[index].existed {
                std::fs::remove_file(&staged[index].target)
            } else {
                Ok(())
            }
        } else {
            rename(&staged[index].temporary, &staged[index].target)
        };
        if let Err(error) = result {
            let rollback_errors = rollback_staged(&staged, index);
            if rollback_errors.is_empty() {
                cleanup_staged(&staged);
            } else {
                for entry in &staged {
                    let _ = std::fs::remove_file(&entry.temporary);
                }
            }
            return Err(format!(
                "Failed to commit restore: {}{}",
                error,
                if rollback_errors.is_empty() {
                    String::new()
                } else {
                    format!("; rollback incomplete: {}", rollback_errors.join("; "))
                }
            ));
        }
    }
    cleanup_staged(&staged);
    Ok(())
}

/// Restore selected archive sections without writing over existing files in place.
pub fn apply_backup_restore(
    app_data_dir: &Path,
    archive: &ConfigBackupArchive,
    options: &RestoreOptions,
) -> Result<RestoreResult, String> {
    let validation = validate_backup_archive_internal(archive)?;
    check_app_data_dir(app_data_dir)?;
    if !options.restore_profiles && !options.restore_settings && !options.restore_routing {
        return Err("Select at least one section to restore.".to_string());
    }
    let configs_dir = app_data_dir.join("configs");
    if options.restore_profiles && archive.profiles.is_some() {
        match std::fs::symlink_metadata(&configs_dir) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err("Backup configs destination is not a regular directory.".to_string());
            }
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&configs_dir)
                    .map_err(|e| format!("Failed to create configs directory: {}", e))?;
            }
            Err(error) => return Err(format!("Failed to check configs directory: {}", error)),
        }
    }

    let mut files: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
    if options.restore_profiles {
        if let Some(profiles) = &archive.profiles {
            // Install profile configurations before exposing the restored metadata.
            for (id, config) in &archive.profile_configs {
                files.push((
                    configs_dir.join(format!("{}.json", id)),
                    Some(config.as_bytes().to_vec()),
                ));
            }
            if let Some(active_id) = profiles
                .get("activeProfileId")
                .and_then(|value| value.as_str())
            {
                if let Some(config) = archive.profile_configs.get(active_id) {
                    files.push((
                        configs_dir.join("active.json"),
                        Some(config.as_bytes().to_vec()),
                    ));
                }
            } else {
                files.push((configs_dir.join("active.json"), None));
            }
            files.push((
                app_data_dir.join("profiles.json"),
                Some(serde_json::to_vec_pretty(profiles).map_err(|e| e.to_string())?),
            ));
        }
    }
    if options.restore_settings {
        if let Some(settings) = &archive.settings {
            files.push((
                app_data_dir.join("settings.json"),
                Some(serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?),
            ));
        }
    }
    if options.restore_routing {
        if let Some(routing) = &archive.routing {
            files.push((
                app_data_dir.join("routing.json"),
                Some(serde_json::to_vec_pretty(routing).map_err(|e| e.to_string())?),
            ));
        }
    }
    if files.is_empty() {
        return Err("Selected sections are not present in this backup.".to_string());
    }
    for (path, _) in &files {
        check_safe_path(path)?;
    }
    commit_restore_files_with(files, |from, to| std::fs::rename(from, to))?;

    let restored_profiles = if options.restore_profiles {
        validation.profile_count
    } else {
        0
    };
    let restored_rules = if options.restore_routing {
        validation.rule_count
    } else {
        0
    };
    Ok(RestoreResult {
        success: true,
        restored_profiles,
        restored_rules,
        message: format!(
            "Restored {} profile(s) and {} routing rule(s). Restart the app before reconnecting.",
            restored_profiles, restored_rules
        ),
    })
}

/// Export a complete, validated backup archive of the current X-Link configuration.
#[tauri::command]
pub async fn export_config_backup(app: tauri::AppHandle) -> Result<ConfigBackupArchive, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let metadata = BackupMetadata {
        schema_version: BACKUP_SCHEMA_VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: now_ms,
        platform: std::env::consts::OS.to_string(),
    };

    // 1. Read settings.json
    let settings_path = app_data_dir.join("settings.json");
    let settings = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
        Some(serde_json::from_str(&content).map_err(|e| format!("Invalid settings.json: {}", e))?)
    } else {
        None
    };

    // 2. Read profiles.json and scan profile configs
    let profiles_path = app_data_dir.join("profiles.json");
    let mut profile_configs = HashMap::new();
    let profiles = if profiles_path.exists() {
        let content = std::fs::read_to_string(&profiles_path).map_err(|e| e.to_string())?;
        let parsed: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| format!("Invalid profiles.json: {}", e))?;
        let listed = parsed
            .get("profiles")
            .and_then(|p| p.as_array())
            .ok_or_else(|| "profiles.json is missing its profiles array.".to_string())?;
        let configs_dir = app_data_dir.join("configs");
        for profile in listed {
            let id = profile
                .get("id")
                .and_then(|i| i.as_str())
                .ok_or_else(|| "profiles.json contains a profile without an ID.".to_string())?;
            if !is_valid_profile_id(id) {
                return Err(format!("Unsafe profile ID in profiles.json: '{}'", id));
            }
            let config_file = configs_dir.join(format!("{}.json", id));
            let config = std::fs::read_to_string(&config_file)
                .map_err(|e| format!("Failed to export config for '{}': {}", id, e))?;
            profile_configs.insert(id.to_string(), config);
        }
        Some(parsed)
    } else {
        None
    };

    // 3. Read routing.json
    let routing_path = app_data_dir.join("routing.json");
    let routing = if routing_path.exists() {
        let content = std::fs::read_to_string(&routing_path).map_err(|e| e.to_string())?;
        Some(serde_json::from_str(&content).map_err(|e| format!("Invalid routing.json: {}", e))?)
    } else {
        None
    };

    let archive = ConfigBackupArchive {
        metadata,
        settings,
        profiles,
        profile_configs,
        routing,
    };
    validate_backup_archive_internal(&archive)?;
    Ok(archive)
}

/// Validate a candidate backup archive without applying any changes.
#[tauri::command]
pub fn validate_backup_archive(
    archive: ConfigBackupArchive,
) -> Result<BackupValidationResult, String> {
    validate_backup_archive_internal(&archive)
}

/// Restore selected configuration files, rolling back committed files on a write failure.
#[tauri::command]
pub async fn import_config_backup(
    app: tauri::AppHandle,
    archive: ConfigBackupArchive,
    options: RestoreOptions,
) -> Result<RestoreResult, String> {
    // Refresh holds the subscription lock while it restarts a connected proxy, which
    // acquires the toggle lock; keep the same lock order here to avoid deadlock.
    let _subscription_lock = crate::commands::config::SUBSCRIPTION_LOCK.lock().await;
    let _connection_lock = crate::commands::proxy::get_toggle_lock().lock().await;
    let state = app.state::<crate::state::ProxyState>();
    if state.get_status() != crate::state::ConnectionStatus::Disconnected || state.is_running() {
        return Err(
            "Disconnect the proxy before restoring a backup. Restart the app after restoring."
                .to_string(),
        );
    }
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let result = apply_backup_restore(&app_data_dir, &archive, &options)?;

    // If settings were restored, update in-memory state.
    if options.restore_settings && archive.settings.is_some() {
        state.reload_settings(&app)?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_valid_outbound_json() -> String {
        r#"{
            "outbounds": [
                {
                    "type": "vmess",
                    "tag": "proxy-1",
                    "server": "1.2.3.4",
                    "server_port": 443,
                    "uuid": "00000000-0000-0000-0000-000000000000"
                }
            ]
        }"#
        .to_string()
    }

    #[test]
    fn test_is_valid_profile_id_rules() {
        assert!(is_valid_profile_id("default-profile"));
        assert!(is_valid_profile_id("profile_1234"));
        assert!(is_valid_profile_id("01234567-89ab-cdef-0123-456789abcdef"));

        // Reject path traversal and special characters
        assert!(!is_valid_profile_id("../secret"));
        assert!(!is_valid_profile_id("foo/bar"));
        assert!(!is_valid_profile_id("foo\\bar"));
        assert!(!is_valid_profile_id("profile..1"));
        assert!(!is_valid_profile_id(""));
        assert!(!is_valid_profile_id(" "));
        assert!(!is_valid_profile_id("foo;rm -rf /"));
        assert!(!is_valid_profile_id("foo\0bar"));
        assert!(!is_valid_profile_id("active"));
        assert!(!is_valid_profile_id("active_temp"));
        assert!(!is_valid_profile_id("ACTIVE"));
        assert!(!is_valid_profile_id("Active_Temp"));
        assert!(!is_valid_profile_id("CON"));
        assert!(!is_valid_profile_id("com1"));
    }

    #[test]
    fn test_validate_backup_archive_success() {
        let mut configs = HashMap::new();
        configs.insert("p1".to_string(), sample_valid_outbound_json());

        let archive = ConfigBackupArchive {
            metadata: BackupMetadata {
                schema_version: 1,
                app_version: "0.8.0".to_string(),
                exported_at: 1730000000,
                platform: "windows".to_string(),
            },
            settings: Some(serde_json::json!({ "proxyMode": "system", "mixedPort": 2080 })),
            profiles: Some(serde_json::json!({
                "profiles": [
                    { "id": "p1", "name": "Primary Subscription", "type": "subscription", "subscriptionUrl": "https://example.com/sub" }
                ],
                "activeProfileId": "p1"
            })),
            profile_configs: configs,
            routing: Some(serde_json::json!({
                "rules": [{ "id": "r1", "type": "geoip", "value": "private", "outbound": "direct", "invert": false }],
                "ruleSets": []
            })),
        };

        let res = validate_backup_archive_internal(&archive).expect("Should be valid");
        assert!(res.is_valid);
        assert_eq!(res.profile_count, 1);
        assert_eq!(res.profile_names, vec!["Primary Subscription"]);
        assert!(res.has_settings);
        assert_eq!(res.rule_count, 1);
        assert_eq!(res.rule_set_count, 0);
    }

    #[test]
    fn test_validate_backup_archive_rejects_unsafe_profile_id() {
        let mut configs = HashMap::new();
        configs.insert("../../traversal".to_string(), sample_valid_outbound_json());

        let archive = ConfigBackupArchive {
            metadata: BackupMetadata {
                schema_version: 1,
                app_version: "0.8.0".to_string(),
                exported_at: 1730000000,
                platform: "windows".to_string(),
            },
            settings: None,
            profiles: None,
            profile_configs: configs,
            routing: None,
        };

        let err = validate_backup_archive_internal(&archive).unwrap_err();
        assert!(err.contains("profiles metadata"));
    }

    #[test]
    fn test_validate_backup_archive_rejects_empty_outbounds() {
        let mut configs = HashMap::new();
        configs.insert("p1".to_string(), r#"{"outbounds": []}"#.to_string());

        let archive = ConfigBackupArchive {
            metadata: BackupMetadata {
                schema_version: 1,
                app_version: "0.8.0".to_string(),
                exported_at: 1730000000,
                platform: "windows".to_string(),
            },
            settings: None,
            profiles: Some(
                serde_json::json!({ "profiles": [{ "id": "p1", "name": "P1", "type": "manual" }], "activeProfileId": "p1" }),
            ),
            profile_configs: configs,
            routing: None,
        };

        let err = validate_backup_archive_internal(&archive).unwrap_err();
        assert!(err.contains("does not contain any outbounds"));
    }

    #[test]
    fn test_apply_backup_restore_and_rollback() {
        let temp_dir =
            std::env::temp_dir().join(format!("xlink_backup_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let initial_settings = temp_dir.join("settings.json");
        std::fs::write(&initial_settings, b"original-settings").unwrap();

        let mut configs = HashMap::new();
        configs.insert("profile_a".to_string(), sample_valid_outbound_json());

        let archive = ConfigBackupArchive {
            metadata: BackupMetadata {
                schema_version: 1,
                app_version: "0.8.0".to_string(),
                exported_at: 1730000000,
                platform: "windows".to_string(),
            },
            settings: Some(serde_json::json!({ "proxyMode": "tun" })),
            profiles: Some(serde_json::json!({
                "profiles": [{ "id": "profile_a", "name": "Profile A", "type": "manual" }],
                "activeProfileId": "profile_a"
            })),
            profile_configs: configs,
            routing: None,
        };

        let options = RestoreOptions {
            restore_settings: true,
            restore_profiles: true,
            restore_routing: false,
        };

        let res = apply_backup_restore(&temp_dir, &archive, &options).unwrap();
        assert!(res.success);
        assert_eq!(res.restored_profiles, 1);

        // Verify files written
        let config_a = temp_dir.join("configs").join("profile_a.json");
        assert!(config_a.exists());
        let read_config = std::fs::read_to_string(&config_a).unwrap();
        assert!(read_config.contains("proxy-1"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_empty_profile_restore_removes_stale_active_config() {
        let dir = std::env::temp_dir().join(format!(
            "xlink_backup_empty_profiles_{}",
            uuid::Uuid::new_v4()
        ));
        let configs = dir.join("configs");
        std::fs::create_dir_all(&configs).unwrap();
        let active = configs.join("active.json");
        std::fs::write(&active, b"old active config").unwrap();
        let archive = ConfigBackupArchive {
            metadata: BackupMetadata {
                schema_version: 1,
                app_version: "0.8.0".into(),
                exported_at: 1,
                platform: "windows".into(),
            },
            settings: None,
            profiles: Some(serde_json::json!({ "profiles": [], "activeProfileId": null })),
            profile_configs: HashMap::new(),
            routing: None,
        };
        apply_backup_restore(
            &dir,
            &archive,
            &RestoreOptions {
                restore_settings: false,
                restore_profiles: true,
                restore_routing: false,
            },
        )
        .unwrap();
        assert!(!active.exists());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                &std::fs::read(dir.join("profiles.json")).unwrap()
            )
            .unwrap()["profiles"],
            serde_json::json!([])
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_empty_profile_restore_rolls_back_removed_active_config() {
        let dir = std::env::temp_dir().join(format!(
            "xlink_backup_empty_rollback_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let active = dir.join("active.json");
        let profiles = dir.join("profiles.json");
        std::fs::write(&active, b"old active config").unwrap();
        std::fs::write(&profiles, b"old profiles").unwrap();
        let error = commit_restore_files_with(
            vec![
                (active.clone(), None),
                (profiles.clone(), Some(b"new profiles".to_vec())),
            ],
            |_, _| Err(std::io::Error::other("injected failure")),
        )
        .unwrap_err();
        assert!(error.contains("injected failure"));
        assert_eq!(std::fs::read(&active).unwrap(), b"old active config");
        assert_eq!(std::fs::read(&profiles).unwrap(), b"old profiles");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_empty_profile_restore_rolls_back_absent_active_config() {
        let dir = std::env::temp_dir().join(format!(
            "xlink_backup_absent_active_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let active = dir.join("active.json");
        let profiles = dir.join("profiles.json");
        std::fs::write(&profiles, b"old profiles").unwrap();
        let error = commit_restore_files_with(
            vec![
                (active.clone(), None),
                (profiles.clone(), Some(b"new profiles".to_vec())),
            ],
            |_, _| Err(std::io::Error::other("injected failure")),
        )
        .unwrap_err();
        assert!(error.contains("injected failure"));
        assert!(!active.exists());
        assert_eq!(std::fs::read(&profiles).unwrap(), b"old profiles");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_restore_rollback_on_second_commit_failure() {
        let dir =
            std::env::temp_dir().join(format!("xlink_backup_rollback_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = dir.join("one.json");
        let second = dir.join("two.json");
        std::fs::write(&first, b"original one").unwrap();
        std::fs::write(&second, b"original two").unwrap();
        let mut calls = 0;
        let error = commit_restore_files_with(
            vec![
                (first.clone(), Some(b"updated one".to_vec())),
                (second.clone(), Some(b"updated two".to_vec())),
            ],
            |from, to| {
                calls += 1;
                if calls == 2 {
                    return Err(std::io::Error::other("injected failure"));
                }
                std::fs::rename(from, to)
            },
        )
        .unwrap_err();
        assert!(error.contains("injected failure"));
        assert_eq!(std::fs::read(&first).unwrap(), b"original one");
        assert_eq!(std::fs::read(&second).unwrap(), b"original two");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_rejects_invalid_outbound_and_active_profile() {
        let mut archive = ConfigBackupArchive {
            metadata: BackupMetadata {
                schema_version: 1,
                app_version: "0.8.0".into(),
                exported_at: 1,
                platform: "windows".into(),
            },
            settings: None,
            profiles: Some(
                serde_json::json!({ "profiles": [{ "id": "p1", "name": "P1", "type": "manual" }], "activeProfileId": "p1" }),
            ),
            profile_configs: HashMap::from([(
                "p1".into(),
                r#"{"outbounds":[{"type":"vmess"}]}"#.into(),
            )]),
            routing: None,
        };
        assert!(validate_backup_archive_internal(&archive)
            .unwrap_err()
            .contains("invalid outbound"));
        archive
            .profile_configs
            .insert("p1".into(), sample_valid_outbound_json());
        archive.profiles.as_mut().unwrap()["activeProfileId"] = serde_json::json!("missing");
        assert!(validate_backup_archive_internal(&archive)
            .unwrap_err()
            .contains("activeProfileId"));
    }

    #[test]
    fn test_rejects_mismatched_profile_configs() {
        let archive = ConfigBackupArchive {
            metadata: BackupMetadata {
                schema_version: 1,
                app_version: "0.8.0".into(),
                exported_at: 1,
                platform: "windows".into(),
            },
            settings: None,
            profiles: Some(
                serde_json::json!({ "profiles": [{ "id": "p1", "name": "P1", "type": "manual" }], "activeProfileId": "p1" }),
            ),
            profile_configs: HashMap::from([("p2".into(), sample_valid_outbound_json())]),
            routing: None,
        };
        assert!(validate_backup_archive_internal(&archive)
            .unwrap_err()
            .contains("not listed in profiles"));
    }
}
