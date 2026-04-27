
use std::collections::HashMap;
use std::io::Write;
use std::fs::File;
use std::env;
// use std::io::prelude::*;
use rayon::prelude::*;
use fitparser;

fn count_kinds(data: &Vec<fitparser::FitDataRecord>) -> HashMap<String, usize> {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args : Vec<String> = env::args().collect();
    let fit_file = args.get(1).unwrap_or_else(|| {
        eprintln!("Usage: {} <fit_file>", args[0]);
        std::process::exit(1);
    });
    let default_out = "parsed_content.txt".to_string();
    let out_file = args.get(2).unwrap_or(&default_out);

    println!("Parsing FIT files using Profile version: {}", fitparser::profile::VERSION);
    let mut fp = File::open(fit_file)?;
    let mut ofp = File::create(out_file)?;

    let data = fitparser::from_reader(&mut fp).unwrap();

    // Step 1: Count the entries
    let kind_counter = count_kinds(&data);

    // Step 2: print the parsed data into the output file
    for entry in data {
        writeln!(ofp, "{:#?}", entry)?;
    }

    // Outcommented counting when doing it all serially, which is much slower than the parallel version above
    // let mut kind_counter = HashMap::new();
    // fitparser::from_reader(&mut fp)?.iter_mut().for_each(|data| {
    //     // print the data in FIT file
    //     writeln!(ofp, "{:#?}", data).unwrap();
    //     let kind = data.kind().to_string();
    //     *kind_counter.entry(kind).or_insert(0) += 1;
    // });
    //for data in fitparser::from_reader(&mut fp)? {
    //    // print the data in FIT file
    //    writeln!(ofp, "{:#?}", data)?;
    //    let kind = data.kind().to_string();
    //    *kind_counter.entry(kind).or_insert(0) += 1;
    //}

    // Step 3: print the counting results
    for (kind, count) in kind_counter {
        println!("{}: {}", kind, count);
    }

    Ok(())
}
