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

use ccan::{CoChanges, CoChangesOpt};
use predict::{NaivePrediction, Prediction, PredictionOpt};

use crate::bettergit::{BetterGit, BetterGitOpt};
use crate::changes::Changes;


pub mod bettergit;
pub mod changes;
pub mod matrix;
pub mod ccan;
pub mod freqs;
pub mod probs;
pub mod predict;

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
    pub pred_opts: PredictionOpt
}

pub struct AnalysisOutput {
    pub changes: Changes,
    pub co_changes: CoChanges,
    pub predict: Vec<Prediction>
}

impl Analysis {
    pub fn new(opts: Options) -> Analysis {
        Analysis { id: 0, opts, output: None, start: None, end: None, duration: Duration::seconds(0), status: AnalysisStatus::Initialized }
    }
    pub fn run(&mut self) -> Result<&AnalysisOutput>{
        self.status = AnalysisStatus::Running;
        self.start = Some(Utc::now());
        let result = Analysis::execute(&self.opts);
        self.end = Some(Utc::now());
        self.duration = self.end.unwrap() - self.start.unwrap();
        return match result {
            Ok(cc) =>{
                self.status = AnalysisStatus::Completed;
                self.output = Some(cc);
                Ok(self.output.as_ref().unwrap())
            } ,
            Err(e) => {
                self.status = AnalysisStatus::Failed;
                bail!(e)
            }
        }
    }

    fn execute(opt: &Options) -> Result<AnalysisOutput> {
        let repo = Repository::open(&opt.repository)?;
        let diffs = repo.mine_diffs(&opt.git_opts)?;
        let changes = Changes::from_diffs(diffs);
        let co_changes = CoChanges::from_changes(&changes, &opt.cc_opts);
        //let prediction = <NaivePrediction as predict::ChangePredictor>::predict0(&co_changes, &changes, &opt.pred_opts);
        Ok(AnalysisOutput{
            changes,
            co_changes,
            predict: Vec::new()
        })
    }
}