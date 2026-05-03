use std::collections::HashMap;

use chrono::{DateTime, Local};
use fitparser::profile::field_types::MesgNum;
use fitparser::{FitDataRecord, Value};

/// Count occurrences of every message kind present in `data`.
///
/// The returned map uses the human-readable kind name as key (e.g. `"record"`,
/// `"lap"`, `"session"`).
pub fn count_kinds(data: &[FitDataRecord]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for entry in data {
        *counts.entry(entry.kind().to_string()).or_insert(0) += 1;
    }
    counts
}

/// Return references to every record whose message kind matches `kind`.
///
/// Callers that need owned values can `.cloned()` the result.
pub fn select_kind(data: &[FitDataRecord], kind: MesgNum) -> Vec<&FitDataRecord> {
    data.iter().filter(|e| e.kind() == kind).collect()
}

/// Return references to records of `kind` whose `timestamp` field falls in
/// the half-open interval `[from, until)`.
///
/// Records that lack a `timestamp` field or whose timestamp is not a
/// [`Value::Timestamp`] variant are silently skipped.
pub fn select_kind_with_ts(
    data: &[FitDataRecord],
    kind: MesgNum,
    from: DateTime<Local>,
    until: DateTime<Local>,
) -> Vec<&FitDataRecord> {
    data.iter()
        .filter(|e| {
            if e.kind() != kind {
                return false;
            }
            matches!(record_timestamp(e), Some(t) if t >= from && t < until)
        })
        .collect()
}

/// Extract the `timestamp` field value from a record, if present and valid.
///
/// Returns `None` if the field is absent or holds an unexpected value variant.
pub fn record_timestamp(record: &FitDataRecord) -> Option<DateTime<Local>> {
    record
        .fields()
        .iter()
        .find(|f| f.name() == "timestamp")
        .and_then(|f| {
            if let Value::Timestamp(t) = f.value() {
                Some(*t)
            } else {
                None
            }
        })
}
