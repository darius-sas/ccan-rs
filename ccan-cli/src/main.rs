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

use anyhow::{bail, Result};
use args::Args;
use clap::Parser;
use log::{error, info, warn};
use simple_logger::SimpleLogger;

use ccan::Analysis;
use output::{mkdir, write_arr, write_matrix, write_named_matrix};

use crate::output::{csv_file_name, output_dir};

mod args;
mod output;

fn run(args: Args) -> Result<()> {
    let output_dir = output_dir(&args);
    let cc_freqs_file = &csv_file_name(&args, "cc_freqs");
    let cc_probs_file = &csv_file_name(&args, "cc_probs");
    let cc_files_file = &csv_file_name(&args, "cc_files");
    let c_data_file = &csv_file_name(&args, "c_hist");
    let c_ripple_file = &csv_file_name(&args, "c_ripple");

    info!("Started analysing {}", args.repository.as_str());
    let skip_predict = args.skip_predict;
    let mut analysis = Analysis::new(args.to_options());
    match analysis.run() {
        Ok(output) => {
            info!("Writing output to {}", output_dir.as_str());
            mkdir(&output_dir)?;
            write_matrix(cc_freqs_file, &output.co_changes.freqs.matrix)?;
            write_arr(cc_files_file, &output.co_changes.freqs.col_names)?;
            write_matrix(cc_probs_file, &output.co_changes.probs.matrix)?;
            write_named_matrix(c_data_file, &output.changes.freqs)?;
            if !skip_predict {
                write_arr(c_ripple_file, &output.ripples.get_probabilities())?;
                println!("{}", &output.ripples);
            }
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
        .init()
        .unwrap();
    match run(args) {
        Err(e) => {
            error!("Error occurred: {}", e);
        }
        _ => (),
    }
}
