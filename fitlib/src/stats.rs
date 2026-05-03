/// Aggregated statistics over a set of FIT records.
///
/// All numeric fields found in the record set are summarised.
/// Use `field_filter` to restrict which fields are included.
use fitparser::{FitDataRecord, Value};
use serde::Serialize;

use crate::hierarchy::{FitActivity, FitSession};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct FieldStats {
    pub field: String,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub sum: f64,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordSetStats {
    pub label: String,
    pub fields: Vec<FieldStats>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Aggregate numeric fields across `records`.
///
/// `field_filter`: if non-empty, only these field names are included;
/// otherwise all numeric fields are aggregated.
pub fn aggregate(records: &[&FitDataRecord], field_filter: &[&str]) -> RecordSetStats {
    aggregate_labeled("activity".to_string(), records, field_filter)
}

/// Compute per-lap statistics for a session.
///
/// Returns one [`RecordSetStats`] per lap in the session.
pub fn per_lap(
    data: &[FitDataRecord],
    session: &FitSession,
    field_filter: &[&str],
) -> Vec<RecordSetStats> {
    session
        .laps
        .iter()
        .map(|lap| {
            let records: Vec<&FitDataRecord> =
                lap.record_indices.iter().map(|&i| &data[i]).collect();
            aggregate_labeled(
                format!("Session {} / Lap {}", session.index, lap.index),
                &records,
                field_filter,
            )
        })
        .collect()
}

/// Compute per-session statistics for an activity.
///
/// Returns one [`RecordSetStats`] per session.
pub fn per_session(
    data: &[FitDataRecord],
    activity: &FitActivity,
    field_filter: &[&str],
) -> Vec<RecordSetStats> {
    activity
        .sessions
        .iter()
        .map(|session| {
            // Collect all record indices across all laps in the session.
            let records: Vec<&FitDataRecord> = session
                .laps
                .iter()
                .flat_map(|lap| lap.record_indices.iter().map(|&i| &data[i]))
                .collect();
            aggregate_labeled(
                format!("Session {}", session.index),
                &records,
                field_filter,
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn aggregate_labeled(
    label: String,
    records: &[&FitDataRecord],
    field_filter: &[&str],
) -> RecordSetStats {
    use std::collections::HashMap;

    // Accumulate (sum, count, min, max) per field name.
    let mut acc: HashMap<String, (f64, usize, f64, f64)> = HashMap::new();

    for record in records {
        for field in record.fields() {
            if !field_filter.is_empty() && !field_filter.contains(&field.name()) {
                continue;
            }
            if let Some(v) = numeric_value(field.value()) {
                let entry = acc.entry(field.name().to_string()).or_insert((0.0, 0, f64::MAX, f64::MIN));
                entry.0 += v;
                entry.1 += 1;
                entry.2 = entry.2.min(v);
                entry.3 = entry.3.max(v);
            }
        }
    }

    let mut fields: Vec<FieldStats> = acc
        .into_iter()
        .map(|(name, (sum, count, min, max))| FieldStats {
            field: name,
            min,
            max,
            mean: if count > 0 { sum / count as f64 } else { 0.0 },
            sum,
            count,
        })
        .collect();
    fields.sort_by(|a, b| a.field.cmp(&b.field));

    RecordSetStats { label, fields }
}

fn numeric_value(v: &Value) -> Option<f64> {
    match v {
        Value::SInt8(x)   => Some(*x as f64),
        Value::UInt8(x)   => Some(*x as f64),
        Value::SInt16(x)  => Some(*x as f64),
        Value::UInt16(x)  => Some(*x as f64),
        Value::SInt32(x)  => Some(*x as f64),
        Value::UInt32(x)  => Some(*x as f64),
        Value::SInt64(x)  => Some(*x as f64),
        Value::UInt64(x)  => Some(*x as f64),
        Value::Float32(x) => Some(*x as f64),
        Value::Float64(x) => Some(*x),
        Value::UInt8z(x)  => Some(*x as f64),
        Value::UInt16z(x) => Some(*x as f64),
        Value::UInt32z(x) => Some(*x as f64),
        Value::UInt64z(x) => Some(*x as f64),
        _ => None,
    }
}
