
use std::collections::HashMap;
use std::io::Write;
use std::fs::File;
use std::env;
use rayon::prelude::*;
use chrono::{DateTime, TimeZone, Local};

use fitparser::{FitDataRecord, Value};
use fitparser::profile::field_types::MesgNum;

fn count_kinds(data: &Vec<FitDataRecord>) -> HashMap<String, usize> {
    data.into_par_iter()
        .fold(
            || HashMap::new(), 
            |mut acc, entry| {
                let kind = entry.kind().to_string();
                *acc.entry(kind).or_insert(0) += 1;
                acc
        })

        .reduce(
            || HashMap::new(),
            |mut m1, m2| {
                for (k, v) in m2 {
                    *m1.entry(k).or_insert(0) += v;
                }
                m1
            }
        )
}

fn select_kind(data: &Vec<FitDataRecord>, kind_name : MesgNum) -> Vec<FitDataRecord> {
    data.into_par_iter()
        .fold(
            || Vec::new(),
            |mut acc, entry| {
                let kind = entry.kind();

                if kind == kind_name {
                    acc.push(entry.clone());
                }
                acc
            }
        )

        .reduce(
            ||Vec::new(),
            |m1 : Vec<FitDataRecord>, m2 : Vec<FitDataRecord>| {
                m1.into_iter().chain(m2).collect()
            }
        )
}


fn select_kind_with_ts(data: &Vec<FitDataRecord>, kind_name : MesgNum, from_ts: DateTime<Local>, until_ts: DateTime<Local>) -> Vec<FitDataRecord> {
    data.into_par_iter()
        .fold(
            || Vec::new(),
            |mut acc, entry| {
                let kind = entry.kind();

                if kind == kind_name {
                    let datafield = entry.fields().iter().find(|f| f.name() == "timestamp");
                    let entry_from = match datafield {
                        Some(f) => match f.value() {
                            Value::Timestamp(ts) => *ts,
                            _ => return acc, // Skip if timestamp field is not a DateTime
                        },
                        None => return acc, // Skip if no timestamp field found
                    };
                    if entry_from >= from_ts && entry_from < until_ts {
                        acc.push(entry.clone());
                    }
                }
                acc
            }
        )

        .reduce(
            ||Vec::new(),
            |m1 : Vec<FitDataRecord>, m2 : Vec<FitDataRecord>| {
                m1.into_iter().chain(m2).collect()
            }
        )
}

fn kind_and_ts_to_str(record : &FitDataRecord) -> String {
    let kind = record.kind();
    let ts_field = record.fields().iter().find(|f| f.name() == "timestamp");
    let ts_str = match ts_field {
        Some(f) => match f.value() {
            Value::Timestamp(ts) => ts.to_string(),
            _ => "N/A".to_string(),
        },
        None => "N/A".to_string(),
    };
    format!("{}: {}", kind, ts_str)
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args : Vec<String> = env::args().collect();
    let fit_file = args.get(1).unwrap_or_else(|| {
        eprintln!("Usage: {} <fit_file>", args[0]);
        std::process::exit(1);
    });
    let default_out = "parsed_content.json".to_string();
    let out_file = args.get(2).unwrap_or(&default_out);

    // Step 0: Parse the FIT file and print the profile version
    println!("Parsing FIT files using Profile version: {}", fitparser::profile::VERSION);
    let mut fp = File::open(fit_file)?;
    let mut ofp = File::create(out_file)?;

    let data = fitparser::from_reader(&mut fp).unwrap();

    // Step 1: Count the entries
    let kind_counter = count_kinds(&data);
    
    // Step 2: Select FileId record and print the time_created field
    let fileidrecord = select_kind(&data, MesgNum::FileId);
    println!("FileId: {:#?}", fileidrecord);
    let fileid_ts_value = fileidrecord[0].fields().iter().find(|f| f.name() == "time_created").unwrap().value();
    let fielid_ts = match fileid_ts_value {
        Value::Timestamp(ts) => *ts,
        _ => panic!("Expected a DateTime value for time_created"),
    };
    println!("File created at: {}", fielid_ts);

    println!("\n\n------------------------------\n\n");

    // Step 3: Select Record records created within a specific time range
    let from_ts = chrono::Local.with_ymd_and_hms(2026, 04, 23, 09, 58, 43).unwrap(); // 2026-04-23T09:58:43-04:00
    let until_ts = from_ts + chrono::TimeDelta::seconds(5); // 5 seconds later
    let records = select_kind_with_ts(&data, MesgNum::Record, from_ts, until_ts);
    println!("Number of records in range: {}", records.len());
    for r in records {
        let s = kind_and_ts_to_str(&r);
        println!("{}", s);
    }
    

    // Step 4: print the parsed data into the output file in json format
    let s = serde_json::to_string_pretty(&data)?;
    writeln!(ofp, "{}", s)?;

    println!("\n\n------------------------------\n\n");

    // Step 5: print the counting results
    for (kind, count) in kind_counter {
        println!("{}: {}", kind, count);
    }

    Ok(())
}
