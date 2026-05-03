use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use fitparser::FitDataRecord;

use crate::cli::GlobalArgs;
use crate::commands::{resolve_input, to_json, write_output};

#[derive(Args)]
pub struct DumpArgs {
    /// Input FIT file (positional alternative to the global --input flag).
    pub input: Option<PathBuf>,

    /// Write one JSON file per message type instead of a single output.
    ///
    /// Requires --output-dir.
    #[arg(long)]
    pub split: bool,

    /// Directory for split output files (requires --split).
    #[arg(long, requires = "split")]
    pub output_dir: Option<PathBuf>,

    /// Include raw integer values alongside decoded string values in output.
    #[arg(long)]
    pub include_raw: bool,
}

pub fn run(global: &GlobalArgs, args: DumpArgs) -> Result<()> {
    let path = resolve_input(global, &args.input)?;
    let data = fitlib::parse::load_file(&path)?;

    if args.split {
        dump_split(global, &args, &data)
    } else {
        let json = to_json(global, &data)?;
        write_output(global, &json)
    }
}

fn dump_split(
    global: &GlobalArgs,
    args: &DumpArgs,
    data: &[FitDataRecord],
) -> Result<()> {
    use std::collections::HashMap;
    use std::fs;

    let out_dir = args
        .output_dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    fs::create_dir_all(&out_dir)?;

    // Group records by kind name.
    let mut by_kind: HashMap<String, Vec<&FitDataRecord>> = HashMap::new();
    for record in data {
        by_kind
            .entry(record.kind().to_string())
            .or_default()
            .push(record);
    }

    for (kind, records) in &by_kind {
        let file_path = out_dir.join(format!("{}.json", kind));
        let json = if global.pretty || !global.compact {
            serde_json::to_string_pretty(records)?
        } else {
            serde_json::to_string(records)?
        };
        let mut f = std::fs::File::create(&file_path)?;
        use std::io::Write;
        writeln!(f, "{}", json)?;
        eprintln!("  wrote {}", file_path.display());
    }

    Ok(())
}
