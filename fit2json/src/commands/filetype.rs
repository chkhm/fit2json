use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use serde::Serialize;

use crate::cli::GlobalArgs;
use crate::commands::{resolve_input, to_json, write_output};

#[derive(Args)]
pub struct FileTypeArgs {
    pub input: Option<PathBuf>,
    /// Output format: `text` (default, one-line string) or `json` (includes
    /// manufacturer and creation timestamp).
    #[arg(long, default_value = "text")]
    pub format: super::types::OutputFormat,
}

// ---------------------------------------------------------------------------
// JSON output type
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct FileTypeInfo {
    file_type: Option<String>,
    manufacturer: Option<String>,
    time_created: Option<String>,
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

pub fn run(global: &GlobalArgs, args: FileTypeArgs) -> Result<()> {
    let path = resolve_input(global, &args.input)?;
    let data = fitlib::parse::load_file(&path)?;

    use super::types::OutputFormat;
    match args.format {
        OutputFormat::Text | OutputFormat::Table => {
            let ft = fitlib::file_type(&data).unwrap_or_else(|| "unknown".to_string());
            write_output(global, &ft)
        }
        OutputFormat::Json => {
            // Build a richer payload from the file_id record.
            let activity = fitlib::hierarchy::build_activity(&data).ok();
            let file_id = activity.as_ref().map(|a| &a.file_id);
            let info = FileTypeInfo {
                file_type:    fitlib::file_type(&data),
                manufacturer: file_id.and_then(|id| id.manufacturer.clone()),
                time_created: file_id
                    .and_then(|id| id.time_created)
                    .map(|t| t.to_rfc3339()),
            };
            let json = to_json(global, &info)?;
            write_output(global, &json)
        }
    }
}
