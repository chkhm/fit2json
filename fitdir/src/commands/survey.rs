use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Result;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::cli::{SurveyArgs, SurveyFormat};
use crate::commands::OutputOpts;
use fitlib::survey::{FileSurveySample, TypeStats};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(out: &OutputOpts, args: SurveyArgs) -> Result<()> {
    // Configure rayon thread pool if the user requested a specific count.
    if let Some(n) = args.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .unwrap_or(()); // ignore error if pool already initialised
    }

    // Collect *.fit paths.
    let paths = collect_fit_paths(&args.dir, args.recursive)?;
    eprintln!("Scanning {} FIT file(s)…", paths.len());

    // Parse in parallel; skip and warn on errors (REQ-DIR-006).
    let samples: Vec<FileSurveySample> = paths
        .par_iter()
        .filter_map(|path| {
            let size = std::fs::metadata(path)
                .map(|m| m.len())
                .unwrap_or(0);
            match fitlib::parse::load_file(path) {
                Ok(data) => Some(fitlib::survey::collect_sample(size, &data)),
                Err(e) => {
                    eprintln!("Warning: {}: {e}", path.display());
                    None
                }
            }
        })
        .collect();

    if samples.is_empty() {
        eprintln!("No FIT files could be parsed.");
        return Ok(());
    }

    let stats = fitlib::survey::summarize(&samples);

    match args.format {
        SurveyFormat::Json  => output_json(out, &stats)?,
        SurveyFormat::Table => output_table(out, &stats)?,
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Directory walker
// ---------------------------------------------------------------------------

fn collect_fit_paths(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
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

// ---------------------------------------------------------------------------
// JSON output
// ---------------------------------------------------------------------------

fn output_json(out: &OutputOpts, stats: &[TypeStats]) -> Result<()> {
    let pretty = match (out.pretty, out.compact, &out.output) {
        (true, _, _)       => true,
        (_, true, _)       => false,
        (_, _, Some(_))    => true,   // file → pretty by default
        _                  => false,  // stdout → compact by default
    };

    let json = if pretty {
        serde_json::to_string_pretty(stats)?
    } else {
        serde_json::to_string(stats)?
    };

    write_output(out, &json)
}

fn write_output(out: &OutputOpts, content: &str) -> Result<()> {
    if let Some(path) = &out.output {
        let mut file = std::fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        writeln!(file)?;
    } else {
        println!("{content}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Table output
// ---------------------------------------------------------------------------

/// Format bytes as a human-readable string with KB/MB suffix.
fn fmt_bytes(b: f64) -> String {
    if b >= 1_048_576.0 {
        format!("{:.1}M", b / 1_048_576.0)
    } else {
        format!("{:.0}K", b / 1_024.0)
    }
}

fn output_table(out: &OutputOpts, stats: &[TypeStats]) -> Result<()> {
    // Column widths — derive from data so the table stays compact.
    let type_w = stats
        .iter()
        .map(|s| s.file_type.len())
        .max()
        .unwrap_or(9)
        .max(9); // "File Type"

    // Build lines.
    let mut lines: Vec<String> = Vec::new();

    // Header
    lines.push(format!(
        "{:<type_w$}  {:>6}  {:>28}  {:>28}  {}",
        "File Type",
        "Files",
        "Size  min / avg / median / max",
        "Records  min / avg / median / max",
        "Date range",
        type_w = type_w,
    ));
    lines.push("─".repeat(lines[0].len() + 10));

    for s in stats {
        let size_col = format!(
            "{} / {} / {} / {}",
            fmt_bytes(s.size_min_bytes as f64),
            fmt_bytes(s.size_mean_bytes),
            fmt_bytes(s.size_median_bytes),
            fmt_bytes(s.size_max_bytes as f64),
        );
        let rec_col = format!(
            "{} / {:.0} / {:.0} / {}",
            s.records_min,
            s.records_mean,
            s.records_median,
            s.records_max,
        );
        let date_col = match (&s.oldest_date, &s.newest_date) {
            (Some(a), Some(b)) if a == b => a.clone(),
            (Some(a), Some(b))           => format!("{a} – {b}"),
            _                            => "—".to_string(),
        };

        lines.push(format!(
            "{:<type_w$}  {:>6}  {:>28}  {:>28}  {}",
            s.file_type,
            s.file_count,
            size_col,
            rec_col,
            date_col,
            type_w = type_w,
        ));
    }

    let table = lines.join("\n");
    write_output(out, &table)
}
