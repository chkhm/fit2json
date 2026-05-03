use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalArgs;
use crate::commands::{require_activity_file, resolve_input, to_json, write_output};

#[derive(Args)]
pub struct LapsArgs {
    pub input: Option<PathBuf>,
    /// Restrict to session N (1-based); default: all sessions.
    #[arg(short, long)]
    pub session: Option<usize>,
    #[arg(long, default_value = "table")]
    pub format: super::types::OutputFormat,
}

pub fn run(global: &GlobalArgs, args: LapsArgs) -> Result<()> {
    let path = resolve_input(global, &args.input)?;
    let data = fitlib::parse::load_file(&path)?;
    require_activity_file(&data, "laps")?;
    let activity = fitlib::hierarchy::build_activity(&data)?;

    let laps: Vec<_> = match args.session {
        Some(n) => fitlib::hierarchy::laps_for_session(&activity, n)?.to_vec(),
        None    => activity.sessions.iter().flat_map(|s| s.laps.clone()).collect(),
    };

    let json = to_json(global, &laps)?;
    write_output(global, &json)
}
