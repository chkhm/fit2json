pub mod compare;
pub mod dump;
pub mod events;
pub mod gps;
pub mod info;
pub mod laps;
pub mod select;
pub mod sessions;
pub mod stats;
pub mod types;
pub mod validate;

use anyhow::Result;
use serde::Serialize;

use crate::cli::{Cli, Command, GlobalArgs};

pub fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Dump(args)     => dump::run(&cli.global, args),
        Command::Info(args)     => info::run(&cli.global, args),
        Command::Types(args)    => types::run(&cli.global, args),
        Command::Select(args)   => select::run(&cli.global, args),
        Command::Stats(args)    => stats::run(&cli.global, args),
        Command::Gps(args)      => gps::run(&cli.global, args),
        Command::Events(args)   => events::run(&cli.global, args),
        Command::Sessions(args) => sessions::run(&cli.global, args),
        Command::Laps(args)     => laps::run(&cli.global, args),
        Command::Validate(args) => validate::run(&cli.global, args),
        Command::Compare(args)  => compare::run(&cli.global, args),
    }
}

// ---------------------------------------------------------------------------
// Shared helpers used by multiple subcommands
// ---------------------------------------------------------------------------

use std::io::{self, Write};
use std::path::PathBuf;

/// Resolve the input path: prefer the subcommand-level `input` positional
/// argument, then fall back to the global `--input` flag.
pub fn resolve_input(global: &GlobalArgs, sub_input: &Option<PathBuf>) -> Result<PathBuf> {
    sub_input
        .clone()
        .or_else(|| global.input.clone())
        .ok_or_else(|| anyhow::anyhow!("No input file specified. Pass a FIT file as an argument or use --input."))
}

/// Write `content` to the output destination: file if `--output` is set,
/// otherwise stdout.
pub fn write_output(global: &GlobalArgs, content: &str) -> Result<()> {
    if let Some(path) = &global.output {
        let mut f = std::fs::File::create(path)?;
        writeln!(f, "{}", content)?;
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        writeln!(handle, "{}", content)?;
    }
    Ok(())
}

/// Serialize `value` to JSON, choosing pretty or compact based on global flags.
/// Default: pretty when writing to a file, compact when writing to stdout.
pub fn to_json(global: &GlobalArgs, value: &impl Serialize) -> Result<String> {
    let use_pretty = global.pretty || (global.output.is_some() && !global.compact);
    if use_pretty {
        Ok(serde_json::to_string_pretty(value)?)
    } else {
        Ok(serde_json::to_string(value)?)
    }
}
