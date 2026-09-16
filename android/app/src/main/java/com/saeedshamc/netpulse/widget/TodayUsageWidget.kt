package com.saeedshamc.netpulse.widget

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.widget.RemoteViews
import com.saeedshamc.netpulse.MainActivity
import com.saeedshamc.netpulse.R
import com.saeedshamc.netpulse.data.LocalDb
import java.util.Locale

class TodayUsageWidget : AppWidgetProvider() {
    override fun onUpdate(
        context: Context,
        appWidgetManager: AppWidgetManager,
        appWidgetIds: IntArray,
    ) {
        for (id in appWidgetIds) {
            updateWidget(context, appWidgetManager, id)
        }
    }

    companion object {
        fun refreshAll(context: Context) {
            val manager = AppWidgetManager.getInstance(context)
            val ids = manager.getAppWidgetIds(ComponentName(context, TodayUsageWidget::class.java))
            if (ids.isEmpty()) return
            val provider = TodayUsageWidget()
            provider.onUpdate(context, manager, ids)
        }

        fun updateWidget(context: Context, manager: AppWidgetManager, appWidgetId: Int) {
            val db = LocalDb(context.applicationContext)
            val today = db.todayTotals()
            val month = db.monthTotals()
            val views = RemoteViews(context.packageName, R.layout.widget_today)
            views.setTextViewText(R.id.widget_title, "NetPulse")
            views.setTextViewText(R.id.widget_subtitle, context.getString(R.string.widget_today_label))
            views.setTextViewText(R.id.widget_usage, formatBytes(today.total))
            views.setTextViewText(
                R.id.widget_detail,
                String.format(
                    Locale.US,
                    "↑ %s  ↓ %s · %s %s",
                    formatBytes(today.sent),
                    formatBytes(today.received),
                    context.getString(R.string.widget_month_label),
                    formatBytes(month.total),
                ),
            )

            val intent = Intent(context, MainActivity::class.java)
            val pending = PendingIntent.getActivity(
                context,
                0,
                intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
            views.setOnClickPendingIntent(R.id.widget_root, pending)
            manager.updateAppWidget(appWidgetId, views)
        }

        private fun formatBytes(bytes: Long): String {
            if (bytes < 1024) return "$bytes B"
            val units = arrayOf("KB", "MB", "GB", "TB")
            var value = bytes.toDouble()
            var idx = -1
            while (value >= 1024 && idx < units.lastIndex) {
                value /= 1024.0
                idx++
            }
            return String.format(Locale.US, "%.1f %s", value, units[idx.coerceAtLeast(0)])
        }
    }
}
