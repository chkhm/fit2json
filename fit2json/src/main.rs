mod cli;
mod commands;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    commands::dispatch(cli)
}
