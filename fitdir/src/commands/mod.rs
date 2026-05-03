pub mod survey;

use anyhow::Result;

use crate::cli::{Cli, Command};

pub fn dispatch(cli: Cli) -> Result<()> {
    let Cli { command, output, pretty, compact } = cli;
    let out = OutputOpts { output, pretty, compact };
    match command {
        Command::Survey(args) => survey::run(&out, args),
    }
}

/// Subset of global CLI flags that control where and how output is written.
/// Passed to subcommand `run` functions so they don't need the full `Cli`.
pub struct OutputOpts {
    pub output: Option<std::path::PathBuf>,
    pub pretty: bool,
    pub compact: bool,
}
