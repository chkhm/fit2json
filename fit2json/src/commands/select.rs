use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use fitparser::profile::field_types::MesgNum;

use crate::cli::GlobalArgs;
use crate::commands::{resolve_input, to_json, write_output};

#[derive(Args)]
pub struct SelectArgs {
    pub input: Option<PathBuf>,
    /// Message type to select (e.g. `record`, `lap`, `session`, `event`).
    #[arg(short = 't', long = "type")]
    pub message_type: String,
    /// Include records at or after this time (ISO 8601 or HH:MM:SS relative).
    #[arg(long)]
    pub from: Option<String>,
    /// Include records before this time.
    #[arg(long)]
    pub until: Option<String>,
    /// Include records within this many seconds after --from.
    #[arg(short, long)]
    pub duration: Option<u64>,
    /// Restrict to session N (1-based, repeatable).
    #[arg(short, long)]
    pub session: Vec<usize>,
    /// Restrict to lap N within the selected session(s) (1-based, repeatable).
    #[arg(short, long)]
    pub lap: Vec<String>,
    /// Filter by field value predicate, e.g. `heart_rate>150` (repeatable, AND).
    #[arg(short, long = "field")]
    pub fields_filter: Vec<String>,
    /// Output only these fields per record (comma-separated projection).
    #[arg(long)]
    pub fields: Option<String>,
    /// Return at most N records.
    #[arg(long)]
    pub limit: Option<usize>,
    /// Print only the count of matching records.
    #[arg(long)]
    pub count: bool,
}

pub fn run(global: &GlobalArgs, args: SelectArgs) -> Result<()> {
    let path = resolve_input(global, &args.input)?;
    let data = fitlib::parse::load_file(&path)?;

    // Resolve the message type name to a MesgNum.
    // fitparser exposes MesgNum::from_str via the profile; we use a simple
    // string comparison against known names for now.
    let kind = parse_mesg_num(&args.message_type)
        .ok_or_else(|| anyhow::anyhow!("Unknown message type: '{}'", args.message_type))?;

    // Apply time-range filtering if --from / --until / --duration are given.
    let selected: Vec<&fitparser::FitDataRecord> = if args.from.is_some() || args.until.is_some() {
        // Determine the activity start for relative HH:MM:SS parsing.
        let activity_start = fitlib::filter::select_kind(&data, MesgNum::FileId)
            .first()
            .and_then(|r| fitlib::filter::record_timestamp(r));

        let from_ts = args
            .from
            .as_deref()
            .map(|s| fitlib::timestamp::parse_timestamp(s, activity_start))
            .transpose()?;

        let until_ts = if let Some(dur) = args.duration {
            from_ts.map(|f| f + chrono::TimeDelta::seconds(dur as i64))
        } else {
            args.until
                .as_deref()
                .map(|s| fitlib::timestamp::parse_timestamp(s, activity_start))
                .transpose()?
        };

        match (from_ts, until_ts) {
            (Some(f), Some(u)) => fitlib::filter::select_kind_with_ts(&data, kind, f, u),
            _ => fitlib::filter::select_kind(&data, kind),
        }
    } else {
        fitlib::filter::select_kind(&data, kind)
    };

    // Apply --limit.
    let selected: Vec<_> = match args.limit {
        Some(n) => selected.into_iter().take(n).collect(),
        None    => selected,
    };

    if args.count {
        write_output(global, &selected.len().to_string())?;
        return Ok(());
    }

    let json = to_json(global, &selected)?;
    write_output(global, &json)
}

/// Map a lowercase message type name to the corresponding `MesgNum` variant.
fn parse_mesg_num(s: &str) -> Option<MesgNum> {
    match s.to_lowercase().as_str() {
        "record"            => Some(MesgNum::Record),
        "lap"               => Some(MesgNum::Lap),
        "session"           => Some(MesgNum::Session),
        "event"             => Some(MesgNum::Event),
        "file_id"           => Some(MesgNum::FileId),
        "activity"          => Some(MesgNum::Activity),
        "device_info"       => Some(MesgNum::DeviceInfo),
        "developer_data_id" => Some(MesgNum::DeveloperDataId),
        "workout"           => Some(MesgNum::Workout),
        "workout_step"      => Some(MesgNum::WorkoutStep),
        "user_profile"      => Some(MesgNum::UserProfile),
        "zones_target"      => Some(MesgNum::ZonesTarget),
        "hr_zone"           => Some(MesgNum::HrZone),
        "power_zone"        => Some(MesgNum::PowerZone),
        "length"            => Some(MesgNum::Length),
        "monitoring"        => Some(MesgNum::Monitoring),
        _                   => None,
    }
}
