use tauri::{
    menu::{CheckMenuItemBuilder, Menu, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Emitter, Manager, Wry,
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

    // 2. Clear active session ID
    if let Ok(mut session_lock) = state.active_session_id.lock() {
        *session_lock = None;
    }

    // 3. Clear OS proxy settings
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

/// Resolves the name of the currently active config from profiles.json
fn get_active_profile_name(app: &AppHandle) -> String {
    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("profiles.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(name) = val
                        .get("activeConfig")
                        .and_then(|c| c.get("name"))
                        .and_then(|n| n.as_str())
                    {
                        return name.to_string();
                    }
                }
            }
        }
    }

    "Active Config".to_string()
}

/// Returns list of (tag, type) pairs from the active config
fn get_node_tags_from_active_config(app: &AppHandle) -> Vec<(String, String)> {
    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("active.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(outbounds) = val.get("outbounds").and_then(|o| o.as_array()) {
                        let skip = ["direct", "block", "dns-out", "auto-select"];
                        return outbounds
                            .iter()
                            .filter_map(|o| {
                                let tag = o.get("tag")?.as_str()?;
                                let otype = o.get("type")?.as_str()?;
                                if skip.contains(&tag)
                                    || ["direct", "block", "dns", "selector", "urltest"]
                                        .contains(&otype)
                                {
                                    None
                                } else {
                                    Some((tag.to_string(), otype.to_string()))
                                }
                            })
                            .take(10)
                            .collect();
                    }
                }
            }
        }
    }
    Vec::new()
}

/// Gets the currently selected outbound tag from the ProxyState
fn get_selected_node_tag(app: &AppHandle) -> Option<String> {
    let state = app.state::<crate::state::ProxyState>();
    let result = if let Ok(lock) = state.selected_outbound_tag.lock() {
        lock.clone()
    } else {
        None
    };
    result
}

/// Reads the current proxyMode from settings.json ("system" or "tun")
fn get_current_proxy_mode(app: &AppHandle) -> String {
    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("settings.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(mode) = val.get("proxyMode").and_then(|v| v.as_str()) {
                        return mode.to_string();
                    }
                }
            }
        }
    }
    "system".to_string()
}

/// Writes the proxyMode value to settings.json (merges with existing settings)
fn set_proxy_mode(app: &AppHandle, mode: &str) {
    if let Ok(mut path) = app.path().app_data_dir() {
        path.push("settings.json");
        let mut settings = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                .unwrap_or_else(|| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };
        settings["proxyMode"] = serde_json::json!(mode);
        let _ = std::fs::write(
            &path,
            serde_json::to_string_pretty(&settings).unwrap_or_default(),
        );
    }
}

/// Formats bytes-per-second into a human-readable speed string
fn format_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec >= 1_048_576 {
        format!("{:.1} MB/s", bytes_per_sec as f64 / 1_048_576.0)
    } else if bytes_per_sec >= 1024 {
        format!("{} KB/s", bytes_per_sec / 1024)
    } else {
        format!("{} B/s", bytes_per_sec)
    }
}

/// Updates the tray icon tooltip with live speed data or connection status
pub fn update_tray_tooltip(app: &AppHandle) {
    let state = app.state::<crate::state::ProxyState>();
    let status = state.get_status();

    let tooltip = match status {
        crate::state::ConnectionStatus::Connected => {
            let down = state.download_speed.lock().map(|v| *v).unwrap_or(0);
            let up = state.upload_speed.lock().map(|v| *v).unwrap_or(0);
            format!(
                "X-Link — \u{2193} {} \u{2191} {}",
                format_speed(down),
                format_speed(up)
            )
        }
        crate::state::ConnectionStatus::Connecting => "X-Link — Connecting...".to_string(),
        crate::state::ConnectionStatus::Disconnected => "X-Link — Disconnected".to_string(),
    };

    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

fn get_disconnected_icon() -> tauri::image::Image<'static> {
    tauri::image::Image::from_bytes(include_bytes!("../icons/System_tray_ico/disconnected.png"))
        .expect("Failed to load disconnected icon")
}

fn get_connecting_icon() -> tauri::image::Image<'static> {
    tauri::image::Image::from_bytes(include_bytes!("../icons/System_tray_ico/connecting.png"))
        .expect("Failed to load connecting icon")
}

fn get_connected_icon() -> tauri::image::Image<'static> {
    tauri::image::Image::from_bytes(include_bytes!("../icons/System_tray_ico/connected.png"))
        .expect("Failed to load connected icon")
}

