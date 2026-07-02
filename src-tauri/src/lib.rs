mod commands;
pub mod config;
pub mod os;
mod state;
pub mod tray;

use state::ProxyState;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

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
        .manage(ProxyState::new())
        .invoke_handler(tauri::generate_handler![
            commands::system::get_singbox_version,
            commands::system::get_app_version,
            commands::system::show_window,
            commands::system::check_port_conflict,
            commands::system::set_autostart,
            commands::system::check_tun_support,
            commands::system::request_elevation,
            commands::system::set_runas_admin,
            commands::logs::get_buffered_logs,
            commands::proxy::get_proxy_status,
            commands::proxy::get_traffic_stats,
            commands::proxy::get_active_connections,
            commands::proxy::close_connection,
            commands::proxy::close_all_connections,
            commands::proxy::toggle_proxy,
            commands::proxy::reload_active_config,
            commands::proxy::switch_node_hot,
            commands::config::import_config,
            commands::config::get_active_outbound,
            commands::config::get_config_outbounds,
            commands::config::update_node,
            commands::config::delete_profile_config,
            commands::config::switch_profile,
            commands::config::get_profile_outbounds,
            commands::latency::test_node_latency,
            commands::latency::test_all_nodes
        ])
        .setup(|app| {
            // Load and cache settings dynamically on boot
            // Run in a background thread to avoid blocking the app startup
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let state = app_handle.state::<ProxyState>();
                if let Ok(settings) = state.reload_settings(&app_handle) {
                    // Persistence: if running elevated and proxyMode is 'tun', ensure runas registry flag is set
                    if crate::os::is_elevated() && settings.proxy_mode == "tun" {
                        let _ = crate::commands::system::set_runas_admin(true);
                    }
                }
            });

            // Copy wintun.dll next to sing-box sidecar dynamically on boot to ensure TUN mode works
            // Run in a background thread to avoid blocking the setup hook with filesystem scans
            let handle_for_wintun = app.handle().clone();
            std::thread::spawn(move || {
                copy_wintun_dll_to_sidecar_dir(&handle_for_wintun);
            });

            // Pre-fetch and cache the sing-box version in the background
            let handle_for_version = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(sidecar) = handle_for_version.shell().sidecar("sing-box") {
                    if let Ok(output) = sidecar.args(["version"]).output().await {
                        if output.status.success() {
                            let version = String::from_utf8_lossy(&output.stdout).into_owned();
                            {
                                let state = handle_for_version.state::<ProxyState>();
                                if let Ok(mut guard) = state.singbox_version.lock() {
                                    *guard = Some(version);
                                };
                            }
                        }
                    }
                }
            });

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

pub(crate) fn copy_wintun_dll_to_sidecar_dir(app: &tauri::AppHandle) {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
    }
    #[cfg(target_os = "windows")]
    {
        use tauri::path::BaseDirectory;

        if let Ok(resource_dir) = app.path().resolve(".", BaseDirectory::Resource) {
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
