pub mod list;
pub mod survey;

use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

use crate::cli::{Cli, Command};

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub fn dispatch(cli: Cli) -> Result<()> {
    let Cli { command, output, pretty, compact } = cli;
    let out = OutputOpts { output, pretty, compact };
    match command {
        Command::Survey(args) => survey::run(&out, args),
        Command::List(args)   => list::run(&out, args),
    }
}

// ---------------------------------------------------------------------------
// Shared output helpers
// ---------------------------------------------------------------------------

/// Subset of global CLI flags that control where and how output is written.
/// Passed to subcommand `run` functions so they don't need the full `Cli`.
pub struct OutputOpts {
    pub output: Option<PathBuf>,
    pub pretty: bool,
    pub compact: bool,
}

impl OutputOpts {
    /// Write `content` to `--output <file>` or stdout.
    pub fn write(&self, content: &str) -> Result<()> {
        if let Some(path) = &self.output {
            let mut file = std::fs::File::create(path)?;
            file.write_all(content.as_bytes())?;
            writeln!(file)?;
        } else {
            println!("{content}");
        }
        Ok(())
    }

    /// Return `true` when JSON output should be pretty-printed.
    /// Default: pretty when writing to a file, compact to stdout.
    pub fn use_pretty(&self) -> bool {
        match (self.pretty, self.compact, &self.output) {
            (true, _, _)    => true,
            (_, true, _)    => false,
            (_, _, Some(_)) => true,
            _               => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared directory-scan helper
// ---------------------------------------------------------------------------

/// Collect all `*.fit` paths under `dir`.
///
/// When `recursive` is `false` only the immediate contents of `dir` are
/// scanned (`max_depth(1)`); when `true` the full subtree is walked.
pub fn collect_fit_paths(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let walker = if recursive {
        WalkDir::new(dir)
    } else {
        WalkDir::new(dir).max_depth(1)
    };

    let paths: Vec<PathBuf> = walker
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("fit"))
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    Ok(paths)
}
