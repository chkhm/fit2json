use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, ValueEnum};
use fitparser::FitDataRecord;

use crate::cli::GlobalArgs;
use crate::commands::{require_activity_file, resolve_input, to_json, write_output};

#[derive(ValueEnum, Clone, Default)]
pub enum AggBy {
    #[default]
    Activity,
    Session,
    Lap,
}

#[derive(Args)]
pub struct StatsArgs {
    pub input: Option<PathBuf>,
    /// Aggregation granularity.
    #[arg(long, default_value = "activity")]
    pub by: AggBy,
    /// Restrict to session N (1-based).
    #[arg(short, long)]
    pub session: Option<usize>,
    /// Restrict aggregation to these fields (comma-separated).
    #[arg(long)]
    pub fields: Option<String>,
    #[arg(long, default_value = "table")]
    pub format: super::types::OutputFormat,
}

pub fn run(global: &GlobalArgs, args: StatsArgs) -> Result<()> {
    let path = resolve_input(global, &args.input)?;
    let data = fitlib::parse::load_file(&path)?;

    // Guard hierarchy-dependent modes before the expensive build_activity call.
    if matches!(args.by, AggBy::Session | AggBy::Lap) {
        require_activity_file(&data, "stats --by session/lap")?;
    }

    let activity = fitlib::hierarchy::build_activity(&data)?;

    let field_filter: Vec<&str> = args
        .fields
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();

    let result = match args.by {
        AggBy::Activity => {
            let all_refs: Vec<&FitDataRecord> = data.iter().collect();
            vec![fitlib::stats::aggregate(&all_refs, &field_filter)]
        }
        AggBy::Session  => fitlib::stats::per_session(&data, &activity, &field_filter),
        AggBy::Lap      => {
            let session_n = args.session.unwrap_or(1);
            let session = fitlib::hierarchy::session(&activity, session_n)?;
            fitlib::stats::per_lap(&data, session, &field_filter)
        }
    };

    let json = to_json(global, &result)?;
    write_output(global, &json)
}
