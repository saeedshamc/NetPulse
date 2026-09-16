package com.saeedshamc.netpulse.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val colors = darkColorScheme(
    primary = Color(0xFF3ECF8E),
    onPrimary = Color(0xFF062016),
    background = Color(0xFF0F1714),
    surface = Color(0xFF16221C),
    onBackground = Color(0xFFE7F0EA),
    onSurface = Color(0xFFE7F0EA),
    secondary = Color(0xFF6AA8FF),
)

@Composable
fun NetPulseTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = colors,
        content = content,
    )
}
