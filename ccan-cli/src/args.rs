use ccan::bettergit::{BetterGitOpt, CommitFilteringOpt, DateGrouping, FileFilteringOpt};
use ccan::cochanges::CoChangesOpt;
use ccan::model::ModelTypes;
use ccan::predict::PredictionOpt;
use ccan::Options;
use chrono::{DateTime, Days, NaiveDate, TimeZone, Utc};
use clap::{arg, Parser};
use log::LevelFilter;

use std::ops::{Add, Sub};

use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = " Mine file co-changes from a Git repository."
)]
#[command(
    help_template = "{about-section} Version: {version} \n by {author} \n {usage-heading} {usage} \n {all-args} {tab}"
)]
pub struct Args {
    #[arg(short, long, required = true, help = "The git repository")]
    pub repository: String,
    #[arg(short, long, required = true, help = "The branch to mine commits from")]
    pub branch: String,
    #[arg(
        short,
        long,
        default_value = "5",
        help = "Ignore files with fewer total changes than given"
    )]
    pub changes_min: u32,
    #[arg(
        short,
        long,
        default_value = "5",
        help = "Remove file pairs with co-change frequency lower than given"
    )]
    pub freq_min: u32,
    #[arg(
        long,
        default_value = "9999-1-1",
        help = "Select commits until given date (YYYY-MM-DD)"
    )]
    pub until: NaiveDate,
    #[arg(
        long,
        default_value = "1900-1-1",
        help = "Select commits after given date (YYYY-MM-DD)"
    )]
    pub since: NaiveDate,
    #[arg(short, long, value_enum, default_value = "none", help = "Binning strategy for commits. None is more precise, but slower. [possible values: none, daily, weekly, monthly]", value_parser = DateGrouping::from_str)]
    pub date_binning: DateGrouping,
    #[arg(short, long, value_enum, default_value = "naive", help = "Impact probability calculation algorithm. [possible values: naive, bayes, mixed, nop]", value_parser = ModelTypes::from_str)]
    pub algorithm: ModelTypes,
    #[arg(
        long,
        default_value = ".*",
        help = "Regex to include matching files (case insensitive)"
    )]
    pub include_regex: String,
    #[arg(
        long,
        default_value = r".*(json|lock|sh|proto|bat|md|txt|yaml|yml|Dockerfile|mod|sum|.DS_Store|.gitignore)$",
        help = "Regex to exclude matching files (case insensitive)"
    )]
    pub exclude_regex: String,
    #[arg(
        long,
        default_value = "false",
        help = "Do not perform a prediction using the cochange probability"
    )]
    pub skip_predict: bool,
    #[arg(
        long,
        default_value_t = Utc::now().sub(Days::new(30)).date_naive(),
        help = "Predict changes based on files changed since the given date (YYYY-MM-DD)"
    )]
    predict_since: NaiveDate,
    #[arg(
        long,
        default_value_t = Utc::now().add(Days::new(1)).date_naive(),
        help = "Predict changes based on files changed until the given date (YYYY-MM-DD)"
    )]
    predict_until: NaiveDate,
    #[arg(
        short,
        long,
        required = true,
        help = "Directory to write output files to"
    )]
    pub output_dir: String,
    #[arg(
        short,
        long,
        default_value = "Debug",
        help = "Logging level [possible values: Off, Error, Warn, Info, Debug, Trace]"
    )]
    pub log_level: LevelFilter,
}

impl Args {
    pub fn to_options(self) -> Options {
        let since = Args::to_datetime_0(&self.since);
        let until = Args::to_datetime_23(&self.until);
        let predict_since = Args::to_datetime_0(&self.predict_since);
        let predict_until = Args::to_datetime_23(&self.predict_until);

        let file_filters = FileFilteringOpt::new(
            &[self.exclude_regex.as_str()],
            &[self.include_regex.as_str()],
        );
        Options {
            repository: self.repository,
            cc_opts: CoChangesOpt {
                freq_min: self.freq_min,
                changes_min: self.changes_min,
                algorithm: self.algorithm,
            },
            git_opts: BetterGitOpt {
                file_filters,
                commit_filters: CommitFilteringOpt {
                    branch: self.branch,
                    binning: self.date_binning,
                    since,
                    until,
                },
            },
            pred_opts: PredictionOpt {
                skip: self.skip_predict,
                since_changes: predict_since,
                until_changes: predict_until,
                algorithm: self.algorithm,
            },
        }
    }

    fn to_datetime_0(naive_date: &NaiveDate) -> DateTime<Utc> {
        Utc::from_utc_datetime(&Utc, &naive_date.and_hms_opt(0, 0, 0).unwrap())
    }

    fn to_datetime_23(naive_date: &NaiveDate) -> DateTime<Utc> {
        Utc::from_utc_datetime(&Utc, &naive_date.and_hms_opt(23, 59, 59).unwrap())
    }
}
