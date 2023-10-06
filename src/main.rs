use std::path::Path;

use anyhow::{bail, Result};
use clap::{arg, Parser};
use crate::ccan::{Analysis, Options};
use crate::output::{create_path, mkdir, write_arr, write_matrix};

mod git;
mod ccan;
mod cochanges;
mod output;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, required = true)]
    repository: String,
    #[arg(short, long, required = true)]
    branch: String,
    #[arg(short, long, default_value = "5")]
    changes_min: u32,
    #[arg(short, long, default_value = "5")]
    freq_min: u32,
    #[arg(short, long, default_value = "0")]
    days_binning: u32,
    #[arg(short, long, required = true)]
    output_dir: String,
}

fn run(args: Args) -> Result<()> {
    let basename = Path::new(args.repository.as_str())
        .file_name().map_or_else(||"repo", |p| p.to_str().unwrap());
    let output_dir = create_path(&[args.output_dir.as_str(), "ccan-output", basename]);
    let cc_freqs_file =  &create_path(&[output_dir.as_str(), "cc_freqs.csv"]);
    let cc_probs_file = &create_path(&[output_dir.as_str(), "cc_probs.csv"]);
    let cc_files_file = &create_path(&[output_dir.as_str(), "cc_files.csv"]);

    println!("Started analysing {}", args.repository.as_str());
    let mut analysis = Analysis::new(Options {
        repository: args.repository,
        branch: args.branch,
        days_binning: args.days_binning,
        freq_min: args.freq_min,
        changes_min: args.changes_min,
    });
    match analysis.run() {
        Ok(cc) => {
            println!("Writing output to {}", output_dir.as_str());
            if let Some(cc_freqs) = &cc.cc_freq {
                mkdir(&output_dir)?;
                write_matrix(cc_freqs_file, &cc_freqs.matrix)?;
                write_arr(cc_files_file, &cc_freqs.col_names)?
            }
            if let Some(cc_probs) = &cc.cc_prob {
                write_matrix(cc_probs_file, &cc_probs.matrix)?
            }
            println!("Completed in {}ms", analysis.duration.num_milliseconds());
            Ok(())
        }
        Err(e) => {
            println!("Failed in {}ms", &analysis.duration.num_milliseconds());
            bail!(e)
        }
    }
}


fn main() {
    let args = Args::parse();
    match run(args) {
        Err(e) => {
            println!("Error occurred: {}", e)
        }
        _ => ()
    }
}
