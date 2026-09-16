package com.saeedshamc.netpulse.data

import android.app.AppOpsManager
import android.app.usage.NetworkStats
import android.app.usage.NetworkStatsManager
import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Process
import android.provider.Settings
import androidx.core.content.getSystemService

class NetworkStatsRepository(private val context: Context) {
    private val statsManager = context.getSystemService<NetworkStatsManager>()
        ?: error("NetworkStatsManager unavailable")
    private val packageManager = context.packageManager

    fun hasUsageAccess(): Boolean {
        val appOps = context.getSystemService<AppOpsManager>() ?: return false
        val mode = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            appOps.unsafeCheckOpNoThrow(
                AppOpsManager.OPSTR_GET_USAGE_STATS,
                Process.myUid(),
                context.packageName,
            )
        } else {
            @Suppress("DEPRECATION")
            appOps.checkOpNoThrow(
                AppOpsManager.OPSTR_GET_USAGE_STATS,
                Process.myUid(),
                context.packageName,
            )
        }
        return mode == AppOpsManager.MODE_ALLOWED
    }

    fun usageAccessSettingsIntent(): Intent =
        Intent(Settings.ACTION_USAGE_ACCESS_SETTINGS)

    fun currentSsid(): String? {
        val wifi = context.getSystemService<WifiManager>() ?: return null
        @Suppress("DEPRECATION")
        val info = wifi.connectionInfo ?: return null
        val ssid = info.ssid ?: return null
        if (ssid == "<unknown ssid>" || ssid == "0x") return null
        return ssid.trim('"')
    }

    fun snapshot(rangeStartMs: Long, rangeEndMs: Long): MonitorSnapshot {
        val note = if (hasUsageAccess()) {
            null
        } else {
            "Grant Usage access in system settings so NetPulse can read NetworkStatsManager history."
        }

        if (!hasUsageAccess()) {
            return MonitorSnapshot(
                timestampMs = System.currentTimeMillis(),
                apps = emptyList(),
                interfaces = emptyList(),
                currentSsid = currentSsid(),
                privilegeNote = note,
            )
        }

        val apps = queryPerApp(rangeStartMs, rangeEndMs)
        val interfaces = queryPerInterface(rangeStartMs, rangeEndMs)
        return MonitorSnapshot(
            timestampMs = System.currentTimeMillis(),
            apps = apps,
            interfaces = interfaces,
            currentSsid = currentSsid(),
            privilegeNote = note,
        )
    }

    fun hourlyBuckets(rangeStartMs: Long, rangeEndMs: Long): List<UsageBucket> {
        if (!hasUsageAccess()) return emptyList()
        return queryBuckets(ConnectivityManager.TYPE_WIFI, rangeStartMs, rangeEndMs, bucketMs = 3_600_000L) +
            queryBuckets(ConnectivityManager.TYPE_MOBILE, rangeStartMs, rangeEndMs, bucketMs = 3_600_000L)
    }

    fun dailyBuckets(rangeStartMs: Long, rangeEndMs: Long): List<UsageBucket> {
        if (!hasUsageAccess()) return emptyList()
        return queryBuckets(ConnectivityManager.TYPE_WIFI, rangeStartMs, rangeEndMs, bucketMs = 86_400_000L) +
            queryBuckets(ConnectivityManager.TYPE_MOBILE, rangeStartMs, rangeEndMs, bucketMs = 86_400_000L)
    }

    fun exportCsv(rangeStartMs: Long, rangeEndMs: Long): String {
        val rows = hourlyBuckets(rangeStartMs, rangeEndMs)
        val sb = StringBuilder("start_ms,end_ms,interface,ssid,package,bytes_sent,bytes_received\n")
        for (row in rows) {
            sb.append(row.startMs).append(',')
                .append(row.endMs).append(',')
                .append(row.interfaceName.orEmpty()).append(',')
                .append(row.ssid.orEmpty()).append(',')
                .append(row.packageName.orEmpty()).append(',')
                .append(row.bytesSent).append(',')
                .append(row.bytesReceived).append('\n')
        }
        return sb.toString()
    }

    fun exportJson(rangeStartMs: Long, rangeEndMs: Long): String {
        val rows = hourlyBuckets(rangeStartMs, rangeEndMs)
        val body = rows.joinToString(",\n") { row ->
            """{"start_ms":${row.startMs},"end_ms":${row.endMs},"interface":"${row.interfaceName.orEmpty()}","ssid":"${row.ssid.orEmpty()}","package":"${row.packageName.orEmpty()}","bytes_sent":${row.bytesSent},"bytes_received":${row.bytesReceived}}"""
        }
        return "[\n$body\n]"
    }

    private fun queryPerApp(start: Long, end: Long): List<AppTraffic> {
        val aggregated = linkedMapOf<Int, Pair<Long, Long>>()
        for (type in listOf(ConnectivityManager.TYPE_WIFI, ConnectivityManager.TYPE_MOBILE)) {
            val subscriberId = subscriberIdFor(type)
            val stats = statsManager.querySummary(type, subscriberId, start, end)
            val bucket = NetworkStats.Bucket()
            while (stats.hasNextBucket()) {
                stats.getNextBucket(bucket)
                if (bucket.uid == NetworkStats.Bucket.UID_ALL ||
                    bucket.uid == NetworkStats.Bucket.UID_REMOVED ||
                    bucket.uid == NetworkStats.Bucket.UID_TETHERING
                ) {
                    continue
                }
                val prev = aggregated[bucket.uid] ?: (0L to 0L)
                aggregated[bucket.uid] =
                    (prev.first + bucket.txBytes) to (prev.second + bucket.rxBytes)
            }
            stats.close()
        }

        return aggregated.map { (uid, bytes) ->
            val pkg = packageManager.getPackagesForUid(uid)?.firstOrNull() ?: "uid:$uid"
            val label = try {
                val info: ApplicationInfo = packageManager.getApplicationInfo(pkg, 0)
                packageManager.getApplicationLabel(info).toString()
            } catch (_: PackageManager.NameNotFoundException) {
                pkg
            }
            AppTraffic(
                uid = uid,
                packageName = pkg,
                label = label,
                bytesSent = bytes.first,
                bytesReceived = bytes.second,
            )
        }.sortedByDescending { it.bytesSent + it.bytesReceived }
    }

    private fun queryPerInterface(start: Long, end: Long): List<InterfaceTraffic> {
        val ssid = currentSsid()
        val result = mutableListOf<InterfaceTraffic>()
        for ((type, name) in listOf(
            ConnectivityManager.TYPE_WIFI to "wifi",
            ConnectivityManager.TYPE_MOBILE to "mobile",
        )) {
            val subscriberId = subscriberIdFor(type)
            val bucket = statsManager.querySummaryForDevice(type, subscriberId, start, end)
            result += InterfaceTraffic(
                name = name,
                type = name,
                bytesSent = bucket.txBytes,
                bytesReceived = bucket.rxBytes,
                ssid = if (type == ConnectivityManager.TYPE_WIFI) ssid else null,
            )
        }
        // Active transports summary for VPN detection
        val cm = context.getSystemService<ConnectivityManager>()
        val active = cm?.getNetworkCapabilities(cm.activeNetwork)
        if (active?.hasTransport(NetworkCapabilities.TRANSPORT_VPN) == true) {
            result += InterfaceTraffic(
                name = "vpn",
                type = "vpn",
                bytesSent = 0,
                bytesReceived = 0,
                ssid = null,
            )
        }
        return result.sortedByDescending { it.bytesSent + it.bytesReceived }
    }

    private fun queryBuckets(
        type: Int,
        start: Long,
        end: Long,
        bucketMs: Long,
    ): List<UsageBucket> {
        val subscriberId = subscriberIdFor(type)
        val out = mutableListOf<UsageBucket>()
        var cursor = start
        val iface = if (type == ConnectivityManager.TYPE_WIFI) "wifi" else "mobile"
        val ssid = if (type == ConnectivityManager.TYPE_WIFI) currentSsid() else null
        while (cursor < end) {
            val next = minOf(cursor + bucketMs, end)
            val bucket = statsManager.querySummaryForDevice(type, subscriberId, cursor, next)
            if (bucket.txBytes > 0 || bucket.rxBytes > 0) {
                out += UsageBucket(
                    startMs = cursor,
                    endMs = next,
                    bytesSent = bucket.txBytes,
                    bytesReceived = bucket.rxBytes,
                    interfaceName = iface,
                    ssid = ssid,
                )
            }
            cursor = next
        }
        return out
    }

    @Suppress("DEPRECATION", "HardwareIds")
    private fun subscriberIdFor(type: Int): String? {
        if (type != ConnectivityManager.TYPE_MOBILE) return null
        // On modern Android, null is accepted for Wi-Fi and often for mobile summaries.
        return null
    }
}
