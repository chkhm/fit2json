use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalArgs;
use crate::commands::{resolve_input, to_json, write_output};

#[derive(Args)]
pub struct InfoArgs {
    pub input: Option<PathBuf>,
    #[arg(long, default_value = "text")]
    pub format: super::types::OutputFormat,
    /// Print only the message-type count table.
    #[arg(long)]
    pub counts_only: bool,
}

pub fn run(global: &GlobalArgs, args: InfoArgs) -> Result<()> {
    let path = resolve_input(global, &args.input)?;
    let data = fitlib::parse::load_file(&path)?;
    let activity = fitlib::hierarchy::build_activity(&data)?;
    let json = to_json(global, &activity)?;
    write_output(global, &json)
}
