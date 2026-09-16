mod monitor;
mod storage;
mod commands;

use std::sync::Arc;

use tauri::Manager;

use commands::{
    export_usage, get_daily_history, get_hourly_history, get_latest_snapshot, init_state,
    list_interfaces, list_ssids, start_monitor_loop,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = init_state(app.handle())?;
            start_monitor_loop(app.handle().clone(), Arc::clone(&state));
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_latest_snapshot,
            get_hourly_history,
            get_daily_history,
            export_usage,
            list_ssids,
            list_interfaces
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
