mod cli;
mod commands;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    if let Err(e) = commands::dispatch(cli) {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}