/// Asynchronously handles the Connect/Disconnect toggle from the tray
async fn handle_toggle_proxy(app: AppHandle) -> Result<(), String> {
    let state = app.state::<crate::state::ProxyState>();
    let status = state.get_status();

    if status != crate::state::ConnectionStatus::Disconnected {
        crate::commands::proxy::toggle_proxy(app.clone(), state.clone(), false, None).await?;
    } else {
        let selected_tag = crate::commands::config::get_selected_outbound_tag(&app);
        crate::commands::proxy::toggle_proxy(app.clone(), state.clone(), true, selected_tag)
            .await?;
    }

    // Update tray state
    update_tray(&app);
    Ok(())
}

/// Rebuilds the context-aware tray menu in a clean, state-driven manner
pub fn build_tray_menu(app: &AppHandle) -> Result<Menu<Wry>, String> {
    let state = app.state::<crate::state::ProxyState>();
    let status = state.get_status();

    let profile_name = match status {
        crate::state::ConnectionStatus::Connected => get_active_profile_name(app),
        crate::state::ConnectionStatus::Connecting => "Connecting...".to_string(),
        crate::state::ConnectionStatus::Disconnected => "Disconnected".to_string(),
    };

    let info_text = format!("X-Link — {}", profile_name);
    let selected_node = get_selected_node_tag(app).unwrap_or_else(|| "None".to_string());
    let server_text = format!("Active Server: {}", selected_node);

    let toggle_text = match status {
        crate::state::ConnectionStatus::Connected | crate::state::ConnectionStatus::Connecting => {
            "Disconnect"
        }
        crate::state::ConnectionStatus::Disconnected => "Connect",
    };

    let current_mode = get_current_proxy_mode(app);

    let menu = Menu::new(app).map_err(|e| e.to_string())?;

    let info_item = MenuItemBuilder::new(info_text)
        .enabled(false)
        .build(app)
        .map_err(|e| e.to_string())?;
    let server_item = MenuItemBuilder::new(server_text)
        .enabled(false)
        .build(app)
        .map_err(|e| e.to_string())?;
    let sep1 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let toggle_item = MenuItemBuilder::with_id("toggle", toggle_text)
        .build(app)
        .map_err(|e| e.to_string())?;
    let restart_proxy_item = MenuItemBuilder::with_id("restart_proxy", "Restart Proxy")
        .build(app)
        .map_err(|e| e.to_string())?;

    // Build Proxy Mode submenu
    let mode_system = CheckMenuItemBuilder::with_id("mode:system", "System Proxy")
        .checked(current_mode == "system")
        .build(app)
        .map_err(|e| e.to_string())?;
    let mode_tun = CheckMenuItemBuilder::with_id("mode:tun", "TUN Mode")
        .checked(current_mode == "tun")
        .build(app)
        .map_err(|e| e.to_string())?;
    let mode_submenu = SubmenuBuilder::with_id(app, "proxy_mode", "Proxy Mode")
        .item(&mode_system)
        .item(&mode_tun)
        .build()
        .map_err(|e| e.to_string())?;

    menu.append(&info_item).map_err(|e| e.to_string())?;
    menu.append(&server_item).map_err(|e| e.to_string())?;
    menu.append(&sep1).map_err(|e| e.to_string())?;
    menu.append(&mode_submenu).map_err(|e| e.to_string())?;

    // Add server nodes submenu when connected
    if status == crate::state::ConnectionStatus::Connected {
        let nodes = get_node_tags_from_active_config(app);
        if !nodes.is_empty() {
            let selected = get_selected_node_tag(app);
            let mut submenu_builder = SubmenuBuilder::with_id(app, "servers", "Switch Server");
            for (tag, _otype) in &nodes {
                let is_selected = selected.as_deref() == Some(tag.as_str());
                let item_id = format!("node:{}", tag);
                let item = CheckMenuItemBuilder::with_id(item_id, tag)
                    .checked(is_selected)
                    .build(app)
                    .map_err(|e| e.to_string())?;
                submenu_builder = submenu_builder.item(&item);
            }
            let submenu = submenu_builder.build().map_err(|e| e.to_string())?;
            menu.append(&submenu).map_err(|e| e.to_string())?;

            let sep_nodes = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
            menu.append(&sep_nodes).map_err(|e| e.to_string())?;
        }
    }

    menu.append(&toggle_item).map_err(|e| e.to_string())?;
    menu.append(&restart_proxy_item)
        .map_err(|e| e.to_string())?;

    let open_item = MenuItemBuilder::with_id("open", "Open Dashboard")
        .build(app)
        .map_err(|e| e.to_string())?;
    let restart_app_item = MenuItemBuilder::with_id("restart_app", "Restart X-Link")
        .build(app)
        .map_err(|e| e.to_string())?;
    let sep2 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit")
        .build(app)
        .map_err(|e| e.to_string())?;

    menu.append(&open_item).map_err(|e| e.to_string())?;
    menu.append(&restart_app_item).map_err(|e| e.to_string())?;
    menu.append(&sep2).map_err(|e| e.to_string())?;
    menu.append(&quit_item).map_err(|e| e.to_string())?;

    Ok(menu)
}

