use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::monitor::{
    AppTraffic, DailyAggregate, HourlyAggregate, InterfaceTraffic, InterfaceType, MonitorSnapshot,
    Protocol,
};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| e.to_string())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY
            );

            CREATE TABLE IF NOT EXISTS application (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                executable_path TEXT,
                platform TEXT NOT NULL,
                UNIQUE(name, executable_path, platform)
            );

            CREATE TABLE IF NOT EXISTS network_interface (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                ssid_or_label TEXT,
                UNIQUE(name, type, ssid_or_label)
            );

            CREATE TABLE IF NOT EXISTS traffic_record (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                interface_id INTEGER REFERENCES network_interface(id),
                application_id INTEGER REFERENCES application(id),
                bytes_sent INTEGER NOT NULL DEFAULT 0,
                bytes_received INTEGER NOT NULL DEFAULT 0,
                network_ssid TEXT,
                protocol TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_traffic_ts ON traffic_record(timestamp);
            CREATE INDEX IF NOT EXISTS idx_traffic_ssid ON traffic_record(network_ssid);

            CREATE TABLE IF NOT EXISTS aggregate_hourly (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hour_start INTEGER NOT NULL,
                interface_id INTEGER NOT NULL DEFAULT 0,
                application_id INTEGER NOT NULL DEFAULT 0,
                network_ssid TEXT NOT NULL DEFAULT '',
                bytes_sent INTEGER NOT NULL DEFAULT 0,
                bytes_received INTEGER NOT NULL DEFAULT 0,
                UNIQUE(hour_start, interface_id, application_id, network_ssid)
            );

            CREATE TABLE IF NOT EXISTS aggregate_daily (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                day_start INTEGER NOT NULL,
                interface_id INTEGER NOT NULL DEFAULT 0,
                application_id INTEGER NOT NULL DEFAULT 0,
                network_ssid TEXT NOT NULL DEFAULT '',
                bytes_sent INTEGER NOT NULL DEFAULT 0,
                bytes_received INTEGER NOT NULL DEFAULT 0,
                UNIQUE(day_start, interface_id, application_id, network_ssid)
            );
            "#,
        )
        .map_err(|e| e.to_string())?;

        let version: Option<i64> = conn
            .query_row(
                "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        if version.is_none() {
            conn.execute("INSERT INTO schema_migrations(version) VALUES (1)", [])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn record_snapshot(
        &self,
        snapshot: &MonitorSnapshot,
        app_deltas: &[(AppTraffic, u64, u64)],
        iface_deltas: &[(InterfaceTraffic, u64, u64)],
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let ts = snapshot.timestamp_ms as i64;
        let platform = std::env::consts::OS;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

        for (app, sent_delta, recv_delta) in app_deltas {
            if *sent_delta == 0 && *recv_delta == 0 {
                continue;
            }
            let app_id = upsert_application(
                &tx,
                &app.name,
                app.executable_path.as_deref(),
                platform,
            )?;
            tx.execute(
                "INSERT INTO traffic_record (timestamp, interface_id, application_id, bytes_sent, bytes_received, network_ssid, protocol)
                 VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6)",
                params![
                    ts,
                    app_id,
                    *sent_delta as i64,
                    *recv_delta as i64,
                    snapshot.current_ssid,
                    protocol_str(app.protocol)
                ],
            )
            .map_err(|e| e.to_string())?;

            upsert_hourly(
                &tx,
                ts,
                0,
                app_id,
                snapshot.current_ssid.as_deref(),
                *sent_delta,
                *recv_delta,
            )?;
            upsert_daily(
                &tx,
                ts,
                0,
                app_id,
                snapshot.current_ssid.as_deref(),
                *sent_delta,
                *recv_delta,
            )?;
        }

        for (iface, sent_delta, recv_delta) in iface_deltas {
            if *sent_delta == 0 && *recv_delta == 0 {
                continue;
            }
            let iface_id = upsert_interface(
                &tx,
                &iface.name,
                iface.interface_type,
                iface.ssid.as_deref(),
            )?;
            tx.execute(
                "INSERT INTO traffic_record (timestamp, interface_id, application_id, bytes_sent, bytes_received, network_ssid, protocol)
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5, NULL)",
                params![
                    ts,
                    iface_id,
                    *sent_delta as i64,
                    *recv_delta as i64,
                    iface.ssid
                ],
            )
            .map_err(|e| e.to_string())?;

            upsert_hourly(
                &tx,
                ts,
                iface_id,
                0,
                iface.ssid.as_deref(),
                *sent_delta,
                *recv_delta,
            )?;
            upsert_daily(
                &tx,
                ts,
                iface_id,
                0,
                iface.ssid.as_deref(),
                *sent_delta,
                *recv_delta,
            )?;
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn hourly_aggregates(
        &self,
        from_ms: i64,
        to_ms: i64,
        ssid: Option<&str>,
        interface_name: Option<&str>,
        scope: &str,
    ) -> Result<Vec<HourlyAggregate>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut sql = String::from(
            "SELECT h.hour_start, ni.name, a.name, h.network_ssid, h.bytes_sent, h.bytes_received
             FROM aggregate_hourly h
             LEFT JOIN network_interface ni ON ni.id = h.interface_id AND h.interface_id != 0
             LEFT JOIN application a ON a.id = h.application_id AND h.application_id != 0
             WHERE h.hour_start >= ?1 AND h.hour_start < ?2",
        );
        // Avoid double-counting: interface rows and app rows both store the same traffic.
        match scope {
            "apps" => sql.push_str(" AND h.application_id != 0 AND h.interface_id = 0"),
            _ => sql.push_str(" AND h.interface_id != 0 AND h.application_id = 0"),
        }
        if ssid.is_some() {
            sql.push_str(" AND h.network_ssid = ?3");
        }
        if interface_name.is_some() {
            sql.push_str(if ssid.is_some() {
                " AND ni.name = ?4"
            } else {
                " AND ni.name = ?3"
            });
        }
        sql.push_str(" ORDER BY h.hour_start ASC");

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = match (ssid, interface_name) {
            (Some(s), Some(i)) => stmt
                .query_map(params![from_ms, to_ms, s, i], map_hourly)
                .map_err(|e| e.to_string())?,
            (Some(s), None) => stmt
                .query_map(params![from_ms, to_ms, s], map_hourly)
                .map_err(|e| e.to_string())?,
            (None, Some(i)) => stmt
                .query_map(params![from_ms, to_ms, i], map_hourly)
                .map_err(|e| e.to_string())?,
            (None, None) => stmt
                .query_map(params![from_ms, to_ms], map_hourly)
                .map_err(|e| e.to_string())?,
        };

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn daily_aggregates(
        &self,
        from_ms: i64,
        to_ms: i64,
        ssid: Option<&str>,
        interface_name: Option<&str>,
        scope: &str,
    ) -> Result<Vec<DailyAggregate>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut sql = String::from(
            "SELECT d.day_start, ni.name, a.name, d.network_ssid, d.bytes_sent, d.bytes_received
             FROM aggregate_daily d
             LEFT JOIN network_interface ni ON ni.id = d.interface_id AND d.interface_id != 0
             LEFT JOIN application a ON a.id = d.application_id AND d.application_id != 0
             WHERE d.day_start >= ?1 AND d.day_start < ?2",
        );
        match scope {
            "apps" => sql.push_str(" AND d.application_id != 0 AND d.interface_id = 0"),
            _ => sql.push_str(" AND d.interface_id != 0 AND d.application_id = 0"),
        }
        if ssid.is_some() {
            sql.push_str(" AND d.network_ssid = ?3");
        }
        if interface_name.is_some() {
            sql.push_str(if ssid.is_some() {
                " AND ni.name = ?4"
            } else {
                " AND ni.name = ?3"
            });
        }
        sql.push_str(" ORDER BY d.day_start ASC");

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = match (ssid, interface_name) {
            (Some(s), Some(i)) => stmt
                .query_map(params![from_ms, to_ms, s, i], map_daily)
                .map_err(|e| e.to_string())?,
            (Some(s), None) => stmt
                .query_map(params![from_ms, to_ms, s], map_daily)
                .map_err(|e| e.to_string())?,
            (None, Some(i)) => stmt
                .query_map(params![from_ms, to_ms, i], map_daily)
                .map_err(|e| e.to_string())?,
            (None, None) => stmt
                .query_map(params![from_ms, to_ms], map_daily)
                .map_err(|e| e.to_string())?,
        };

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn export_json(
        &self,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<String, String> {
        let hourly = self.hourly_aggregates(from_ms, to_ms, None, None, "interfaces")?;
        serde_json::to_string_pretty(&hourly).map_err(|e| e.to_string())
    }

    pub fn export_csv(&self, from_ms: i64, to_ms: i64) -> Result<String, String> {
        let hourly = self.hourly_aggregates(from_ms, to_ms, None, None, "interfaces")?;
        let mut out = String::from(
            "hour_start_ms,interface_name,application_name,network_ssid,bytes_sent,bytes_received\n",
        );
        for row in hourly {
            out.push_str(&format!(
                "{},{},{},{},{},{}\n",
                row.hour_start_ms,
                csv_escape(row.interface_name.as_deref().unwrap_or("")),
                csv_escape(row.application_name.as_deref().unwrap_or("")),
                csv_escape(row.network_ssid.as_deref().unwrap_or("")),
                row.bytes_sent,
                row.bytes_received
            ));
        }
        Ok(out)
    }

    pub fn distinct_ssids(&self) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT network_ssid FROM traffic_record WHERE network_ssid IS NOT NULL ORDER BY network_ssid",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn distinct_interfaces(&self) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT DISTINCT name FROM network_interface ORDER BY name")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// Delete raw traffic records older than cutoff; keep aggregates for charts.
    pub fn purge_raw_older_than(&self, cutoff_ms: i64) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let n = conn
            .execute(
                "DELETE FROM traffic_record WHERE timestamp < ?1",
                params![cutoff_ms],
            )
            .map_err(|e| e.to_string())?;
        Ok(n as u64)
    }

    /// Sum interface-scoped daily usage for today (or optional SSID filter).
    pub fn today_interface_bytes(&self, ssid: Option<&str>) -> Result<u64, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let day_start = now - (now % 86_400_000);
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let total: i64 = if let Some(s) = ssid {
            conn.query_row(
                "SELECT COALESCE(SUM(bytes_sent + bytes_received), 0)
                 FROM aggregate_daily
                 WHERE day_start = ?1 AND interface_id != 0 AND application_id = 0
                   AND network_ssid = ?2",
                params![day_start, s],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?
        } else {
            conn.query_row(
                "SELECT COALESCE(SUM(bytes_sent + bytes_received), 0)
                 FROM aggregate_daily
                 WHERE day_start = ?1 AND interface_id != 0 AND application_id = 0",
                params![day_start],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?
        };
        Ok(total as u64)
    }
}

