use chrono::{DateTime, Local, TimeZone, Utc};
use fitparser::FitDataRecord;

use crate::FitError;

/// Controls how timestamps are rendered for display.
#[derive(Debug, Clone, Default)]
pub enum TzMode {
    /// Display in the system local timezone (default).
    #[default]
    Local,
    /// Display in UTC.
    Utc,
    /// Display in the given IANA timezone name (e.g. `"Europe/Berlin"`).
    Named(String),
}

/// Format a local timestamp for display according to `mode`.
pub fn format_ts(ts: DateTime<Local>, mode: &TzMode) -> String {
    match mode {
        TzMode::Local => ts.to_string(),
        TzMode::Utc => ts.with_timezone(&Utc).to_string(),
        // Named timezone support requires the `chrono-tz` crate; fall back to
        // UTC until that dependency is added in a later step.
        TzMode::Named(name) => {
            eprintln!("Warning: named timezone '{}' not yet supported; using UTC", name);
            ts.with_timezone(&Utc).to_string()
        }
    }
}

/// Format a record as `"<kind>: <timestamp>"` for human-readable display.
///
/// Migrated from `main.rs::kind_and_ts_to_str`.
pub fn kind_and_ts_to_str(record: &FitDataRecord) -> String {
    let kind = record.kind();
    let ts_str = crate::filter::record_timestamp(record)
        .map(|ts| ts.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    format!("{}: {}", kind, ts_str)
}

/// Parse a timestamp string into a [`DateTime<Local>`].
///
/// Accepted formats:
/// - ISO 8601 with timezone: `2026-04-23T09:58:43+02:00`
/// - ISO 8601 without timezone (assumed local): `2026-04-23T09:58:43`
/// - `HH:MM:SS` relative offset from `activity_start` (requires `activity_start` to be `Some`)
///
/// Returns `FitError::TimestampMissing` if the string cannot be parsed.
pub fn parse_timestamp(
    s: &str,
    activity_start: Option<DateTime<Local>>,
) -> Result<DateTime<Local>, FitError> {
    // Try relative HH:MM:SS first
    if let Some(base) = activity_start
        && let Some(ts) = parse_relative(s, base)
    {
        return Ok(ts);
    }

    // Try ISO 8601 with timezone offset
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Local));
    }

    // Try ISO 8601 without timezone (assume local)
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Local
            .from_local_datetime(&naive)
            .single()
            .ok_or_else(|| FitError::TimestampMissing { kind: s.to_string() });
    }

    Err(FitError::TimestampMissing { kind: s.to_string() })
}

fn parse_relative(s: &str, base: DateTime<Local>) -> Option<DateTime<Local>> {
    let parts: Vec<&str> = s.splitn(3, ':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: i64 = parts[0].parse().ok()?;
    let m: i64 = parts[1].parse().ok()?;
    let sec: i64 = parts[2].parse().ok()?;
    let offset = chrono::TimeDelta::seconds(h * 3600 + m * 60 + sec);
    Some(base + offset)
}
