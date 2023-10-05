use std::fs;
use std::time::Instant;

use anyhow::Result;
use clap::{arg, Parser};
use git2::Repository;

use crate::ccan::CoChanges;
use crate::git::SimpleGit;
use crate::output::{p, write_arr, write_matrix};

mod git;
mod ccan;
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
    let cc_freqs_file = &p(&args.output_dir, "cc_freqs.csv")?;
    let cc_probs_file = &p(&args.output_dir, "cc_probs.csv")?;
    let cc_files_file = &p(&args.output_dir, "cc_files.csv")?;

    let repo = Repository::open(args.repository)?;
    let diffs = repo.diffs(args.branch.as_str())?;
    let mut cc = CoChanges::from_diffs(diffs);
    cc.calculate_cc_freq(args.changes_min);
    cc.filter_freqs(args.freq_min);
    if let Some(cc_freqs) = &cc.cc_freq {
        fs::create_dir_all(args.output_dir)?;
        write_matrix(cc_freqs_file, &cc_freqs.matrix)?;
        write_arr(cc_files_file, &cc_freqs.col_names)?
    }
    cc.calculate_cc_prob();
    if let Some(cc_probs) = &cc.cc_prob {
        write_matrix(cc_probs_file, &cc_probs.matrix)?
    }
    Ok(())
}

fn main() {
    let args = Args::parse();
    let start = Instant::now();
    println!("Started analysing {}", args.repository);
    match run(args) {
        Ok(_) => {
            let stop = Instant::now() - start;
            println!("Completed in {}ms", stop.as_millis())
        }
        Err(e) => {
            println!("Error occurred: {}", e)
        }
    }
}
