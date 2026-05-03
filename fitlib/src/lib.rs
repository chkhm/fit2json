pub mod error;
pub mod fields;
pub mod parse;
pub mod filter;
pub mod hierarchy;
pub mod timestamp;
pub mod stats;
pub mod gps;
pub mod validate;
pub mod survey;

pub use error::FitError;

/// Return the FIT file type string from the `file_id` record
/// (e.g. `"activity"`, `"workout"`, `"course"`).
///
/// Returns `None` if the file has no `file_id` record or the `type` field is
/// absent.  This is a fast O(n) scan that does not build the activity hierarchy,
/// making it suitable as a pre-filter in batch tools like `fitdir`.
pub fn file_type(data: &[fitparser::FitDataRecord]) -> Option<String> {
    use fitparser::profile::field_types::MesgNum;
    data.iter()
        .find(|r| r.kind() == MesgNum::FileId)
        .and_then(|r| r.fields().iter().find(|f| f.name() == "type"))
        .map(|f| f.value().to_string())
}
