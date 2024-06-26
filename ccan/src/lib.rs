extern crate anyhow;
extern crate chrono;
extern crate git2;
extern crate itertools;
extern crate log;
extern crate ndarray;
extern crate regex;

use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use git2::Repository;

use cochanges::{CoChanges, CoChangesOpt};
use predict::{PredictionOpt, RippleChangeProbabilities};

use crate::bettergit::{BetterGit, BetterGitOpt};
use crate::changes::Changes;

pub mod bayes;
pub mod bettergit;
pub mod changes;
pub mod cochanges;
pub mod matrix;
pub mod model;
pub mod naive;
pub mod predict;
pub mod nop;

pub enum AnalysisStatus {
    Initialized,
    Running,
    Completed,
    Failed,
}

pub struct Analysis {
    pub id: u64,
    pub opts: Options,
    pub output: Option<AnalysisOutput>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub duration: Duration,
    pub status: AnalysisStatus,
}

#[derive(Clone)]
pub struct Options {
    pub repository: String,
    pub git_opts: BetterGitOpt,
    pub cc_opts: CoChangesOpt,
    pub pred_opts: PredictionOpt,
}

pub struct AnalysisOutput {
    pub changes: Changes,
    pub co_changes: CoChanges,
    pub ripples: RippleChangeProbabilities,
}

impl Analysis {
    pub fn new(opts: Options) -> Analysis {
        Analysis {
            id: 0,
            opts,
            output: None,
            start: None,
            end: None,
            duration: Duration::seconds(0),
            status: AnalysisStatus::Initialized,
        }
    }

    pub fn run(&mut self) -> Result<&AnalysisOutput> {
        self.status = AnalysisStatus::Running;
        self.start = Some(Utc::now());
        let result = Analysis::execute(&self.opts);
        self.end = Some(Utc::now());
        self.duration = self.end.unwrap() - self.start.unwrap();
        return match result {
            Ok(cc) => {
                self.status = AnalysisStatus::Completed;
                self.output = Some(cc);
                Ok(self.output.as_ref().unwrap())
            }
            Err(e) => {
                self.status = AnalysisStatus::Failed;
                bail!(e)
            }
        };
    }

    fn execute(opt: &Options) -> Result<AnalysisOutput> {
        let repo = Repository::open(&opt.repository)?;
        let diffs = repo.mine_diffs(&opt.git_opts)?;
        let changes = Changes::from_diffs(diffs);
        let co_changes = CoChanges::from_changes(&changes, &opt.cc_opts);
        let predictions = RippleChangeProbabilities::from(&co_changes, &changes, &opt.pred_opts);
        Ok(AnalysisOutput {
            changes,
            co_changes,
            ripples: predictions,
        })
    }
}
