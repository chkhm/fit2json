/// All errors that can originate inside `fitlib`.
///
/// Binary crates wrap these with `anyhow` for ergonomic reporting.
/// Library callers can match on the variants for structured handling.
#[derive(Debug, thiserror::Error)]
pub enum FitError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("FIT parse error: {0}")]
    Parse(#[from] fitparser::Error),

    #[error("required message missing: {0}")]
    MissingMessage(String),

    #[error("timestamp field absent or wrong type on {kind} record")]
    TimestampMissing { kind: String },

    #[error("file integrity check failed: {0}")]
    IntegrityFailure(String),

    #[error("GPS data unavailable")]
    NoGpsData,

    #[error("session index {0} out of range")]
    SessionOutOfRange(usize),

    #[error("lap index {0} out of range in session {1}")]
    LapOutOfRange(usize, usize),

    #[error("the `{cmd}` command only applies to activity files (this file has type `{file_type}`)")]
    NotAnActivityFile { cmd: String, file_type: String },
}
