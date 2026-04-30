
use std::collections::HashMap;
use std::io::Write;
use std::fs::File;
use std::env;
use rayon::prelude::*;

use fitparser::FitDataRecord;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args : Vec<String> = env::args().collect();
    let fit_file = args.get(1).unwrap_or_else(|| {
        eprintln!("Usage: {} <fit_file>", args[0]);
        std::process::exit(1);
    });
    let default_out = "parsed_content.json".to_string();
    let out_file = args.get(2).unwrap_or(&default_out);

    println!("Parsing FIT files using Profile version: {}", fitparser::profile::VERSION);
    let mut fp = File::open(fit_file)?;
    let mut ofp = File::create(out_file)?;

    let data = fitparser::from_reader(&mut fp).unwrap();

    // Step 1: Count the entries
    let kind_counter = count_kinds(&data);

    let fileidrecord = select_kind(&data, MesgNum::FileId);

    println!("FileId: {:#?}", fileidrecord);

    // Step 2: print the parsed data into the output file
    let s = serde_json::to_string_pretty(&data)?;
    writeln!(ofp, "{}", s)?;

    // Step 3: print the counting results
    for (kind, count) in kind_counter {
        println!("{}: {}", kind, count);
    }

    Ok(())
}
