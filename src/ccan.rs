use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use git2::Repository;
use crate::ccan::AnalysisStatus::{Completed, Failed, Initialized, Running};
use crate::cochanges::CoChanges;
use crate::git::{DateGrouping, SimpleGit};

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
    pub branch: String,
    pub changes_min: u32,
    pub freq_min: u32,
    pub binning: DateGrouping,
    pub max_commits: u32
}

impl Analysis {
    pub fn new(opts: Options) -> Analysis {
        Analysis { id: 0, opts, result: None, start: None, end: None, duration: Duration::seconds(0), status: Initialized }
    }
    pub fn run(&mut self) -> Result<&CoChanges>{
        self.status = Running;
        self.start = Some(Utc::now());
        let result = Analysis::execute(&self.opts);
        self.end = Some(Utc::now());
        self.duration = self.end.unwrap() - self.start.unwrap();
        return match result {
            Ok(cc) =>{
                self.status = Completed;
                self.result = Some(cc);
                Ok(self.result.as_ref().unwrap())
            } ,
            Err(e) => {
                self.status = Failed;
                bail!(e)
            }
        }
    }

    fn execute(opt: &Options) -> Result<CoChanges> {
        let repo = Repository::open(&opt.repository)?;
        let diffs = repo.diffs_max(opt.branch.as_str(), &opt.binning, opt.max_commits as usize)?;
        let mut cc = CoChanges::from_diffs(diffs);
        cc.calculate_cc_freq(opt.changes_min);
        cc.filter_freqs(opt.freq_min);
        cc.calculate_cc_prob();
        Ok(cc)
    }
}