/// Programmatically builds the tray icon and attaches menu event callbacks
pub fn create_tray(app: &AppHandle) -> Result<TrayIcon, String> {
    let menu = build_tray_menu(app)?;

    let tray = TrayIconBuilder::with_id("main")
        .icon(get_disconnected_icon())
        .tooltip("X-Link — Disconnected")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let id = event.id.as_ref().to_string();
            // Handle node switch from tray
            if id.starts_with("node:") {
                let node_tag = id.strip_prefix("node:").unwrap_or("").to_string();
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<crate::state::ProxyState>();
                    let _ = crate::commands::proxy::switch_node_hot(
                        app_handle.clone(),
                        state.clone(),
                        node_tag,
                    )
                    .await;
                    update_tray(&app_handle);
                    let _ = app_handle.emit("node-switched", ());
                });
                return;
            }
            // Handle proxy mode switch from tray
            if id.starts_with("mode:") {
                let new_mode = id.strip_prefix("mode:").unwrap_or("system").to_string();
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    set_proxy_mode(&app_handle, &new_mode);
                    let state = app_handle.state::<crate::state::ProxyState>();
                    let status = state.get_status();

                    // If currently connected, restart proxy with the new mode
                    if status != crate::state::ConnectionStatus::Disconnected {
                        let selected_tag =
                            crate::commands::config::get_selected_outbound_tag(&app_handle);
                        let _ = crate::commands::proxy::toggle_proxy(
                            app_handle.clone(),
                            state.clone(),
                            false,
                            None,
                        )
                        .await;
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        let _ = crate::commands::proxy::toggle_proxy(
                            app_handle.clone(),
                            state.clone(),
                            true,
                            selected_tag,
                        )
                        .await;
                    }

                    update_tray(&app_handle);
                    // Notify frontend so Dashboard updates mode display
                    let _ = app_handle.emit("settings-changed", ());
                });
                return;
            }
            match id.as_str() {
                "toggle" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = handle_toggle_proxy(app_handle).await;
                    });
                }
                "restart_proxy" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<crate::state::ProxyState>();
                        let selected_tag =
                            crate::commands::config::get_selected_outbound_tag(&app_handle);

                        // Stop the proxy
                        let _ = crate::commands::proxy::toggle_proxy(
                            app_handle.clone(),
                            state.clone(),
                            false,
                            None,
                        )
                        .await;
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                        // Start the proxy again
                        let _ = crate::commands::proxy::toggle_proxy(
                            app_handle.clone(),
                            state.clone(),
                            true,
                            selected_tag,
                        )
                        .await;
                        update_tray(&app_handle);
                    });
                }
                "open" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "restart_app" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        perform_clean_cleanup(&app_handle);
                        app_handle.restart();
                    });
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

/// Triggers a dynamic redraw of the system tray icon, menu, and tooltip from any state transition
pub fn update_tray(app: &AppHandle) {
    let state = app.state::<crate::state::ProxyState>();
    let status = state.get_status();

    if let Some(tray) = app.tray_by_id("main") {
        let icon = match status {
            crate::state::ConnectionStatus::Disconnected => get_disconnected_icon(),
            crate::state::ConnectionStatus::Connecting => get_connecting_icon(),
            crate::state::ConnectionStatus::Connected => get_connected_icon(),
        };
        let _ = tray.set_icon(Some(icon));

        if let Ok(menu) = build_tray_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }

    // Also refresh the tooltip
    update_tray_tooltip(app);
}

/// Alias for backward compatibility
pub fn update_tray_menu(app: &AppHandle) {
    update_tray(app);
}
