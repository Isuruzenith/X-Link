use tauri::{
    menu::{Menu, MenuItemBuilder, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Manager, Wry,
};

/// Safely unsets system proxy and terminates the sing-box process
pub fn perform_clean_cleanup(app: &AppHandle) {
    let state = app.state::<crate::state::ProxyState>();
    
    // 1. Kill the sing-box sidecar process if it's running
    if let Ok(mut process_lock) = state.child_process.lock() {
        if let Some(child) = process_lock.take() {
            let _ = child.kill();
        }
    }
    
    // 2. Clear OS proxy settings
    let _ = crate::os::disable_system_proxy();
}

/// Manually parses settings.json to check closeToTray setting
pub fn get_close_to_tray_setting(app: &AppHandle) -> bool {
    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("settings.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(close_to_tray) = val.get("closeToTray").and_then(|v| v.as_bool()) {
                        return close_to_tray;
                    }
                }
            }
        }
    }
    // Default to true for proxy apps
    true
}

/// Resolves the name of the currently active profile from profiles.json
fn get_active_profile_name(app: &AppHandle) -> String {
    let active_id = match crate::commands::proxy::get_active_profile_id() {
        Some(id) => id,
        None => return "Disconnected".to_string(),
    };

    if active_id == "default" {
        return "Default Outbound".to_string();
    }

    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("profiles.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    // tauri-plugin-store stores standard collections under the key 'profiles'
                    if let Some(profiles) = val.get("profiles").and_then(|p| p.as_array()) {
                        for p in profiles {
                            if let Some(id) = p.get("id").and_then(|i| i.as_str()) {
                                if id == active_id {
                                    if let Some(name) = p.get("name").and_then(|n| n.as_str()) {
                                        return name.to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    format!("Profile ({})", &active_id[0..std::cmp::min(8, active_id.len())])
}

/// Asynchronously handles the Connect/Disconnect toggle from the tray
async fn handle_toggle_proxy(app: AppHandle) -> Result<(), String> {
    let state = app.state::<crate::state::ProxyState>();
    let is_running = state.is_running();

    if is_running {
        crate::commands::proxy::toggle_proxy(app.clone(), state.clone(), false, "".to_string()).await?;
    } else {
        // Find the first available profile to connect to, fallback to "default"
        let mut profile_id = "default".to_string();
        if let Ok(mut path) = app.path().app_data_dir() {
            path.push("profiles.json");
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(profiles) = val.get("profiles").and_then(|p| p.as_array()) {
                            if let Some(first_profile) = profiles.first() {
                                if let Some(id) = first_profile.get("id").and_then(|i| i.as_str()) {
                                    profile_id = id.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
        crate::commands::proxy::toggle_proxy(app.clone(), state.clone(), true, profile_id).await?;
    }

    // Update tray menu state
    update_tray_menu(&app);
    Ok(())
}

/// Rebuilds the context-aware tray menu in a clean, state-driven manner
pub fn build_tray_menu(app: &AppHandle) -> Result<Menu<Wry>, String> {
    let is_running = {
        let state = app.state::<crate::state::ProxyState>();
        state.is_running()
    };

    let profile_name = if is_running {
        get_active_profile_name(app)
    } else {
        "Disconnected".to_string()
    };

    let info_text = format!("TunX — {}", profile_name);
    let toggle_text = if is_running { "Disconnect" } else { "Connect" };

    let menu = Menu::new(app).map_err(|e| e.to_string())?;
    
    let info_item = MenuItemBuilder::new(info_text).enabled(false).build(app).map_err(|e| e.to_string())?;
    let sep1 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let toggle_item = MenuItemBuilder::with_id("toggle", toggle_text).build(app).map_err(|e| e.to_string())?;
    let open_item = MenuItemBuilder::with_id("open", "Open Dashboard").build(app).map_err(|e| e.to_string())?;
    let sep2 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app).map_err(|e| e.to_string())?;

    menu.append(&info_item).map_err(|e| e.to_string())?;
    menu.append(&sep1).map_err(|e| e.to_string())?;
    menu.append(&toggle_item).map_err(|e| e.to_string())?;
    menu.append(&open_item).map_err(|e| e.to_string())?;
    menu.append(&sep2).map_err(|e| e.to_string())?;
    menu.append(&quit_item).map_err(|e| e.to_string())?;

    Ok(menu)
}

/// Programmatically builds the tray icon and attaches menu event callbacks
pub fn create_tray(app: &AppHandle) -> Result<TrayIcon, String> {
    let menu = build_tray_menu(app)?;

    let tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "toggle" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = handle_toggle_proxy(app_handle).await;
                    });
                }
                "open" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        perform_clean_cleanup(&app_handle);
                        app_handle.exit(0);
                    });
                }
                _ => {}
            }
        })
        .build(app)
        .map_err(|e| format!("Failed to build tray icon: {}", e))?;

    Ok(tray)
}

/// Triggers a dynamic redraw of the system tray menu from any state transition
pub fn update_tray_menu(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_tray_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}
