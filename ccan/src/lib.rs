
pub mod bettergit;
pub mod cc;
pub mod matrix;

extern crate anyhow;
extern crate log;
extern crate chrono;
extern crate git2;
extern crate itertools;
extern crate regex;
extern crate ndarray;

use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use git2::Repository;
use crate::cc::{CoChanges, CoChangesOpt};
use crate::bettergit::{BetterGit, BetterGitOpt};

pub enum AnalysisStatus {
    Initialized,
    Running,
    Completed,
    Failed,
}
pub struct Analysis {
    pub id: u64,
    pub opts: Options,
    pub result: Option<CoChanges>,
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
}

impl Analysis {
    pub fn new(opts: Options) -> Analysis {
        Analysis { id: 0, opts, result: None, start: None, end: None, duration: Duration::seconds(0), status: AnalysisStatus::Initialized }
    }
    pub fn run(&mut self) -> Result<&CoChanges>{
        self.status = AnalysisStatus::Running;
        self.start = Some(Utc::now());
        let result = Analysis::execute(&self.opts);
        self.end = Some(Utc::now());
        self.duration = self.end.unwrap() - self.start.unwrap();
        return match result {
            Ok(cc) =>{
                self.status = AnalysisStatus::Completed;
                self.result = Some(cc);
                Ok(self.result.as_ref().unwrap())
            } ,
            Err(e) => {
                self.status = AnalysisStatus::Failed;
                bail!(e)
            }
        }
    }

    fn execute(opt: &Options) -> Result<CoChanges> {
        let repo = Repository::open(&opt.repository)?;
        let diffs = repo.mine_diffs(&opt.git_opts)?;
        let mut cc = CoChanges::from_diffs(diffs);
        cc.calculate_cc_freq(opt.cc_opts.changes_min);
        cc.filter_freqs(opt.cc_opts.freq_min);
        cc.calculate_cc_prob();
        Ok(cc)
    }
}