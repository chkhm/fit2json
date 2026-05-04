/// Directory-level FIT file survey — collect and aggregate per-file metadata.
///
/// Two usage patterns are supported:
///
/// **Aggregate survey** (`fitdir survey`):
/// ```text
/// let size   = std::fs::metadata(path)?.len();
/// let data   = fitlib::parse::load_file(path)?;
/// let sample = fitlib::survey::collect_sample(size, &data);
/// // … collect many samples …
/// let stats  = fitlib::survey::summarize(&samples);
/// ```
///
/// **Per-file listing** (`fitdir list`):
/// ```text
/// let size  = std::fs::metadata(path)?.len();
/// let data  = fitlib::parse::load_file(path)?;
/// let entry = fitlib::survey::to_file_entry(path.to_path_buf(), size, &data);
/// ```
use std::path::PathBuf;

use chrono::{DateTime, Local};
use fitparser::profile::field_types::MesgNum;
use fitparser::{FitDataRecord, Value};
use serde::Serialize;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Per-file data collected during a directory scan.
///
/// Cheap to construct and `Clone`; safe to send across threads.
#[derive(Debug, Clone)]
pub struct FileSurveySample {
    /// File type string from the `file_id` record (e.g. `"activity"`,
    /// `"monitoring_b"`), or `"unknown"` when the field is absent.
    pub file_type: String,
    /// Raw file size in bytes, from `std::fs::metadata`.
    pub size_bytes: u64,
    /// Total number of FIT messages in the file (all kinds combined).
    pub record_count: usize,
    /// `time_created` from the `file_id` record, or `None` if absent.
    pub time_created: Option<DateTime<Local>>,
}

/// Per-type aggregated statistics across all files of that type.
///
/// All size values are in bytes; display code converts to KB/MB as needed.
#[derive(Debug, Serialize)]
pub struct TypeStats {
    pub file_type: String,
    pub file_count: usize,

    // byte-size distribution
    pub size_min_bytes: u64,
    pub size_max_bytes: u64,
    pub size_mean_bytes: f64,
    pub size_median_bytes: f64,

    // message-count distribution
    pub records_min: usize,
    pub records_max: usize,
    pub records_mean: f64,
    pub records_median: f64,

    // date range (ISO 8601 date strings, `None` when no file had a `time_created`)
    pub oldest_date: Option<String>,
    pub newest_date: Option<String>,
}

/// Per-file entry produced by the listing pipeline (`fitdir list`).
///
/// Holds the same per-file statistics as [`FileSurveySample`] plus the file
/// path, making it suitable for sorted, filtered, per-row display.
///
/// # Future extensions
///
/// The struct is designed to accommodate sport metadata in a future phase:
/// uncomment the `sport` / `sub_sport` fields and populate them from the first
/// `session` record for `file_type == "activity"` files.
#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub file_type: String,
    pub size_bytes: u64,
    pub record_count: usize,
    pub time_created: Option<DateTime<Local>>,
    // Phase 2 — activity sport filtering:
    // pub sport: Option<String>,
    // pub sub_sport: Option<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a [`FileSurveySample`] from an already-parsed FIT file.
///
/// `size_bytes` must be supplied by the caller (e.g. via
/// `std::fs::metadata(path)?.len()`) so that this function stays pure and
/// testable without filesystem access.
pub fn collect_sample(size_bytes: u64, data: &[FitDataRecord]) -> FileSurveySample {
    let file_type = crate::file_type(data).unwrap_or_else(|| "unknown".to_string());
    let record_count = data.len();
    let time_created = extract_time_created(data);
    FileSurveySample { file_type, size_bytes, record_count, time_created }
}

/// Build a [`FileEntry`] from an already-parsed FIT file.
///
/// `path` is stored as-is (the caller controls whether it is absolute or
/// relative).  `size_bytes` must be supplied by the caller via
/// `std::fs::metadata(path)?.len()`.
pub fn to_file_entry(path: PathBuf, size_bytes: u64, data: &[FitDataRecord]) -> FileEntry {
    FileEntry {
        file_type:    crate::file_type(data).unwrap_or_else(|| "unknown".to_string()),
        record_count: data.len(),
        time_created: extract_time_created(data),
        path,
        size_bytes,
    }
}

/// Group [`FileSurveySample`]s by file type and compute per-group statistics.
///
/// Returns one [`TypeStats`] per distinct file type, sorted by `file_count`
/// descending (most common type first).
pub fn summarize(samples: &[FileSurveySample]) -> Vec<TypeStats> {
    // Group indices by file_type.
    let mut groups: std::collections::HashMap<&str, Vec<usize>> =
        std::collections::HashMap::new();
    for (i, s) in samples.iter().enumerate() {
        groups.entry(s.file_type.as_str()).or_default().push(i);
    }

    let mut stats: Vec<TypeStats> = groups
        .into_iter()
        .map(|(file_type, indices)| {
            let group: Vec<&FileSurveySample> =
                indices.iter().map(|&i| &samples[i]).collect();

            // Collect and sort size + record vectors for median computation.
            let mut sizes: Vec<u64> = group.iter().map(|s| s.size_bytes).collect();
            let mut records: Vec<usize> = group.iter().map(|s| s.record_count).collect();
            sizes.sort_unstable();
            records.sort_unstable();

            let n = group.len();
            let size_sum: u64 = sizes.iter().sum();
            let rec_sum: usize = records.iter().sum();

            // Date range — compare as DateTime values, format as YYYY-MM-DD.
            let mut oldest: Option<DateTime<Local>> = None;
            let mut newest: Option<DateTime<Local>> = None;
            for s in &group {
                if let Some(t) = s.time_created {
                    oldest = Some(oldest.map_or(t, |o: DateTime<Local>| o.min(t)));
                    newest = Some(newest.map_or(t, |p: DateTime<Local>| p.max(t)));
                }
            }

            TypeStats {
                file_type: file_type.to_string(),
                file_count: n,

                size_min_bytes: sizes[0],
                size_max_bytes: sizes[n - 1],
                size_mean_bytes: size_sum as f64 / n as f64,
                size_median_bytes: median_u64(&sizes),

                records_min: records[0],
                records_max: records[n - 1],
                records_mean: rec_sum as f64 / n as f64,
                records_median: median_usize(&records),

                oldest_date: oldest.map(|t| t.format("%Y-%m-%d").to_string()),
                newest_date: newest.map(|t| t.format("%Y-%m-%d").to_string()),
            }
        })
        .collect();

    // Most common file type first.
    stats.sort_unstable_by(|a, b| b.file_count.cmp(&a.file_count).then(a.file_type.cmp(&b.file_type)));
    stats
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Extract the `time_created` field from a `file_id` record.
fn extract_time_created(data: &[FitDataRecord]) -> Option<DateTime<Local>> {
    let file_id = data.iter().find(|r| r.kind() == MesgNum::FileId)?;
    file_id.fields().iter().find(|f| f.name() == "time_created").and_then(|f| {
        if let Value::Timestamp(t) = f.value() { Some(*t) } else { None }
    })
}

fn median_u64(sorted: &[u64]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        sorted[n / 2] as f64
    } else {
        (sorted[n / 2 - 1] as f64 + sorted[n / 2] as f64) / 2.0
    }
}

fn median_usize(sorted: &[usize]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        sorted[n / 2] as f64
    } else {
        (sorted[n / 2 - 1] as f64 + sorted[n / 2] as f64) / 2.0
    }
}
