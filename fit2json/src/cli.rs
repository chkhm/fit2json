use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::commands::{
    compare::CompareArgs, dump::DumpArgs, events::EventsArgs, filetype::FileTypeArgs,
    gps::GpsArgs, info::InfoArgs, laps::LapsArgs, select::SelectArgs, sessions::SessionsArgs,
    stats::StatsArgs, types::TypesArgs, validate::ValidateArgs,
};

/// Command-line tool for querying and extracting data from Garmin FIT files.
#[derive(Parser)]
#[command(name = "fit2json", version, about, long_about = None)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Command,
}

/// Options that apply to every subcommand.
#[derive(Args, Clone)]
pub struct GlobalArgs {
    /// Input FIT file (can also be passed as a positional argument to each
    /// subcommand).
    #[arg(short = 'i', long, global = true)]
    pub input: Option<PathBuf>,

    /// Write output to this file instead of stdout.
    #[arg(short = 'o', long, global = true)]
    pub output: Option<PathBuf>,

    /// Pretty-print JSON output (default when writing to a file).
    #[arg(long, global = true, conflicts_with = "compact")]
    pub pretty: bool,

    /// Compact single-line JSON (default when writing to stdout).
    #[arg(long, global = true, conflicts_with = "pretty")]
    pub compact: bool,

    /// Display all timestamps in UTC.
    #[arg(long, global = true, conflicts_with = "timezone")]
    pub utc: bool,

    /// Display timestamps in this IANA timezone (e.g. `Europe/Berlin`).
    #[arg(long, global = true)]
    pub timezone: Option<String>,

    /// Suppress fields with unknown names or Connect IQ developer-defined fields.
    #[arg(long = "no-unknown", global = true)]
    pub no_unknown: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Extract all records from the FIT file to JSON (full dump).
    Dump(DumpArgs),
    /// Print a human-readable summary of the file.
    Info(InfoArgs),
    /// List all FIT message types present in the file with their record counts.
    Types(TypesArgs),
    /// Query and filter records by type, time range, session, lap, and field values.
    Select(SelectArgs),
    /// Compute aggregated statistics (min/max/mean) across records.
    Stats(StatsArgs),
    /// Extract the GPS track as GeoJSON or GPX.
    Gps(GpsArgs),
    /// Show the event log (timer start/stop, lap triggers, etc.).
    Events(EventsArgs),
    /// List all sessions in the file (useful for multi-sport activities).
    Sessions(SessionsArgs),
    /// Show a per-lap summary table.
    Laps(LapsArgs),
    /// Check the file for structural and logical integrity issues.
    Validate(ValidateArgs),
    /// Compare statistics between two FIT files.
    Compare(CompareArgs),
    /// Report the FIT file type from the file_id record (activity, workout, course, …).
    #[command(name = "filetype")]
    FileType(FileTypeArgs),
}
