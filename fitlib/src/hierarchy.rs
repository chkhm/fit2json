/// Logical tree reconstruction of a FIT activity file.
///
/// A FIT file is a flat, chronological binary stream.  This module reconstructs
/// the containment hierarchy (Activity → Session → Lap → Records) by matching
/// `start_time` and `timestamp` (end time) fields on `session` and `lap`
/// summary messages.
use chrono::{DateTime, Local};
use fitparser::profile::field_types::MesgNum;
use fitparser::{FitDataRecord, Value};
use serde::Serialize;

use crate::filter::record_timestamp;
use crate::FitError;

// Type aliases to keep complex tuple types readable.
type LapTimeWindow     = (Option<DateTime<Local>>, Option<DateTime<Local>>);
type SessionTimeWindow = (Option<DateTime<Local>>, Option<DateTime<Local>>, Option<String>, Option<String>);

// ---------------------------------------------------------------------------
// Public domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct FileIdInfo {
    pub time_created: Option<DateTime<Local>>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FitActivity {
    pub file_id: FileIdInfo,
    /// Matches `activity.num_sessions`; equal to `sessions.len()`.
    pub num_sessions: usize,
    pub sessions: Vec<FitSession>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FitSession {
    /// 1-based session index within the activity.
    pub index: usize,
    pub sport: Option<String>,
    pub sub_sport: Option<String>,
    pub start_time: Option<DateTime<Local>>,
    pub end_time: Option<DateTime<Local>>,
    pub laps: Vec<FitLap>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FitLap {
    /// 1-based lap index within the session.
    pub index: usize,
    pub start_time: Option<DateTime<Local>>,
    pub end_time: Option<DateTime<Local>>,
    /// Indices into the original flat `data` slice for every `record` message
    /// whose timestamp falls within `[start_time, end_time]`.
    pub record_indices: Vec<usize>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a [`FitActivity`] from a flat record slice by reconstructing the
/// Session → Lap → Record containment hierarchy.
///
/// ## Algorithm
/// 1. Extract all `lap` summary records; each carries `start_time` and
///    `timestamp` (end time).
/// 2. For each lap, collect the indices of all `record` messages whose
///    timestamp falls in `[lap.start_time, lap.timestamp]`.
/// 3. Extract all `session` summary records (same fields).
/// 4. Assign each lap to the session that contains its `start_time`.
/// 5. Populate [`FileIdInfo`] from the first `file_id` record.
pub fn build_activity(data: &[FitDataRecord]) -> Result<FitActivity, FitError> {
    let file_id = build_file_id_info(data);

    // Collect (start_time, end_time) for every lap summary, preserving order.
    let raw_laps: Vec<LapTimeWindow> = data
        .iter()
        .filter(|r| r.kind() == MesgNum::Lap)
        .map(|r| (field_ts(r, "start_time"), record_timestamp(r)))
        .collect();

    // Collect (start_time, end_time, sport, sub_sport) for every session summary.
    let raw_sessions: Vec<SessionTimeWindow> = data
        .iter()
        .filter(|r| r.kind() == MesgNum::Session)
        .map(|r| {
            (
                field_ts(r, "start_time"),
                record_timestamp(r),
                field_string(r, "sport"),
                field_string(r, "sub_sport"),
            )
        })
        .collect();

    // Build FitLap structs with record indices.
    let fit_laps: Vec<FitLap> = raw_laps
        .iter()
        .enumerate()
        .map(|(i, (start, end))| {
            let record_indices = data
                .iter()
                .enumerate()
                .filter_map(|(idx, r)| {
                    if r.kind() != MesgNum::Record {
                        return None;
                    }
                    let ts = record_timestamp(r)?;
                    let after_start = start.is_none_or(|s| ts >= s);
                    let before_end = end.is_none_or(|e| ts <= e);
                    if after_start && before_end {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect();
            FitLap {
                index: i + 1,
                start_time: *start,
                end_time: *end,
                record_indices,
            }
        })
        .collect();

    // Assign laps to sessions.
    let sessions: Vec<FitSession> = raw_sessions
        .iter()
        .enumerate()
        .map(|(si, (s_start, s_end, sport, sub_sport))| {
            let laps: Vec<FitLap> = fit_laps
                .iter()
                .filter(|lap| {
                    // A lap belongs to this session if its start_time falls
                    // within the session's time window.
                    match (lap.start_time, s_start, s_end) {
                        (Some(lt), Some(ss), Some(se)) => lt >= *ss && lt <= *se,
                        (Some(lt), Some(ss), None) => lt >= *ss,
                        _ => true, // can't determine; include if we have no bounds
                    }
                })
                .enumerate()
                .map(|(li, lap)| FitLap {
                    index: li + 1, // re-number within session
                    ..lap.clone()
                })
                .collect();

            FitSession {
                index: si + 1,
                sport: sport.clone(),
                sub_sport: sub_sport.clone(),
                start_time: *s_start,
                end_time: *s_end,
                laps,
            }
        })
        .collect();

    let num_sessions = sessions.len();

    Ok(FitActivity {
        file_id,
        num_sessions,
        sessions,
    })
}

/// Return the session at 1-based index `n`.
pub fn session(activity: &FitActivity, n: usize) -> Result<&FitSession, FitError> {
    activity
        .sessions
        .get(n.saturating_sub(1))
        .ok_or(FitError::SessionOutOfRange(n))
}

/// Return all laps for session at 1-based index `n`.
pub fn laps_for_session(activity: &FitActivity, n: usize) -> Result<&[FitLap], FitError> {
    let s = session(activity, n)?;
    Ok(&s.laps)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn build_file_id_info(data: &[FitDataRecord]) -> FileIdInfo {
    let Some(rec) = data.iter().find(|r| r.kind() == MesgNum::FileId) else {
        return FileIdInfo {
            time_created: None,
            manufacturer: None,
            product: None,
            serial_number: None,
        };
    };
    FileIdInfo {
        time_created: field_ts(rec, "time_created"),
        manufacturer: field_string(rec, "manufacturer"),
        product: field_string(rec, "product_name"),
        serial_number: field_u32(rec, "serial_number"),
    }
}

/// Read a field whose value is a `Value::Timestamp`.
fn field_ts(record: &FitDataRecord, name: &str) -> Option<DateTime<Local>> {
    record.fields().iter().find(|f| f.name() == name).and_then(|f| {
        if let Value::Timestamp(t) = f.value() {
            Some(*t)
        } else {
            None
        }
    })
}

/// Read a field whose value can be rendered as a string.
fn field_string(record: &FitDataRecord, name: &str) -> Option<String> {
    record
        .fields()
        .iter()
        .find(|f| f.name() == name)
        .map(|f| f.value().to_string())
}

/// Read a field whose value is a `Value::UInt32` or similar unsigned integer.
fn field_u32(record: &FitDataRecord, name: &str) -> Option<u32> {
    record.fields().iter().find(|f| f.name() == name).and_then(|f| {
        match f.value() {
            Value::UInt32(v) => Some(*v),
            Value::UInt32z(v) => Some(*v),
            _ => None,
        }
    })
}
