use std::path::Path;

use anyhow::{bail, Result};
use chrono::{NaiveDate, TimeZone, Utc};
use clap::{arg, Parser};
use log::{info, LevelFilter, warn};
use simple_logger::SimpleLogger;
use crate::bettergit::{BetterGitOpt, CommitFilteringOpt, DateGrouping, FileFilteringOpt};
use crate::cc::CoChangesOpt;
use crate::ccan::{Analysis, Options};
use crate::output::{create_path, mkdir, write_arr, write_matrix};

mod ccan;
mod matrix;
mod output;
mod bettergit;
mod cc;

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
    #[arg(long, default_value = "9999-1-1")]
    until: NaiveDate,
    #[arg(long, default_value = "1900-1-1")]
    since: NaiveDate,
    #[arg(long, default_value = ".*")]
    include_regex: String,
    #[arg(long, default_value = r".*(json|lock|sh|proto|bat|md|txt|yaml|yml|Dockerfile|mod|sum|.DS_Store|.gitignore)$",)]
    exclude_regex: String,
    #[arg(short, long, default_value = "none")]
    date_binning: DateGrouping,
    #[arg(short, long, required = true)]
    output_dir: String,
}

fn run(args: Args) -> Result<()> {
    let basename = Path::new(args.repository.as_str())
        .file_name().map_or_else(||"repo", |p| p.to_str().unwrap());
    let output_dir = create_path(&[args.output_dir.as_str(), "ccan-output", basename]);
    let d = &args.date_binning;
    let c = args.changes_min;
    let f = args.changes_min;
    let cc_freqs_file =  &create_path(&[output_dir.as_str(), format!("cc_freqs-d{d}-c{c}-f{f}.csv").as_str()]);
    let cc_probs_file = &create_path(&[output_dir.as_str(), format!("cc_probs-d{d}-c{c}-f{f}.csv").as_str()]);
    let cc_files_file = &create_path(&[output_dir.as_str(), format!("cc_files-d{d}-c{c}-f{f}.csv").as_str()]);

    info!("Started analysing {}", args.repository.as_str());
    let since = Utc::from_utc_datetime(&Utc, &args.since.and_hms_opt(0, 0, 0).unwrap());
    let until = Utc::from_utc_datetime(&Utc, &args.until.and_hms_opt(23, 59, 59).unwrap());
    let file_filters = FileFilteringOpt::new(&[args.exclude_regex.as_str()], &[args.include_regex.as_str()]);
    let mut analysis = Analysis::new(
        Options {
            repository: args.repository,
            cc_opts: CoChangesOpt {
                freq_min: args.freq_min,
                changes_min: args.changes_min
            },
            git_opts: BetterGitOpt {
                file_filters,
                commit_filters: CommitFilteringOpt {
                    branch: args.branch,
                    binning: args.date_binning,
                    since,
                    until,
                }
            }
        }
    );
    match analysis.run() {
        Ok(cc) => {
            info!("Writing output to {}", output_dir.as_str());
            if let Some(cc_freqs) = &cc.cc_freq {
                mkdir(&output_dir)?;
                write_matrix(cc_freqs_file, &cc_freqs.matrix)?;
                write_arr(cc_files_file, &cc_freqs.col_names)?
            }
            if let Some(cc_probs) = &cc.cc_prob {
                write_matrix(cc_probs_file, &cc_probs.matrix)?
            }
            info!("Completed in {}ms", analysis.duration.num_milliseconds());
            Ok(())
        }
        Err(e) => {
            warn!("Failed in {}ms", &analysis.duration.num_milliseconds());
            bail!(e)
        }
    }
}


fn main() {
    let args = Args::parse();
    SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .init().unwrap();
    match run(args) {
        Err(e) => {
            info!("Error occurred: {}", e);
        }
        _ => ()
    }
}
