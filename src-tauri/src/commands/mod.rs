use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::monitor::{
    create_monitor, DailyAggregate, HourlyAggregate, MonitorSnapshot, NetworkMonitor,
};
use crate::storage::Database;

pub struct AppState {
    pub db: Arc<Database>,
    pub last_snapshot: Mutex<Option<MonitorSnapshot>>,
    pub prev_app_bytes: Mutex<std::collections::HashMap<u32, (u64, u64)>>,
    pub prev_iface_bytes: Mutex<std::collections::HashMap<u32, (u64, u64)>>,
    pub monitoring_enabled: Mutex<bool>,
    pub db_path: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorStatus {
    pub monitoring_enabled: bool,
    pub platform: String,
    pub db_path: String,
    pub app_count: usize,
    pub interface_count: usize,
    pub current_ssid: Option<String>,
    pub last_poll_ms: Option<u64>,
}

#[tauri::command]
pub fn get_latest_snapshot(state: State<'_, Arc<AppState>>) -> Option<MonitorSnapshot> {
    state.last_snapshot.lock().ok().and_then(|g| g.clone())
}

#[tauri::command]
pub fn get_hourly_history(
    state: State<'_, Arc<AppState>>,
    from_ms: i64,
    to_ms: i64,
    ssid: Option<String>,
    interface_name: Option<String>,
    scope: Option<String>,
) -> Result<Vec<HourlyAggregate>, String> {
    let scope = scope.unwrap_or_else(|| "interfaces".into());
    state.db.hourly_aggregates(
        from_ms,
        to_ms,
        ssid.as_deref(),
        interface_name.as_deref(),
        &scope,
    )
}

#[tauri::command]
pub fn get_daily_history(
    state: State<'_, Arc<AppState>>,
    from_ms: i64,
    to_ms: i64,
    ssid: Option<String>,
    interface_name: Option<String>,
    scope: Option<String>,
) -> Result<Vec<DailyAggregate>, String> {
    let scope = scope.unwrap_or_else(|| "interfaces".into());
    state.db.daily_aggregates(
        from_ms,
        to_ms,
        ssid.as_deref(),
        interface_name.as_deref(),
        &scope,
    )
}

#[tauri::command]
pub fn export_usage(
    state: State<'_, Arc<AppState>>,
    format: String,
    from_ms: i64,
    to_ms: i64,
) -> Result<String, String> {
    match format.as_str() {
        "json" => state.db.export_json(from_ms, to_ms),
        "csv" => state.db.export_csv(from_ms, to_ms),
        _ => Err("format must be json or csv".into()),
    }
}

#[tauri::command]
pub fn list_ssids(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    state.db.distinct_ssids()
}

#[tauri::command]
pub fn list_interfaces(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    state.db.distinct_interfaces()
}

#[tauri::command]
pub fn set_monitoring_enabled(
    state: State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<bool, String> {
    let mut guard = state.monitoring_enabled.lock().map_err(|e| e.to_string())?;
    *guard = enabled;
    Ok(enabled)
}

#[tauri::command]
pub fn get_monitor_status(state: State<'_, Arc<AppState>>) -> Result<MonitorStatus, String> {
    let enabled = *state
        .monitoring_enabled
        .lock()
        .map_err(|e| e.to_string())?;
    let snap = state.last_snapshot.lock().map_err(|e| e.to_string())?;
    Ok(MonitorStatus {
        monitoring_enabled: enabled,
        platform: state.platform.clone(),
        db_path: state.db_path.clone(),
        app_count: snap.as_ref().map(|s| s.apps.len()).unwrap_or(0),
        interface_count: snap.as_ref().map(|s| s.interfaces.len()).unwrap_or(0),
        current_ssid: snap.as_ref().and_then(|s| s.current_ssid.clone()),
        last_poll_ms: snap.as_ref().map(|s| s.timestamp_ms),
    })
}

pub fn start_monitor_loop(app: AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let mut monitor: Box<dyn NetworkMonitor> = create_monitor();
        loop {
            let enabled = state
                .monitoring_enabled
                .lock()
                .map(|g| *g)
                .unwrap_or(true);

            if enabled {
                match monitor.poll_snapshot() {
                    Ok(snapshot) => {
                        let app_deltas = {
                            let mut prev = state.prev_app_bytes.lock().unwrap();
                            let mut deltas = Vec::new();
                            for app_row in &snapshot.apps {
                                let last = prev.get(&app_row.pid).copied().unwrap_or((0, 0));
                                let sent_delta = app_row.bytes_sent.saturating_sub(last.0);
                                let recv_delta = app_row.bytes_received.saturating_sub(last.1);
                                prev.insert(app_row.pid, (app_row.bytes_sent, app_row.bytes_received));
                                if sent_delta > 0 || recv_delta > 0 {
                                    deltas.push((app_row.clone(), sent_delta, recv_delta));
                                }
                            }
                            deltas
                        };

                        let iface_deltas = {
                            let mut prev = state.prev_iface_bytes.lock().unwrap();
                            let mut deltas = Vec::new();
                            for iface in &snapshot.interfaces {
                                let last = prev.get(&iface.index).copied().unwrap_or((0, 0));
                                let sent_delta = iface.bytes_sent.saturating_sub(last.0);
                                let recv_delta = iface.bytes_received.saturating_sub(last.1);
                                let had = prev.contains_key(&iface.index);
                                prev.insert(iface.index, (iface.bytes_sent, iface.bytes_received));
                                if had && (sent_delta > 0 || recv_delta > 0) {
                                    deltas.push((iface.clone(), sent_delta, recv_delta));
                                }
                            }
                            deltas
                        };

                        let _ = state
                            .db
                            .record_snapshot(&snapshot, &app_deltas, &iface_deltas);

                        if let Ok(mut guard) = state.last_snapshot.lock() {
                            *guard = Some(snapshot.clone());
                        }
                        let _ = app.emit("traffic://update", snapshot);
                    }
                    Err(err) => {
                        let _ = app.emit("traffic://error", err);
                    }
                }
            }

            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

pub fn init_state(app: &AppHandle) -> Result<Arc<AppState>, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = dir.join("netpulse.db");
    let db_path_str = db_path.to_string_lossy().to_string();
    let db = Arc::new(Database::open(&db_path)?);
    Ok(Arc::new(AppState {
        db,
        last_snapshot: Mutex::new(None),
        prev_app_bytes: Mutex::new(Default::default()),
        prev_iface_bytes: Mutex::new(Default::default()),
        monitoring_enabled: Mutex::new(true),
        db_path: db_path_str,
        platform: std::env::consts::OS.to_string(),
    }))
}
