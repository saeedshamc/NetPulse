package com.saeedshamc.netpulse.data

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper

class LocalDb(context: Context) :
    SQLiteOpenHelper(context, "netpulse.db", null, 1) {

    override fun onCreate(db: SQLiteDatabase) {
        db.execSQL(
            """
            CREATE TABLE IF NOT EXISTS application (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL,
              executable_path TEXT,
              platform TEXT NOT NULL,
              UNIQUE(name, executable_path, platform)
            )
            """.trimIndent(),
        )
        db.execSQL(
            """
            CREATE TABLE IF NOT EXISTS network_interface (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL,
              type TEXT NOT NULL,
              ssid_or_label TEXT,
              UNIQUE(name, type, ssid_or_label)
            )
            """.trimIndent(),
        )
        db.execSQL(
            """
            CREATE TABLE IF NOT EXISTS traffic_record (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              timestamp INTEGER NOT NULL,
              interface_id INTEGER,
              application_id INTEGER,
              bytes_sent INTEGER NOT NULL DEFAULT 0,
              bytes_received INTEGER NOT NULL DEFAULT 0,
              network_ssid TEXT,
              protocol TEXT
            )
            """.trimIndent(),
        )
        db.execSQL(
            """
            CREATE TABLE IF NOT EXISTS aggregate_hourly (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              hour_start INTEGER NOT NULL,
              interface_id INTEGER NOT NULL DEFAULT 0,
              application_id INTEGER NOT NULL DEFAULT 0,
              network_ssid TEXT NOT NULL DEFAULT '',
              bytes_sent INTEGER NOT NULL DEFAULT 0,
              bytes_received INTEGER NOT NULL DEFAULT 0,
              UNIQUE(hour_start, interface_id, application_id, network_ssid)
            )
            """.trimIndent(),
        )
        db.execSQL(
            """
            CREATE TABLE IF NOT EXISTS aggregate_daily (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              day_start INTEGER NOT NULL,
              interface_id INTEGER NOT NULL DEFAULT 0,
              application_id INTEGER NOT NULL DEFAULT 0,
              network_ssid TEXT NOT NULL DEFAULT '',
              bytes_sent INTEGER NOT NULL DEFAULT 0,
              bytes_received INTEGER NOT NULL DEFAULT 0,
              UNIQUE(day_start, interface_id, application_id, network_ssid)
            )
            """.trimIndent(),
        )
    }

    override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) = Unit

    fun recordSnapshot(snapshot: MonitorSnapshot) {
        val db = writableDatabase
        val ts = snapshot.timestampMs
        db.beginTransaction()
        try {
            for (app in snapshot.apps) {
                if (app.bytesSent == 0L && app.bytesReceived == 0L) continue
                val appId = upsertApplication(db, app.label, app.packageName)
                db.execSQL(
                    """
                    INSERT INTO traffic_record
                    (timestamp, interface_id, application_id, bytes_sent, bytes_received, network_ssid, protocol)
                    VALUES (?, NULL, ?, ?, ?, ?, 'mixed')
                    """.trimIndent(),
                    arrayOf(ts, appId, app.bytesSent, app.bytesReceived, snapshot.currentSsid),
                )
                upsertAggregate(db, "aggregate_hourly", ts - (ts % 3_600_000), 0, appId, snapshot.currentSsid, app.bytesSent, app.bytesReceived)
                upsertAggregate(db, "aggregate_daily", ts - (ts % 86_400_000), 0, appId, snapshot.currentSsid, app.bytesSent, app.bytesReceived)
            }
            for (iface in snapshot.interfaces) {
                if (iface.bytesSent == 0L && iface.bytesReceived == 0L) continue
                val ifaceId = upsertInterface(db, iface.name, iface.type, iface.ssid)
                db.execSQL(
                    """
                    INSERT INTO traffic_record
                    (timestamp, interface_id, application_id, bytes_sent, bytes_received, network_ssid, protocol)
                    VALUES (?, ?, NULL, ?, ?, ?, NULL)
                    """.trimIndent(),
                    arrayOf(ts, ifaceId, iface.bytesSent, iface.bytesReceived, iface.ssid),
                )
                upsertAggregate(db, "aggregate_hourly", ts - (ts % 3_600_000), ifaceId, 0, iface.ssid, iface.bytesSent, iface.bytesReceived)
                upsertAggregate(db, "aggregate_daily", ts - (ts % 86_400_000), ifaceId, 0, iface.ssid, iface.bytesSent, iface.bytesReceived)
            }
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
    }

    private fun upsertApplication(db: SQLiteDatabase, name: String, path: String): Long {
        db.execSQL(
            """
            INSERT OR IGNORE INTO application (name, executable_path, platform)
            VALUES (?, ?, 'android')
            """.trimIndent(),
            arrayOf(name, path),
        )
        db.rawQuery(
            "SELECT id FROM application WHERE name = ? AND executable_path = ? AND platform = 'android'",
            arrayOf(name, path),
        ).use { c ->
            c.moveToFirst()
            return c.getLong(0)
        }
    }

    private fun upsertInterface(db: SQLiteDatabase, name: String, type: String, ssid: String?): Long {
        db.execSQL(
            """
            INSERT OR IGNORE INTO network_interface (name, type, ssid_or_label)
            VALUES (?, ?, ?)
            """.trimIndent(),
            arrayOf(name, type, ssid),
        )
        db.rawQuery(
            "SELECT id FROM network_interface WHERE name = ? AND type = ? AND IFNULL(ssid_or_label,'') = IFNULL(?, '')",
            arrayOf(name, type, ssid),
        ).use { c ->
            c.moveToFirst()
            return c.getLong(0)
        }
    }

    data class UsageTotals(val sent: Long, val received: Long) {
        val total: Long get() = sent + received
    }

    fun todayTotals(): UsageTotals {
        val now = System.currentTimeMillis()
        val dayStart = now - (now % 86_400_000L)
        return sumDailyFrom(dayStart)
    }

    fun monthTotals(): UsageTotals {
        val now = System.currentTimeMillis()
        val day = java.util.Calendar.getInstance().apply { timeInMillis = now }
        day.set(java.util.Calendar.DAY_OF_MONTH, 1)
        day.set(java.util.Calendar.HOUR_OF_DAY, 0)
        day.set(java.util.Calendar.MINUTE, 0)
        day.set(java.util.Calendar.SECOND, 0)
        day.set(java.util.Calendar.MILLISECOND, 0)
        return sumDailyFrom(day.timeInMillis)
    }

    private fun sumDailyFrom(startMs: Long): UsageTotals {
        readableDatabase.rawQuery(
            """
            SELECT COALESCE(SUM(bytes_sent), 0), COALESCE(SUM(bytes_received), 0)
            FROM aggregate_daily
            WHERE day_start >= ? AND application_id = 0
            """.trimIndent(),
            arrayOf(startMs.toString()),
        ).use { c ->
            c.moveToFirst()
            return UsageTotals(c.getLong(0), c.getLong(1))
        }
    }

    private fun upsertAggregate(
        db: SQLiteDatabase,
        table: String,
        start: Long,
        interfaceId: Long,
        applicationId: Long,
        ssid: String?,
        sent: Long,
        recv: Long,
    ) {
        val ssidKey = ssid.orEmpty()
        val startCol = if (table.contains("hourly")) "hour_start" else "day_start"
        db.execSQL(
            """
            INSERT INTO $table ($startCol, interface_id, application_id, network_ssid, bytes_sent, bytes_received)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT($startCol, interface_id, application_id, network_ssid)
            DO UPDATE SET
              bytes_sent = bytes_sent + excluded.bytes_sent,
              bytes_received = bytes_received + excluded.bytes_received
            """.trimIndent(),
            arrayOf(start, interfaceId, applicationId, ssidKey, sent, recv),
        )
    }
}
