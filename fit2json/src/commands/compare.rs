use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use fitparser::FitDataRecord;

use crate::cli::GlobalArgs;
use crate::commands::{to_json, write_output};

#[derive(Args)]
pub struct CompareArgs {
    /// First FIT file.
    pub file1: PathBuf,
    /// Second FIT file.
    pub file2: PathBuf,
    /// Aggregation granularity for comparison.
    #[arg(long, default_value = "activity")]
    pub by: super::stats::AggBy,
    /// Restrict comparison to these fields (comma-separated).
    #[arg(long)]
    pub fields: Option<String>,
}

pub fn run(global: &GlobalArgs, args: CompareArgs) -> Result<()> {
    let data1 = fitlib::parse::load_file(&args.file1)?;
    let data2 = fitlib::parse::load_file(&args.file2)?;

    let field_filter: Vec<&str> = args
        .fields
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();

    let all1: Vec<&FitDataRecord> = data1.iter().collect();
    let all2: Vec<&FitDataRecord> = data2.iter().collect();

    let stats1 = fitlib::stats::aggregate(&all1, &field_filter);
    let stats2 = fitlib::stats::aggregate(&all2, &field_filter);

    let result = serde_json::json!({
        "file1": args.file1.display().to_string(),
        "file2": args.file2.display().to_string(),
        "stats1": stats1,
        "stats2": stats2,
    });

    let json = to_json(global, &result)?;
    write_output(global, &json)
}
