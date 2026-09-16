use std::sync::{Arc, Mutex};
use std::time::Duration;

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
) -> Result<Vec<HourlyAggregate>, String> {
    state.db.hourly_aggregates(
        from_ms,
        to_ms,
        ssid.as_deref(),
        interface_name.as_deref(),
    )
}

#[tauri::command]
pub fn get_daily_history(
    state: State<'_, Arc<AppState>>,
    from_ms: i64,
    to_ms: i64,
    ssid: Option<String>,
    interface_name: Option<String>,
) -> Result<Vec<DailyAggregate>, String> {
    state.db.daily_aggregates(
        from_ms,
        to_ms,
        ssid.as_deref(),
        interface_name.as_deref(),
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

pub fn start_monitor_loop(app: AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let mut monitor: Box<dyn NetworkMonitor> = create_monitor();
        loop {
            match monitor.poll_snapshot() {
                Ok(snapshot) => {
                    let app_deltas = {
                        let mut prev = state.prev_app_bytes.lock().unwrap();
                        let mut deltas = Vec::new();
                        for app in &snapshot.apps {
                            let last = prev.get(&app.pid).copied().unwrap_or((0, 0));
                            let sent_delta = app.bytes_sent.saturating_sub(last.0);
                            let recv_delta = app.bytes_received.saturating_sub(last.1);
                            prev.insert(app.pid, (app.bytes_sent, app.bytes_received));
                            if sent_delta > 0 || recv_delta > 0 {
                                deltas.push((app.clone(), sent_delta, recv_delta));
                            }
                        }
                        deltas
                    };

                    let iface_deltas = {
                        let mut prev = state.prev_iface_bytes.lock().unwrap();
                        let mut deltas = Vec::new();
                        for iface in &snapshot.interfaces {
                            let last = prev.get(&iface.index).copied().unwrap_or((0, 0));
                            // Interface counters are OS lifetime totals; use rate * interval approx via absolute diff
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
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

pub fn init_state(app: &AppHandle) -> Result<Arc<AppState>, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let db_path = dir.join("netpulse.db");
    let db = Arc::new(Database::open(&db_path)?);
    Ok(Arc::new(AppState {
        db,
        last_snapshot: Mutex::new(None),
        prev_app_bytes: Mutex::new(Default::default()),
        prev_iface_bytes: Mutex::new(Default::default()),
    }))
}
