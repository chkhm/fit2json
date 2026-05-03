use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, ValueEnum};

use crate::cli::GlobalArgs;
use crate::commands::{resolve_input, to_json, write_output};

#[derive(ValueEnum, Clone, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Text,
}

#[derive(ValueEnum, Clone, Default)]
pub enum SortBy {
    /// Sort by record count, descending.
    #[default]
    Count,
    /// Sort alphabetically by message type name.
    Name,
}

#[derive(Args)]
pub struct TypesArgs {
    /// Input FIT file (positional alternative to the global --input flag).
    pub input: Option<PathBuf>,

    /// Output format.
    #[arg(long, default_value = "table")]
    pub format: OutputFormat,

    /// Sort order for the output.
    #[arg(long, default_value = "count")]
    pub sort: SortBy,
}

pub fn run(global: &GlobalArgs, args: TypesArgs) -> Result<()> {
    let path = resolve_input(global, &args.input)?;
    let data = fitlib::parse::load_file(&path)?;
    let counts = fitlib::filter::count_kinds(&data);

    // Sort according to --sort flag.
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    match args.sort {
        SortBy::Name  => pairs.sort_by(|a, b| a.0.cmp(&b.0)),
        SortBy::Count => pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0))),
    }

    match args.format {
        OutputFormat::Json => {
            let map: std::collections::HashMap<_, _> = pairs.into_iter().collect();
            let json = to_json(global, &map)?;
            write_output(global, &json)?;
        }
        OutputFormat::Table | OutputFormat::Text => {
            let width = pairs.iter().map(|(k, _)| k.len()).max().unwrap_or(10);
            let mut lines = Vec::new();
            for (kind, count) in &pairs {
                lines.push(format!("{:<width$}  {:>6}", kind, count, width = width));
            }
            write_output(global, &lines.join("\n"))?;
        }
    }

    Ok(())
}
