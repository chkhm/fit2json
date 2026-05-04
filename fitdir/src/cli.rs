use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

// ---------------------------------------------------------------------------
// Top-level CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name    = "fitdir",
    about   = "Batch-process a directory of Garmin FIT files",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Write output to this file instead of stdout.
    #[arg(short = 'o', long, global = true)]
    pub output: Option<PathBuf>,

    /// Pretty-print JSON output (default when writing to a file).
    #[arg(long, global = true, conflicts_with = "compact")]
    pub pretty: bool,

    /// Compact single-line JSON output (default when writing to stdout).
    #[arg(long, global = true, conflicts_with = "pretty")]
    pub compact: bool,
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum Command {
    /// Report per-type file count, size, record count, and date range.
    Survey(SurveyArgs),

    /// List individual files filtered by type, sorted by date or other criteria.
    List(ListArgs),
}

// ---------------------------------------------------------------------------
// survey args
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct SurveyArgs {
    /// Directory to scan (default: current directory).
    #[arg(long, short = 'd', default_value = ".")]
    pub dir: PathBuf,

    /// Recurse into subdirectories.
    #[arg(long, short = 'r')]
    pub recursive: bool,

    /// Number of parallel worker threads (default: number of logical CPUs).
    #[arg(long, short = 'j')]
    pub jobs: Option<usize>,

    /// Output format.
    #[arg(long, value_enum, default_value = "table")]
    pub format: SurveyFormat,
}

#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum SurveyFormat {
    /// Human-readable aligned table (default).
    Table,
    /// Machine-readable JSON array of per-type statistics.
    Json,
}

// ---------------------------------------------------------------------------
// list args
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct ListArgs {
    /// Directory to scan (default: current directory).
    #[arg(long, short = 'd', default_value = ".")]
    pub dir: PathBuf,

    /// Recurse into subdirectories.
    #[arg(long, short = 'r')]
    pub recursive: bool,

    /// Number of parallel worker threads (default: number of logical CPUs).
    #[arg(long, short = 'j')]
    pub jobs: Option<usize>,

    /// Filter by file_id type. Repeatable: --type activity --type monitoring_b.
    /// Use the same type strings shown by `fitdir survey` or `fit2json filetype`.
    /// Omit to list all types.
    #[arg(long = "type", short = 't')]
    pub types: Vec<String>,

    /// Field to sort by.
    #[arg(long, value_enum, default_value = "date")]
    pub sort: SortField,

    /// Reverse sort order (default: ascending).
    #[arg(long)]
    pub desc: bool,

    /// Return at most N results (applied after sorting).
    #[arg(long, short = 'n')]
    pub limit: Option<usize>,

    /// Output format.
    #[arg(long, value_enum, default_value = "table")]
    pub format: ListFormat,
}

#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    /// Sort by time_created from the file_id record (default). Files without a
    /// date sort last.
    Date,
    /// Sort by file size in bytes.
    Size,
    /// Sort by total number of FIT messages in the file.
    Records,
    /// Sort alphabetically by filename (case-insensitive).
    Name,
}

#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum ListFormat {
    /// Human-readable aligned table (default).
    Table,
    /// Machine-readable JSON array, one object per file.
    Json,
}
