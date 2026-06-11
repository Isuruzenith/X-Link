mod state;
mod commands;
pub mod config;
pub mod os;
pub mod tray;

use state::ProxyState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(ProxyState::new(7890, 7891, 7892))
        .invoke_handler(tauri::generate_handler![
            commands::system::get_singbox_version,
            commands::system::get_app_version,
            commands::system::check_port_conflict,
            commands::system::set_autostart,
            commands::system::check_tun_support,
            commands::system::request_elevation,
            commands::system::set_runas_admin,
            commands::logs::get_buffered_logs,
            commands::proxy::get_proxy_status,
            commands::proxy::get_traffic_stats,
            commands::proxy::toggle_proxy,
            commands::profiles::import_subscription,
            commands::profiles::import_file,
            commands::profiles::import_from_clipboard,
            commands::profiles::delete_profile,
            commands::profiles::get_profile_outbound,
            commands::profiles::get_profile_outbounds,
            commands::profiles::update_profile_config
        ])
        .setup(|app| {
            // Load saved ports from settings.json dynamically on boot
            let state = app.state::<ProxyState>();
            let mut proxy_mode = "tun".to_string();
            if let Ok(mut path) = app.path().app_data_dir() {
                path.push("settings.json");
                if path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(p) = val.get("mixedPort").and_then(|v| v.as_u64()) {
                                *state.mixed_port.lock().unwrap() = p as u16;
                            }
                            if let Some(p) = val.get("httpPort").and_then(|v| v.as_u64()) {
                                *state.http_port.lock().unwrap() = p as u16;
                            }
                            if let Some(p) = val.get("socksPort").and_then(|v| v.as_u64()) {
                                *state.socks_port.lock().unwrap() = p as u16;
                            }
                            if let Some(m) = val.get("proxyMode").and_then(|v| v.as_str()) {
                                proxy_mode = m.to_string();
                            }
                        }
                    }
                }
            }

            // Persistence: if running elevated and proxyMode is 'tun', ensure runas registry flag is set
            if is_elevated::is_elevated() && proxy_mode == "tun" {
                let _ = crate::commands::system::set_runas_admin(true);
            }

            // Copy wintun.dll next to sing-box sidecar dynamically on boot to ensure TUN mode works
            copy_wintun_dll_to_sidecar_dir(app.handle());

            // Programmatically initialize the system tray helper
            let _ = crate::tray::create_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Intercept close events to hide in the system tray if closeToTray setting is active
                let close_to_tray = crate::tray::get_close_to_tray_setting(window.app_handle());
                if close_to_tray {
                    let _ = window.hide();
                    api.prevent_close();
                } else {
                    api.prevent_close();
                    crate::tray::perform_clean_cleanup(window.app_handle());
                    window.app_handle().exit(0);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Intercept exit event globally to ensure sidecar process termination and proxy configuration cleanup
    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            crate::tray::perform_clean_cleanup(app_handle);
        }
    });
}

fn copy_wintun_dll_to_sidecar_dir(app: &tauri::AppHandle) {
    #[cfg(target_os = "windows")]
    {
        if let Ok(resource_dir) = app.path().resource_dir() {
            let wintun_src = resource_dir.join("binaries").join("wintun.dll");
            if wintun_src.exists() {
                let mut paths_to_try = vec![];
                
                // Try next to main exe (target/debug or production directory)
                if let Ok(exe_path) = std::env::current_exe() {
                    if let Some(exe_dir) = exe_path.parent() {
                        paths_to_try.push(exe_dir.to_path_buf());
                    }
                }
                
                // Try resources directory itself
                paths_to_try.push(resource_dir.clone());
                
                // Try binaries directory inside resources
                paths_to_try.push(resource_dir.join("binaries"));
                
                // Search for any subdirectory in resources (which could be the target sidecar folder)
                if let Ok(entries) = std::fs::read_dir(&resource_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            paths_to_try.push(path);
                        }
                    }
                }

                // Copy wintun.dll next to any found sing-box*.exe
                for dir in paths_to_try {
                    if dir.exists() {
                        let contains_sing_box = if let Ok(entries) = std::fs::read_dir(&dir) {
                            entries.flatten().any(|e| {
                                let name = e.file_name().to_string_lossy().to_lowercase();
                                name.contains("sing-box") && name.ends_with(".exe")
                            })
                        } else {
                            false
                        };
                        
                        if contains_sing_box {
                            let dest = dir.join("wintun.dll");
                            if !dest.exists() {
                                let _ = std::fs::copy(&wintun_src, &dest);
                            }
                        }
                    }
                }
            }
        }
    }
}
