extern crate anyhow;
extern crate ccan;
extern crate chrono;
extern crate clap;
extern crate csv;
extern crate itertools;
extern crate log;
extern crate ndarray;
extern crate ndarray_csv;
extern crate regex;
extern crate serde;
extern crate simple_logger;

use std::path::Path;
use std::str::FromStr;

use anyhow::{bail, Result};
use chrono::{NaiveDate, TimeZone, Utc};
use clap::{arg, Parser};
use log::{error, info, LevelFilter, warn};
use simple_logger::SimpleLogger;

use ccan::{Analysis, Options};
use ccan::bettergit::{BetterGitOpt, CommitFilteringOpt, DateGrouping, FileFilteringOpt};
use ccan::ccan::{CCAlgorithm, CoChangesOpt};
use ccan::predict::PredictionOpt;
use output::{create_path, mkdir, write_arr, write_matrix, write_named_matrix};

mod output;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = " Mine file co-changes from a Git repository.")]
#[command(
help_template = "{about-section} Version: {version} \n by {author} \n {usage-heading} {usage} \n {all-args} {tab}"
)]
struct Args {
    #[arg(short, long, required = true, help = "The git repository")]
    repository: String,
    #[arg(short, long, required = true, help = "The branch to mine commits from")]
    branch: String,
    #[arg(short, long, default_value = "5", help = "Ignore files with fewer total changes than given")]
    changes_min: u32,
    #[arg(short, long, default_value = "5", help = "Remove file pairs with co-change frequency lower than given")]
    freq_min: u32,
    #[arg(long, default_value = "9999-1-1", help = "Select commits until given date (YYYY-MM-DD)")]
    until: NaiveDate,
    #[arg(long, default_value = "1900-1-1", help = "Select commits after given date (YYYY-MM-DD)")]
    since: NaiveDate,
    #[arg(short, long, value_enum, default_value = "none", help = "Binning strategy for commits. None is more precise, but slower. [possible values: none, daily, weekly, monthly]", value_parser = DateGrouping::from_str)]
    date_binning: DateGrouping,
    #[arg(short, long, value_enum, default_value = "naive", help = "Impact probability calculation algorithm. [possible values: naive, bayes, mixed]", value_parser = CCAlgorithm::from_str)]
    algorithm: CCAlgorithm,
    #[arg(long, default_value = ".*", help = "Regex to include matching files (case insensitive)")]
    include_regex: String,
    #[arg(long, default_value = r".*(json|lock|sh|proto|bat|md|txt|yaml|yml|Dockerfile|mod|sum|.DS_Store|.gitignore)$", help = "Regex to exclude matching files (case insensitive)")]
    exclude_regex: String,
    #[arg(long, default_value = "false", help = "Whether to perform a prediction using the output data")]
    predict: bool,
    #[arg(long, default_value = "1900-1-1", help = "Predict changes based on files changed since the given date (YYYY-MM-DD)")]
    predict_since: NaiveDate,
    #[arg(long, default_value = "9999-1-1", help = "Predict changes based on files changed until the given date (YYYY-MM-DD)")]
    predict_until: NaiveDate,
    #[arg(short, long, required = true, help = "Directory to write output files to")]
    output_dir: String,
    #[arg(short, long, default_value = "Debug", help = "Logging level [possible values: Off, Error, Warn, Info, Debug, Trace]")]
    log_level: LevelFilter,
}

fn run(args: Args) -> Result<()> {
    let basename = Path::new(args.repository.as_str())
        .file_name().map_or_else(|| "repo", |p| p.to_str().unwrap());
    let output_dir = create_path(&[args.output_dir.as_str(), "ccan-output", basename]);
    let d = &args.date_binning;
    let c = args.changes_min;
    let f = args.freq_min;
    let a = &args.algorithm;
    let cc_freqs_file = &create_path(&[output_dir.as_str(), format!("cc_freqs-a{a}-d{d}-c{c}-f{f}.csv").as_str()]);
    let cc_probs_file = &create_path(&[output_dir.as_str(), format!("cc_probs-a{a}-d{d}-c{c}-f{f}.csv").as_str()]);
    let cc_files_file = &create_path(&[output_dir.as_str(), format!("cc_files-a{a}-d{d}-c{c}-f{f}.csv").as_str()]);
    let c_data_file = &create_path(&[output_dir.as_str(), format!("c_hist-a{a}-d{d}-c{c}-f{f}.csv").as_str()]);

    info!("Started analysing {}", args.repository.as_str());
    let since = Utc::from_utc_datetime(&Utc, &args.since.and_hms_opt(0, 0, 0).unwrap());
    let until = Utc::from_utc_datetime(&Utc, &args.until.and_hms_opt(23, 59, 59).unwrap());

    let predict_since = Utc::from_utc_datetime(&Utc, &args.predict_since.and_hms_opt(0, 0, 0).unwrap());
    let predict_until = Utc::from_utc_datetime(&Utc, &args.predict_until.and_hms_opt(23, 59, 59).unwrap());

    let file_filters = FileFilteringOpt::new(&[args.exclude_regex.as_str()], &[args.include_regex.as_str()]);
    let mut analysis = Analysis::new(
        Options {
            repository: args.repository,
            cc_opts: CoChangesOpt {
                freq_min: args.freq_min,
                changes_min: args.changes_min,
                algorithm: args.algorithm,
            },
            git_opts: BetterGitOpt {
                file_filters,
                commit_filters: CommitFilteringOpt {
                    branch: args.branch,
                    binning: args.date_binning,
                    since,
                    until,
                },
            },
            pred_opts: PredictionOpt {
                since_changes: predict_since,
                until_changes: predict_until,
            },
        }
    );
    match analysis.run() {
        Ok(output) => {
            info!("Writing output to {}", output_dir.as_str());
            mkdir(&output_dir)?;
            write_matrix(cc_freqs_file, &output.co_changes.freqs.matrix)?;
            write_arr(cc_files_file, &output.co_changes.freqs.col_names)?;
            write_matrix(cc_probs_file, &output.co_changes.probs.matrix)?;
            write_named_matrix(c_data_file, &output.changes.freqs)?;
            println!("{}", &output.predictions);
            info!("Completed in {}ms", (&analysis.duration).num_milliseconds());
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
        .with_level(args.log_level)
        .init().unwrap();
    match run(args) {
        Err(e) => {
            error!("Error occurred: {}", e);
        }
        _ => ()
    }
}
