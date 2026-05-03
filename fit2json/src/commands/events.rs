use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use fitparser::profile::field_types::MesgNum;

use crate::cli::GlobalArgs;
use crate::commands::{resolve_input, to_json, write_output};

#[derive(Args)]
pub struct EventsArgs {
    pub input: Option<PathBuf>,
    /// Filter by event type string (e.g. `timer`, `lap`).  Comma-separated or
    /// flag can be repeated.
    #[arg(short = 't', long = "type")]
    pub event_type: Option<String>,
    #[arg(long, default_value = "table")]
    pub format: super::types::OutputFormat,
}

pub fn run(global: &GlobalArgs, args: EventsArgs) -> Result<()> {
    let path = resolve_input(global, &args.input)?;
    let data = fitlib::parse::load_file(&path)?;

    let events = fitlib::filter::select_kind(&data, MesgNum::Event);

    // Optional filter on the `event` field value.
    let type_filter: Vec<String> = args
        .event_type
        .as_deref()
        .map(|s| s.split(',').map(|p| p.trim().to_lowercase()).collect())
        .unwrap_or_default();

    let filtered: Vec<_> = events
        .into_iter()
        .filter(|r| {
            if type_filter.is_empty() {
                return true;
            }
            r.fields()
                .iter()
                .find(|f| f.name() == "event")
                .map(|f| type_filter.contains(&f.value().to_string().to_lowercase()))
                .unwrap_or(false)
        })
        .collect();

    let json = to_json(global, &filtered)?;
    write_output(global, &json)
}
