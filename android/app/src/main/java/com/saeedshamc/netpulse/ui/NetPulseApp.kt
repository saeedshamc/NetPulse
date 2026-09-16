package com.saeedshamc.netpulse.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.saeedshamc.netpulse.data.AppTraffic
import com.saeedshamc.netpulse.data.InterfaceTraffic
import com.saeedshamc.netpulse.data.UsageBucket
import java.text.DateFormat
import java.util.Date
import kotlin.math.ln
import kotlin.math.pow

@Composable
fun NetPulseApp(vm: NetPulseViewModel = viewModel()) {
    val snap = vm.snapshot
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .padding(16.dp),
    ) {
        Text(
            text = "NetPulse",
            style = MaterialTheme.typography.headlineLarge,
            color = MaterialTheme.colorScheme.primary,
            fontWeight = FontWeight.SemiBold,
        )
        Text(
            text = "Every byte, every app, every network — local and yours.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.7f),
        )
        Text(
            text = "SSID: ${snap?.currentSsid ?: "—"}",
            style = MaterialTheme.typography.bodySmall,
            modifier = Modifier.padding(top = 8.dp),
        )

        snap?.privilegeNote?.let { note ->
            Spacer(Modifier = Modifier.height(8.dp))
            Text(text = note, color = MaterialTheme.colorScheme.secondary)
            TextButton(onClick = vm::openUsageAccess) {
                Text("Open Usage Access settings")
            }
        }

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Tab.entries.forEach { tab ->
                val selected = vm.tab == tab
                Text(
                    text = tab.name,
                    color = if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f),
                    fontWeight = if (selected) FontWeight.Bold else FontWeight.Normal,
                    modifier = Modifier
                        .clickable { vm.selectTab(tab) }
                        .padding(horizontal = 8.dp, vertical = 4.dp),
                )
            }
        }

        when (vm.tab) {
            Tab.Apps -> AppsList(snap?.apps.orEmpty())
            Tab.Interfaces -> InterfacesList(snap?.interfaces.orEmpty())
            Tab.History -> HistoryList(
                buckets = vm.history,
                daily = vm.grainDaily,
                onToggle = vm::setGrainDaily,
            )
            Tab.Export -> ExportPanel(
                text = vm.exportText,
                onCsv = { vm.generateExport(true) },
                onJson = { vm.generateExport(false) },
            )
        }
    }
}

@Composable
private fun AppsList(apps: List<AppTraffic>) {
    LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        items(apps, key = { it.uid }) { app ->
            Column(Modifier = Modifier.fillMaxWidth()) {
                Text(app.label, fontWeight = FontWeight.Medium)
                Text(app.packageName, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f))
                Text(
                    "↑ ${formatBytes(app.bytesSent)}   ↓ ${formatBytes(app.bytesReceived)}",
                    fontFamily = FontFamily.Monospace,
                )
            }
        }
    }
}

@Composable
private fun InterfacesList(items: List<InterfaceTraffic>) {
    LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        items(items, key = { it.name }) { iface ->
            Column(Modifier = Modifier.fillMaxWidth()) {
                Text("${iface.name} (${iface.type})", fontWeight = FontWeight.Medium)
                Text("SSID: ${iface.ssid ?: "—"}", style = MaterialTheme.typography.bodySmall)
                Text(
                    "↑ ${formatBytes(iface.bytesSent)}   ↓ ${formatBytes(iface.bytesReceived)}",
                    fontFamily = FontFamily.Monospace,
                )
            }
        }
    }
}

@Composable
private fun HistoryList(
    buckets: List<UsageBucket>,
    daily: Boolean,
    onToggle: (Boolean) -> Unit,
) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        TextButton(onClick = { onToggle(false) }) {
            Text(if (!daily) "Hourly ✓" else "Hourly")
        }
        TextButton(onClick = { onToggle(true) }) {
            Text(if (daily) "Daily ✓" else "Daily")
        }
    }
    LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        items(buckets) { b ->
            val label = DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT)
                .format(Date(b.startMs))
            Column {
                Text("$label · ${b.interfaceName.orEmpty()} ${b.ssid?.let { "· $it" }.orEmpty()}")
                Text(
                    "↑ ${formatBytes(b.bytesSent)}   ↓ ${formatBytes(b.bytesReceived)}",
                    fontFamily = FontFamily.Monospace,
                )
            }
        }
    }
}

@Composable
private fun ExportPanel(text: String, onCsv: () -> Unit, onJson: () -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(onClick = onCsv) { Text("Generate CSV") }
        Button(onClick = onJson) { Text("Generate JSON") }
    }
    Spacer(Modifier = Modifier.height(12.dp))
    Text(
        text = text.ifBlank { "Export preview appears here (last 90 days)." },
        fontFamily = FontFamily.Monospace,
        style = MaterialTheme.typography.bodySmall,
    )
}

private fun formatBytes(value: Long): String {
    if (value <= 0) return "0 B"
    val units = arrayOf("B", "KB", "MB", "GB", "TB")
    val exp = (ln(value.toDouble()) / ln(1024.0)).toInt().coerceIn(0, units.lastIndex)
    val n = value / 1024.0.pow(exp.toDouble())
    return if (exp == 0) "$value B" else String.format("%.1f %s", n, units[exp])
}
