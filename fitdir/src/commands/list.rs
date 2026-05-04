use std::cmp::Ordering;

use anyhow::Result;
use chrono::{DateTime, Local};
use rayon::prelude::*;

use crate::cli::{ListArgs, ListFormat, SortField};
use crate::commands::{collect_fit_paths, OutputOpts};
use fitlib::survey::FileEntry;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(out: &OutputOpts, args: ListArgs) -> Result<()> {
    // Configure rayon thread pool if the user requested a specific count.
    if let Some(n) = args.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .unwrap_or(());
    }

    // Collect *.fit paths.
    let paths = collect_fit_paths(&args.dir, args.recursive)?;
    eprintln!("Scanning {} FIT file(s)…", paths.len());

    // Parse in parallel → FileEntry. Skip + warn on errors (REQ-DIR-006).
    let mut entries: Vec<FileEntry> = paths
        .par_iter()
        .filter_map(|path| {
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            match fitlib::parse::load_file(path) {
                Ok(data) => Some(fitlib::survey::to_file_entry(path.clone(), size, &data)),
                Err(e) => {
                    eprintln!("Warning: {}: {e}", path.display());
                    None
                }
            }
        })
        .collect();

    if entries.is_empty() {
        eprintln!("No FIT files could be parsed.");
        return Ok(());
    }

    // Filter by --type (case-insensitive, any match).
    if !args.types.is_empty() {
        let types_lc: Vec<String> = args.types.iter().map(|t| t.to_lowercase()).collect();
        entries.retain(|e| types_lc.contains(&e.file_type.to_lowercase()));
        if entries.is_empty() {
            eprintln!("No files matched the requested type(s).");
            return Ok(());
        }
    }

    // Sort.
    entries.sort_unstable_by(|a, b| {
        let ord = match args.sort {
            SortField::Date    => cmp_opt_date(a.time_created, b.time_created)
                                      .then_with(|| a.path.cmp(&b.path)),
            SortField::Size    => a.size_bytes.cmp(&b.size_bytes)
                                      .then_with(|| a.path.cmp(&b.path)),
            SortField::Records => a.record_count.cmp(&b.record_count)
                                      .then_with(|| a.path.cmp(&b.path)),
            SortField::Name    => cmp_filename(a, b)
                                      .then_with(|| cmp_opt_date(a.time_created, b.time_created)),
        };
        if args.desc { ord.reverse() } else { ord }
    });

    // Limit.
    if let Some(n) = args.limit {
        entries.truncate(n);
    }

    // Output.
    match args.format {
        ListFormat::Json  => output_json(out, &entries)?,
        ListFormat::Table => output_table(out, &entries)?,
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Sort helpers
// ---------------------------------------------------------------------------

/// Compare two optional timestamps.
/// `None` sorts after `Some` so files without a date appear at the end,
/// regardless of whether the sort is ascending or descending.
fn cmp_opt_date(a: Option<DateTime<Local>>, b: Option<DateTime<Local>>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None)    => Ordering::Less,
        (None, Some(_))    => Ordering::Greater,
        (None, None)       => Ordering::Equal,
    }
}

/// Compare two `FileEntry` values by their filename (last path component),
/// case-insensitively. Falls back to the full path for ties.
fn cmp_filename(a: &FileEntry, b: &FileEntry) -> Ordering {
    let name_a = a.path.file_name().map(|n| n.to_string_lossy().to_lowercase());
    let name_b = b.path.file_name().map(|n| n.to_string_lossy().to_lowercase());
    name_a.cmp(&name_b).then_with(|| a.path.cmp(&b.path))
}

// ---------------------------------------------------------------------------
// JSON output
// ---------------------------------------------------------------------------

fn output_json(out: &OutputOpts, entries: &[FileEntry]) -> Result<()> {
    let json = if out.use_pretty() {
        serde_json::to_string_pretty(entries)?
    } else {
        serde_json::to_string(entries)?
    };
    out.write(&json)
}

// ---------------------------------------------------------------------------
// Table output
// ---------------------------------------------------------------------------

fn fmt_bytes(b: u64) -> String {
    let b = b as f64;
    if b >= 1_048_576.0 {
        format!("{:.1}M", b / 1_048_576.0)
    } else {
        format!("{:.0}K", b / 1_024.0)
    }
}

fn output_table(out: &OutputOpts, entries: &[FileEntry]) -> Result<()> {
    // Derive column widths from data.
    let type_w = entries
        .iter()
        .map(|e| e.file_type.len())
        .max()
        .unwrap_or(4)
        .max(4); // "Type"

    let path_w = entries
        .iter()
        .map(|e| e.path.display().to_string().len())
        .max()
        .unwrap_or(4)
        .max(4); // "File"

    let mut lines: Vec<String> = Vec::new();

    // Header.
    lines.push(format!(
        "{:>5}  {:<10}  {:<type_w$}  {:>7}  {:>7}  {:<path_w$}",
        "#", "Date", "Type", "Size", "Records", "File",
        type_w = type_w, path_w = path_w,
    ));
    lines.push("─".repeat(5 + 2 + 10 + 2 + type_w + 2 + 7 + 2 + 7 + 2 + path_w));

    // Rows.
    for (i, e) in entries.iter().enumerate() {
        let date = e.time_created
            .map(|t| t.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "—".to_string());

        lines.push(format!(
            "{:>5}  {:<10}  {:<type_w$}  {:>7}  {:>7}  {}",
            i + 1,
            date,
            e.file_type,
            fmt_bytes(e.size_bytes),
            e.record_count,
            e.path.display(),
            type_w = type_w,
        ));
    }

    out.write(&lines.join("\n"))
}
