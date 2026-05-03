
use std::collections::HashMap;
use std::io::Write;
use std::fs::File;
use std::env;

use chrono::{DateTime, TimeZone, Local};

use fitparser::{FitDataRecord, Value};
use fitparser::profile::field_types::MesgNum;

/// Count occurrences of each message kind in `data`.
/// Sequential scan — FIT files contain 3 000–8 000 records, well below the
/// break-even point for rayon parallelism (~100 000 items with cheap per-item work).
fn count_kinds(data: &[FitDataRecord]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for entry in data {
        *counts.entry(entry.kind().to_string()).or_insert(0) += 1;
    }
    counts
}

/// Return references to every record whose kind matches `kind_name`.
/// Callers that need owned values can clone the returned references.
fn select_kind(data: &[FitDataRecord], kind_name: MesgNum) -> Vec<&FitDataRecord> {
    data.iter().filter(|e| e.kind() == kind_name).collect()
}

/// Return references to records of `kind_name` whose timestamp falls in
/// `[from_ts, until_ts)`.  Records without a timestamp field are skipped.
fn select_kind_with_ts(
    data: &[FitDataRecord],
    kind_name: MesgNum,
    from_ts: DateTime<Local>,
    until_ts: DateTime<Local>,
) -> Vec<&FitDataRecord> {
    data.iter()
        .filter(|e| {
            if e.kind() != kind_name {
                return false;
            }
            let ts = e
                .fields()
                .iter()
                .find(|f| f.name() == "timestamp")
                .and_then(|f| {
                    if let Value::Timestamp(t) = f.value() {
                        Some(*t)
                    } else {
                        None
                    }
                });
            matches!(ts, Some(t) if t >= from_ts && t < until_ts)
        })
        .collect()
}

fn kind_and_ts_to_str(record: &FitDataRecord) -> String {
    let kind = record.kind();
    let ts_str = record
        .fields()
        .iter()
        .find(|f| f.name() == "timestamp")
        .and_then(|f| {
            if let Value::Timestamp(ts) = f.value() {
                Some(ts.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "N/A".to_string());
    format!("{}: {}", kind, ts_str)
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let fit_file = args.get(1).unwrap_or_else(|| {
        eprintln!("Usage: {} <fit_file> [output_file]", args[0]);
        std::process::exit(1);
    });
    let default_out = "parsed_content.json".to_string();
    let out_file = args.get(2).unwrap_or(&default_out);

    // Step 0: Parse the FIT file.
    println!("Parsing FIT file using profile version: {}", fitparser::profile::VERSION);
    let mut fp = File::open(fit_file)?;
    let mut ofp = File::create(out_file)?;

    let data = fitparser::from_reader(&mut fp)?;

    // Step 1: Count message kinds.
    let kind_counter = count_kinds(&data);

    // Step 2: Extract the FileId record and read time_created.
    let fileid_records = select_kind(&data, MesgNum::FileId);
    let fileid = fileid_records
        .first()
        .ok_or("FIT file contains no FileId record")?;

    let time_created = fileid
        .fields()
        .iter()
        .find(|f| f.name() == "time_created")
        .ok_or("FileId record has no time_created field")?
        .value();

    let file_ts = match time_created {
        Value::Timestamp(ts) => *ts,
        _ => return Err("time_created is not a timestamp".into()),
    };
    println!("File created at: {}", file_ts);

    println!("\n------------------------------\n");

    // Step 3: Select Record messages within a specific time range.
    // TODO: replace hard-coded timestamps with CLI arguments once the
    //       argument parser is in place.
    let from_ts = chrono::Local
        .with_ymd_and_hms(2026, 4, 23, 9, 58, 43)
        .single()
        .ok_or("Invalid from timestamp")?;
    let until_ts = from_ts + chrono::TimeDelta::seconds(5);

    let records = select_kind_with_ts(&data, MesgNum::Record, from_ts, until_ts);
    println!("Records in range [{}, {}): {}", from_ts, until_ts, records.len());
    for r in &records {
        println!("{}", kind_and_ts_to_str(r));
    }

    // Step 4: Write the full parsed dataset to JSON.
    let json = serde_json::to_string_pretty(&data)?;
    writeln!(ofp, "{}", json)?;

    println!("\n------------------------------\n");

    // Step 5: Print message-kind counts (sorted for reproducible output).
    let mut sorted_counts: Vec<_> = kind_counter.iter().collect();
    sorted_counts.sort_by_key(|(k, _)| k.as_str());
    for (kind, count) in sorted_counts {
        println!("{}: {}", kind, count);
    }

    Ok(())
}
