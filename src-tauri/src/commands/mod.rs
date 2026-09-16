use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::monitor::{
    create_monitor, DailyAggregate, HourlyAggregate, MonitorSnapshot, NetworkMonitor,
};
use crate::settings::AppSettings;
use crate::storage::Database;

pub struct AppState {
    pub db: Arc<Database>,
    pub last_snapshot: Mutex<Option<MonitorSnapshot>>,
    pub prev_app_bytes: Mutex<std::collections::HashMap<u32, (u64, u64)>>,
    pub prev_iface_bytes: Mutex<std::collections::HashMap<u32, (u64, u64)>>,
    pub monitoring_enabled: Mutex<bool>,
    pub settings: Mutex<AppSettings>,
    pub settings_dir: std::path::PathBuf,
    pub db_path: String,
    pub platform: String,
    pub alert_fired_day: Mutex<Option<i64>>,
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
    pub poll_interval_secs: u64,
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
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let snap = state.last_snapshot.lock().map_err(|e| e.to_string())?;
    Ok(MonitorStatus {
        monitoring_enabled: enabled,
        platform: state.platform.clone(),
        db_path: state.db_path.clone(),
        app_count: snap.as_ref().map(|s| s.apps.len()).unwrap_or(0),
        interface_count: snap.as_ref().map(|s| s.interfaces.len()).unwrap_or(0),
        current_ssid: snap.as_ref().and_then(|s| s.current_ssid.clone()),
        last_poll_ms: snap.as_ref().map(|s| s.timestamp_ms),
        poll_interval_secs: settings.poll_interval_secs,
    })
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Result<AppSettings, String> {
    Ok(state.settings.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let cleaned = settings.sanitize();
    cleaned.save(&state.settings_dir)?;
    {
        let mut guard = state.settings.lock().map_err(|e| e.to_string())?;
        *guard = cleaned.clone();
    }

    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let launcher = app.autolaunch();
        if cleaned.start_with_os {
            let _ = launcher.enable();
        } else {
            let _ = launcher.disable();
        }
    }

    let _ = app.emit("settings://updated", cleaned.clone());
    Ok(cleaned)
}

#[tauri::command]
pub fn run_retention_now(state: State<'_, Arc<AppState>>) -> Result<u64, String> {
    let days = state
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .retention_days;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let cutoff = now - (i64::from(days) * 86_400_000);
    state.db.purge_raw_older_than(cutoff)
}

pub fn start_monitor_loop(app: AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let mut monitor: Box<dyn NetworkMonitor> = create_monitor();
        let mut last_retention = std::time::Instant::now()
            .checked_sub(Duration::from_secs(3600))
            .unwrap_or_else(std::time::Instant::now);

        loop {
            let (enabled, interval, settings_snapshot) = {
                let enabled = state
                    .monitoring_enabled
                    .lock()
                    .map(|g| *g)
                    .unwrap_or(true);
                let settings = state
                    .settings
                    .lock()
                    .map(|g| g.clone())
                    .unwrap_or_default();
                (enabled, settings.poll_interval_secs.max(1), settings)
            };

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

                        maybe_fire_alert(&app, &state, &settings_snapshot);
                    }
                    Err(err) => {
                        let _ = app.emit("traffic://error", err);
                    }
                }
            }

            if last_retention.elapsed() > Duration::from_secs(6 * 3600) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                let cutoff = now - (i64::from(settings_snapshot.retention_days) * 86_400_000);
                let _ = state.db.purge_raw_older_than(cutoff);
                last_retention = std::time::Instant::now();
            }

            std::thread::sleep(Duration::from_secs(interval));
        }
    });
}

fn maybe_fire_alert(app: &AppHandle, state: &AppState, settings: &AppSettings) {
    if !settings.alert_enabled {
        return;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let day_start = now - (now % 86_400_000);
    {
        let fired = state.alert_fired_day.lock().ok();
        if let Some(guard) = fired {
            if guard.as_ref() == Some(&day_start) {
                return;
            }
        }
    }

    let Ok(total) = state
        .db
        .today_interface_bytes(settings.alert_ssid_only.as_deref())
    else {
        return;
    };
    if total < settings.alert_daily_bytes {
        return;
    }

    if let Ok(mut guard) = state.alert_fired_day.lock() {
        *guard = Some(day_start);
    }

    let gb = total as f64 / (1024.0 * 1024.0 * 1024.0);
    let msg = format!("Daily usage reached {gb:.2} GB");
    let _ = app.emit("alerts://usage", msg.clone());

    #[cfg(desktop)]
    {
        use tauri_plugin_notification::NotificationExt;
        let _ = app
            .notification()
            .builder()
            .title("NetPulse")
            .body(&msg)
            .show();
    }
}

pub fn init_state(app: &AppHandle) -> Result<Arc<AppState>, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = dir.join("netpulse.db");
    let db_path_str = db_path.to_string_lossy().to_string();
    let settings = AppSettings::load(&dir).sanitize();
    let db = Arc::new(Database::open(&db_path)?);
    Ok(Arc::new(AppState {
        db,
        last_snapshot: Mutex::new(None),
        prev_app_bytes: Mutex::new(Default::default()),
        prev_iface_bytes: Mutex::new(Default::default()),
        monitoring_enabled: Mutex::new(true),
        settings: Mutex::new(settings),
        settings_dir: dir,
        db_path: db_path_str,
        platform: std::env::consts::OS.to_string(),
        alert_fired_day: Mutex::new(None),
    }))
}
