use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalArgs;
use crate::commands::{resolve_input, to_json, write_output};

#[derive(Args)]
pub struct ValidateArgs {
    pub input: Option<PathBuf>,
    #[arg(long, default_value = "text")]
    pub format: super::types::OutputFormat,
}

pub fn run(global: &GlobalArgs, args: ValidateArgs) -> Result<()> {
    let path = resolve_input(global, &args.input)?;
    let data = fitlib::parse::load_file(&path)?;
    let report = fitlib::validate::validate(&data);

    match args.format {
        super::types::OutputFormat::Json => {
            let json = to_json(global, &report)?;
            write_output(global, &json)?;
        }
        _ => {
            let status = if report.passed { "PASSED" } else { "FAILED" };
            let mut lines = vec![format!("Validation: {}", status)];
            for issue in &report.issues {
                lines.push(format!("  [{:?}] {}", issue.severity, issue.message));
            }
            write_output(global, &lines.join("\n"))?;
        }
    }
    Ok(())
}
