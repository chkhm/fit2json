use std::fs::File;
use std::io::Read;
use std::path::Path;

use fitparser::FitDataRecord;

use crate::FitError;

/// Open a FIT file at `path` and parse it into a flat, chronological record list.
pub fn load_file(path: &Path) -> Result<Vec<FitDataRecord>, FitError> {
    let mut fp = File::open(path)?;
    load_reader(&mut fp)
}

/// Parse FIT data from an already-open [`Read`] source.
///
/// Useful when the caller has the bytes in memory (e.g. extracted from a ZIP)
/// and wants to avoid writing them to disk first.
pub fn load_reader<R: Read>(reader: &mut R) -> Result<Vec<FitDataRecord>, FitError> {
    let records = fitparser::from_reader(reader)?;
    Ok(records)
}
