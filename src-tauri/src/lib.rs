mod monitor;
mod storage;
mod commands;
mod settings;

use std::sync::Arc;

use tauri::{Manager, WindowEvent};

use commands::{
    export_usage, get_daily_history, get_hourly_history, get_latest_snapshot, get_monitor_status,
    get_settings, init_state, list_interfaces, list_ssids, run_retention_now, save_settings,
    set_monitoring_enabled, start_monitor_loop,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            let state = init_state(app.handle())?;

            #[cfg(desktop)]
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
                use tauri_plugin_autostart::ManagerExt;

                let start = state
                    .settings
                    .lock()
                    .map(|s| s.start_with_os)
                    .unwrap_or(false);
                let launcher = app.autolaunch();
                if start {
                    let _ = launcher.enable();
                }

                let show_i = MenuItem::with_id(app, "show", "Show NetPulse", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

                let _tray = TrayIconBuilder::with_id("netpulse-tray")
                    .icon(app.default_window_icon().unwrap().clone())
                    .menu(&menu)
                    .tooltip("NetPulse")
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;

                let minimized = std::env::args().any(|a| a == "--minimized");
                if minimized {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
            }

            start_monitor_loop(app.handle().clone(), Arc::clone(&state));
            app.manage(state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let close_to_tray = window
                    .app_handle()
                    .try_state::<Arc<commands::AppState>>()
                    .and_then(|s| s.settings.lock().ok().map(|g| g.close_to_tray))
                    .unwrap_or(true);
                if close_to_tray {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_latest_snapshot,
            get_hourly_history,
            get_daily_history,
            export_usage,
            list_ssids,
            list_interfaces,
            set_monitoring_enabled,
            get_monitor_status,
            get_settings,
            save_settings,
            run_retention_now
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub use commands::AppState;