fn map_hourly(r: &rusqlite::Row<'_>) -> rusqlite::Result<HourlyAggregate> {
    let ssid: String = r.get(3)?;
    Ok(HourlyAggregate {
        hour_start_ms: r.get(0)?,
        interface_name: r.get(1)?,
        application_name: r.get(2)?,
        network_ssid: if ssid.is_empty() { None } else { Some(ssid) },
        bytes_sent: r.get::<_, i64>(4)? as u64,
        bytes_received: r.get::<_, i64>(5)? as u64,
    })
}

fn map_daily(r: &rusqlite::Row<'_>) -> rusqlite::Result<DailyAggregate> {
    let ssid: String = r.get(3)?;
    Ok(DailyAggregate {
        day_start_ms: r.get(0)?,
        interface_name: r.get(1)?,
        application_name: r.get(2)?,
        network_ssid: if ssid.is_empty() { None } else { Some(ssid) },
        bytes_sent: r.get::<_, i64>(4)? as u64,
        bytes_received: r.get::<_, i64>(5)? as u64,
    })
}

fn upsert_application(
    conn: &Connection,
    name: &str,
    path: Option<&str>,
    platform: &str,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO application (name, executable_path, platform) VALUES (?1, ?2, ?3)
         ON CONFLICT(name, executable_path, platform) DO NOTHING",
        params![name, path, platform],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id FROM application WHERE name = ?1 AND executable_path IS ?2 AND platform = ?3",
        params![name, path, platform],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

fn upsert_interface(
    conn: &Connection,
    name: &str,
    iface_type: InterfaceType,
    ssid: Option<&str>,
) -> Result<i64, String> {
    let type_str = match iface_type {
        InterfaceType::Wifi => "wifi",
        InterfaceType::Ethernet => "ethernet",
        InterfaceType::Hotspot => "hotspot",
        InterfaceType::Vpn => "vpn",
        InterfaceType::Bluetooth => "bluetooth",
        InterfaceType::Other => "other",
    };
    conn.execute(
        "INSERT INTO network_interface (name, type, ssid_or_label) VALUES (?1, ?2, ?3)
         ON CONFLICT(name, type, ssid_or_label) DO NOTHING",
        params![name, type_str, ssid],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id FROM network_interface WHERE name = ?1 AND type = ?2 AND ssid_or_label IS ?3",
        params![name, type_str, ssid],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

fn upsert_hourly(
    conn: &Connection,
    ts_ms: i64,
    interface_id: i64,
    application_id: i64,
    ssid: Option<&str>,
    sent: u64,
    recv: u64,
) -> Result<(), String> {
    let hour_start = ts_ms - (ts_ms % 3_600_000);
    let ssid_key = ssid.unwrap_or("");
    conn.execute(
        "INSERT INTO aggregate_hourly (hour_start, interface_id, application_id, network_ssid, bytes_sent, bytes_received)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(hour_start, interface_id, application_id, network_ssid)
         DO UPDATE SET
           bytes_sent = bytes_sent + excluded.bytes_sent,
           bytes_received = bytes_received + excluded.bytes_received",
        params![
            hour_start,
            interface_id,
            application_id,
            ssid_key,
            sent as i64,
            recv as i64
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn upsert_daily(
    conn: &Connection,
    ts_ms: i64,
    interface_id: i64,
    application_id: i64,
    ssid: Option<&str>,
    sent: u64,
    recv: u64,
) -> Result<(), String> {
    let day_start = ts_ms - (ts_ms % 86_400_000);
    let ssid_key = ssid.unwrap_or("");
    conn.execute(
        "INSERT INTO aggregate_daily (day_start, interface_id, application_id, network_ssid, bytes_sent, bytes_received)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(day_start, interface_id, application_id, network_ssid)
         DO UPDATE SET
           bytes_sent = bytes_sent + excluded.bytes_sent,
           bytes_received = bytes_received + excluded.bytes_received",
        params![
            day_start,
            interface_id,
            application_id,
            ssid_key,
            sent as i64,
            recv as i64
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn protocol_str(p: Protocol) -> &'static str {
    match p {
        Protocol::Tcp => "tcp",
        Protocol::Udp => "udp",
        Protocol::Mixed => "mixed",
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
