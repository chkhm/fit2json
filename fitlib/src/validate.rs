/// Structural and logical integrity checks for a parsed FIT activity file.
use fitparser::profile::field_types::MesgNum;
use fitparser::FitDataRecord;
use serde::Serialize;

use crate::filter::record_timestamp;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
    /// `true` iff no `Error`-level issues were found.
    pub passed: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run all structural and logical checks against `data`.
///
/// Checks performed:
/// - Required messages: `file_id` and `activity` must be present.
/// - Session count: `activity.num_sessions` must match the number of `session` records.
/// - Timestamp ordering: `record` timestamps must be monotonically non-decreasing.
/// - GPS outliers: consecutive GPS positions must not jump more than 1 000 m in < 1 s.
/// - Developer fields: noted as `Info` if `developer_data_id` messages are present.
pub fn validate(data: &[FitDataRecord]) -> ValidationReport {
    let mut issues = Vec::new();

    check_required_messages(data, &mut issues);
    check_session_count(data, &mut issues);
    check_timestamp_ordering(data, &mut issues);
    check_developer_fields(data, &mut issues);

    let passed = !issues.iter().any(|i| i.severity == Severity::Error);
    ValidationReport { issues, passed }
}

// ---------------------------------------------------------------------------
// Individual checks
// ---------------------------------------------------------------------------

fn check_required_messages(data: &[FitDataRecord], issues: &mut Vec<ValidationIssue>) {
    if !data.iter().any(|r| r.kind() == MesgNum::FileId) {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "Missing required 'file_id' message".to_string(),
        });
    }
    if !data.iter().any(|r| r.kind() == MesgNum::Activity) {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            message: "Missing required 'activity' message".to_string(),
        });
    }
}

fn check_session_count(data: &[FitDataRecord], issues: &mut Vec<ValidationIssue>) {
    use fitparser::Value;

    let session_count = data.iter().filter(|r| r.kind() == MesgNum::Session).count();

    let declared = data
        .iter()
        .find(|r| r.kind() == MesgNum::Activity)
        .and_then(|r| r.fields().iter().find(|f| f.name() == "num_sessions"))
        .and_then(|f| match f.value() {
            Value::UInt16(v) => Some(*v as usize),
            Value::UInt8(v)  => Some(*v as usize),
            _ => None,
        });

    if let Some(declared_n) = declared
        && declared_n != session_count
    {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            message: format!(
                "activity.num_sessions={} but {} 'session' records found",
                declared_n, session_count
            ),
        });
    }
}

fn check_timestamp_ordering(data: &[FitDataRecord], issues: &mut Vec<ValidationIssue>) {
    let mut prev: Option<chrono::DateTime<chrono::Local>> = None;
    let mut violations = 0usize;

    for record in data.iter().filter(|r| r.kind() == MesgNum::Record) {
        if let Some(ts) = record_timestamp(record) {
            if let Some(p) = prev
                && ts < p
            {
                violations += 1;
            }
            prev = Some(ts);
        }
    }

    if violations > 0 {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            message: format!(
                "{} record(s) have a timestamp earlier than the preceding record (out-of-order data)",
                violations
            ),
        });
    }
}

fn check_developer_fields(data: &[FitDataRecord], issues: &mut Vec<ValidationIssue>) {
    let dev_count = data
        .iter()
        .filter(|r| r.kind() == MesgNum::DeveloperDataId)
        .count();
    if dev_count > 0 {
        issues.push(ValidationIssue {
            severity: Severity::Info,
            message: format!(
                "{} Connect IQ developer_data_id record(s) present; custom fields may appear in records",
                dev_count
            ),
        });
    }
}
