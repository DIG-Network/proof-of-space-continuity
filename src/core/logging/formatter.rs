use chrono::{DateTime, Utc};
/// Log Formatting Utilities
///
/// This module provides consistent formatting for logs across the system
use colored::*;

/// Format a hash for display (truncated with ellipsis)
pub fn format_hash(hash: &[u8], length: usize) -> ColoredString {
    let hex_str = hex::encode(hash);
    let truncated = if hex_str.len() > length {
        format!("{}...", &hex_str[..length])
    } else {
        hex_str
    };
    truncated.bright_cyan()
}

/// Format a timestamp for logging
pub fn format_timestamp() -> ColoredString {
    let now: DateTime<Utc> = Utc::now();
    now.format("%H:%M:%S").to_string().bright_white()
}

/// Format a file size in human-readable format
pub fn format_file_size(bytes: u64) -> ColoredString {
    if bytes < 1024 {
        format!("{} B", bytes).bright_yellow()
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0).bright_yellow()
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0)).bright_yellow()
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)).bright_yellow()
    }
}

/// Format duration in human-readable format
pub fn format_duration_ms(ms: u64) -> ColoredString {
    if ms < 1000 {
        format!("{}ms", ms).bright_yellow()
    } else if ms < 60000 {
        format!("{:.2}s", ms as f64 / 1000.0).bright_yellow()
    } else {
        let minutes = ms / 60000;
        let seconds = (ms % 60000) as f64 / 1000.0;
        format!("{}m{:.1}s", minutes, seconds).bright_yellow()
    }
}

/// Format a percentage with appropriate color coding
pub fn format_percentage(value: f64) -> ColoredString {
    let percentage_str = format!("{:.1}%", value * 100.0);
    if value >= 0.9 {
        percentage_str.bright_green()
    } else if value >= 0.7 {
        percentage_str.bright_yellow()
    } else {
        percentage_str.bright_red()
    }
}

/// Format a count with thousands separators
pub fn format_count(count: u64) -> ColoredString {
    if count < 1000 {
        count.to_string().bright_green()
    } else if count < 1_000_000 {
        format!("{:.1}K", count as f64 / 1000.0).bright_green()
    } else {
        format!("{:.1}M", count as f64 / 1_000_000.0).bright_green()
    }
}

/// Create a progress bar string
pub fn format_progress_bar(current: u64, total: u64, width: usize) -> String {
    let percentage = if total > 0 {
        (current as f64 / total as f64).min(1.0)
    } else {
        0.0
    };

    let filled = (percentage * width as f64) as usize;
    let empty = width - filled;

    format!(
        "[{}{}] {:.1}%",
        "█".repeat(filled).bright_green(),
        "░".repeat(empty).bright_black(),
        percentage * 100.0
    )
}
