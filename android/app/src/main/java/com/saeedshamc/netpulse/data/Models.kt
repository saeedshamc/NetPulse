package com.saeedshamc.netpulse.data

data class AppTraffic(
    val uid: Int,
    val packageName: String,
    val label: String,
    val bytesSent: Long,
    val bytesReceived: Long,
)

data class InterfaceTraffic(
    val name: String,
    val type: String,
    val bytesSent: Long,
    val bytesReceived: Long,
    val ssid: String?,
)

data class UsageBucket(
    val startMs: Long,
    val endMs: Long,
    val bytesSent: Long,
    val bytesReceived: Long,
    val packageName: String? = null,
    val ssid: String? = null,
    val interfaceName: String? = null,
)

data class MonitorSnapshot(
    val timestampMs: Long,
    val apps: List<AppTraffic>,
    val interfaces: List<InterfaceTraffic>,
    val currentSsid: String?,
    val privilegeNote: String?,
)
