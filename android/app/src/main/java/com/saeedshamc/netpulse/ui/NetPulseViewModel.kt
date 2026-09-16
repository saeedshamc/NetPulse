package com.saeedshamc.netpulse.ui

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.saeedshamc.netpulse.data.LocalDb
import com.saeedshamc.netpulse.data.MonitorSnapshot
import com.saeedshamc.netpulse.data.NetworkStatsRepository
import com.saeedshamc.netpulse.data.UsageBucket
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

enum class Tab { Apps, Interfaces, History, Export }

class NetPulseViewModel(app: Application) : AndroidViewModel(app) {
    private val repo = NetworkStatsRepository(app)
    private val db = LocalDb(app)

    var tab by mutableStateOf(Tab.Apps)
        private set
    var snapshot by mutableStateOf<MonitorSnapshot?>(null)
        private set
    var history by mutableStateOf<List<UsageBucket>>(emptyList())
        private set
    var exportText by mutableStateOf("")
        private set
    var grainDaily by mutableStateOf(true)
        private set

    init {
        viewModelScope.launch {
            while (isActive) {
                refresh()
                delay(5_000)
            }
        }
    }

    fun selectTab(value: Tab) {
        tab = value
        if (value == Tab.History) refreshHistory()
    }

    fun setGrainDaily(daily: Boolean) {
        grainDaily = daily
        refreshHistory()
    }

    fun openUsageAccess() {
        getApplication<Application>().startActivity(
            repo.usageAccessSettingsIntent().addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK),
        )
    }

    fun generateExport(csv: Boolean) {
        viewModelScope.launch {
            val end = System.currentTimeMillis()
            val start = end - 90L * 86_400_000L
            exportText = withContext(Dispatchers.IO) {
                if (csv) repo.exportCsv(start, end) else repo.exportJson(start, end)
            }
        }
    }

    private suspend fun refresh() {
        val end = System.currentTimeMillis()
        val start = end - 24L * 86_400_000L
        val snap = withContext(Dispatchers.IO) {
            val s = repo.snapshot(start, end)
            if (repo.hasUsageAccess()) {
                db.recordSnapshot(s)
                com.saeedshamc.netpulse.widget.TodayUsageWidget.refreshAll(getApplication())
            }
            s
        }
        snapshot = snap
    }

    private fun refreshHistory() {
        viewModelScope.launch {
            val end = System.currentTimeMillis()
            val start = if (grainDaily) end - 30L * 86_400_000L else end - 48L * 3_600_000L
            history = withContext(Dispatchers.IO) {
                if (grainDaily) repo.dailyBuckets(start, end) else repo.hourlyBuckets(start, end)
            }
        }
    }
}
