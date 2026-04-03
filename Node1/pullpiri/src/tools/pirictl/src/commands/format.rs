/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
//! Common formatting utilities for pirictl commands

/// Format bytes into human-readable format (KB, MB, GB)
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format memory into Mi (mebibyte) format
pub fn format_memory(bytes: u64) -> String {
    const MI: u64 = 1024 * 1024;
    format!("{}Mi", bytes / MI)
}

/// Format duration in seconds to human-readable format
/// Returns "X days, Y hours, Z minutes, W seconds" or "X ago" format
pub fn format_duration(seconds: u64) -> String {
    if seconds == 0 {
        return "0 seconds".to_string();
    }

    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{} day{}", days, if days == 1 { "" } else { "s" }));
    }
    if hours > 0 {
        parts.push(format!("{} hour{}", hours, if hours == 1 { "" } else { "s" }));
    }
    if minutes > 0 {
        parts.push(format!("{} minute{}", minutes, if minutes == 1 { "" } else { "s" }));
    }
    if secs > 0 || parts.is_empty() {
        parts.push(format!("{} second{}", secs, if secs == 1 { "" } else { "s" }));
    }

    parts.join(", ")
}

/// Format duration with "ago" suffix for timestamps
pub fn format_duration_ago(seconds: u64) -> String {
    if seconds == 0 {
        return "0 seconds ago".to_string();
    }
    format!("{} ago", format_duration(seconds))
}

/// Capitalize first letter of a string
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Format timestamp from ISO 8601 to readable format
/// Example: "Mon, 03 Mar 2026 16:42:55 +0900"
pub fn format_timestamp(timestamp: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
    use chrono::{DateTime, Local};
    let dt = DateTime::parse_from_rfc3339(timestamp)?;
    let local: DateTime<Local> = dt.with_timezone(&Local);
    Ok(local.format("%a, %d %b %Y %H:%M:%S %z").to_string())
}

/// Calculate uptime from start timestamp
/// Returns format like "29h 23m" or "15m"
pub fn calculate_uptime(started_at: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
    use chrono::{DateTime, Utc};
    let started = DateTime::parse_from_rfc3339(started_at)?;
    let now = Utc::now();
    let duration = now.signed_duration_since(started);
    
    let hours = duration.num_hours();
    let minutes = duration.num_minutes() % 60;
    
    if hours > 0 {
        Ok(format!("{}h {}m", hours, minutes))
    } else {
        Ok(format!("{}m", minutes))
    }
}

/// Calculate age from start timestamp in kubectl style
/// Returns format like "6d", "5h", "30m", "45s"
pub fn calculate_age(started_at: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
    use chrono::{DateTime, Utc};
    
    // Check for invalid timestamps
    if started_at.starts_with("0001-") {
        return Ok("N/A".to_string());
    }
    
    let started = DateTime::parse_from_rfc3339(started_at)?;
    let now = Utc::now();
    let duration = now.signed_duration_since(started);
    
    let days = duration.num_days();
    let hours = duration.num_hours();
    let minutes = duration.num_minutes();
    let seconds = duration.num_seconds();
    
    if days > 0 {
        Ok(format!("{}d", days))
    } else if hours > 0 {
        Ok(format!("{}h", hours))
    } else if minutes > 0 {
        Ok(format!("{}m", minutes))
    } else {
        Ok(format!("{}s", seconds))
    }
}

/// Calculate runtime between two timestamps
/// Returns format like "1.234s" or "0.005s"
pub fn calculate_runtime(started_at: &str, finished_at: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
    use chrono::DateTime;
    
    // Check for invalid timestamps (e.g., "0001-01-01T00:00:00Z")
    if started_at.starts_with("0001-") || finished_at.starts_with("0001-") {
        return Ok("N/A".to_string());
    }
    
    let started = DateTime::parse_from_rfc3339(started_at)?;
    let finished = DateTime::parse_from_rfc3339(finished_at)?;
    let duration = finished.signed_duration_since(started);
    
    let total_secs = duration.num_seconds();
    let millis = duration.num_milliseconds() % 1000;
    
    if total_secs > 0 {
        Ok(format!("{}.{:03}s", total_secs, millis.abs()))
    } else {
        Ok(format!("0.{:03}s", millis.abs()))
    }
}

/// Extract network value from network string
/// Example input: "network: {rx_bytes: 1234, tx_bytes: 5678, ...}"
/// Returns the value for the specified key (e.g., "rx_bytes" -> 1234)
pub fn extract_network_value(networks: &str, key: &str) -> u64 {
    // Simple parsing for "key: value" pattern
    if let Some(start_pos) = networks.find(&format!("{}: ", key)) {
        let after_key = &networks[start_pos + key.len() + 2..];
        if let Some(end_pos) = after_key.find(|c: char| c == ',' || c == '}') {
            let value_str = &after_key[..end_pos].trim();
            return value_str.parse::<u64>().unwrap_or(0);
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_format_memory() {
        assert_eq!(format_memory(0), "0Mi");
        assert_eq!(format_memory(1024 * 1024), "1Mi");
        assert_eq!(format_memory(1024 * 1024 * 1024), "1024Mi");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0 seconds");
        assert_eq!(format_duration(1), "1 second");
        assert_eq!(format_duration(60), "1 minute");
        assert_eq!(format_duration(3600), "1 hour");
        assert_eq!(format_duration(86400), "1 day");
        assert_eq!(format_duration(90), "1 minute, 30 seconds");
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("hello"), "Hello");
        assert_eq!(capitalize("WORLD"), "WORLD");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn test_extract_network_value() {
        let network_str = "network: {rx_bytes: 1234, tx_bytes: 5678}";
        assert_eq!(extract_network_value(network_str, "rx_bytes"), 1234);
        assert_eq!(extract_network_value(network_str, "tx_bytes"), 5678);
        assert_eq!(extract_network_value(network_str, "unknown"), 0);
    }
}
