use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Sub;
use std::rc::Rc;
use std::str::FromStr;

use anyhow::{anyhow, bail, Result};
use chrono::{Datelike, DateTime, Days, TimeZone, Utc};
use git2::{Commit, Diff, Object, ObjectType, Repository, Sort};
use itertools::Itertools;
use log::debug;
use regex::{Error, Regex, RegexBuilder};

#[derive(Debug, Clone, Hash)]
pub struct BetterCommit {
    pub sha1: String,
    pub author: String,
    pub when: DateTime<Utc>,
}

pub struct BetterDiff {
    pub parent: Rc<BetterCommit>,
    pub child: Rc<BetterCommit>,
    pub old_files: Vec<Rc<String>>,
    pub new_files: Vec<Rc<String>>,
}

#[derive(Clone)]
pub struct BetterGitOpt {
    pub commit_filters: CommitFilteringOpt,
    pub file_filters: FileFilteringOpt
}

#[derive(Clone)]
pub struct CommitFilteringOpt {
    pub branch: String,
    pub until: DateTime<Utc>,
    pub since: DateTime<Utc>,
    pub binning: DateGrouping,
}

#[derive(Clone)]
pub struct FileFilteringOpt {
    pub exclude_paths: Regex,
    pub include_paths: Regex
}

pub type GroupedBetterDiffs = HashMap<DateTime<Utc>, BetterDiff>;

impl BetterCommit {
    fn from(commit: &Commit) -> BetterCommit {
        BetterCommit {
            sha1: commit.id().to_string(),
            author: commit.author().name().unwrap_or("<no-author-name>").to_string(),
            when: Utc.timestamp_opt(commit.time().seconds(), 0).unwrap(),
        }
    }
}
impl BetterDiff {
    fn new(parent: Rc<BetterCommit>, child: Rc<BetterCommit>) -> BetterDiff {
        BetterDiff {
            parent,
            child,
            old_files: Vec::new(),
            new_files: Vec::new(),
        }
    }
}

impl FileFilteringOpt {
    pub fn new(exclude_patterns: &[&str], include_patterns: &[&str]) -> FileFilteringOpt {
        FileFilteringOpt {
            exclude_paths: FileFilteringOpt::vec_to_regex(exclude_patterns)
                .expect("invalid exclude path regex"),
            include_paths: FileFilteringOpt::vec_to_regex(include_patterns)
                .expect("invalid include path regex")
        }
    }

    fn vec_to_regex(regex_vec: &[&str]) -> std::result::Result<Regex, Error> {
        match regex_vec.len() {
            0 => RegexBuilder::new(r".*"),
            1 => RegexBuilder::new(regex_vec[0]),
            _ => {
                let mut regex_str = regex_vec.join("|").to_owned();
                regex_str.insert(0, '(');
                regex_str.push(')');
                RegexBuilder::new(regex_str.as_str())
            }
        }.case_insensitive(true).build()
    }

    pub fn accept_all() -> FileFilteringOpt {
        FileFilteringOpt::new(&[r"a^"], &[r".*"])
    }

    pub fn include_only(include_patterns: &[&str]) -> FileFilteringOpt {
        FileFilteringOpt::new(&[r"a^"], include_patterns)
    }

    pub fn exclude(&self, path: &String) -> bool {
        self.exclude_paths.is_match(path)
    }

    pub fn include(&self, path: &String) -> bool {
        self.include_paths.is_match(path)
    }

    pub fn matches(&self, path: &String) -> bool {
        !self.exclude(path) && self.include(path)
    }
}

pub trait BetterGit {
    fn mine_objects(&self, filters: &CommitFilteringOpt) -> Result<Vec<Object>>;
    fn sample_commits<'repo>(objects: Vec<Object<'repo>>, binning: &DateGrouping) -> Vec<Object<'repo>>;

    fn diff(&self, parent: &Object, child: &Object) -> Result<Diff>;
    fn diffs(&self, objects: &Vec<Object>, file_filters: &FileFilteringOpt) -> GroupedBetterDiffs;

    fn mine_diffs(&self, options: &BetterGitOpt) -> Result<GroupedBetterDiffs>;
}

impl BetterGit for Repository {
    fn mine_objects(&self, filters: &CommitFilteringOpt) -> Result<Vec<Object>> {
        let mut revwalk = self.revwalk()?;
        revwalk.set_sorting(Sort::REVERSE | Sort::TIME | Sort::TOPOLOGICAL)?;
        let head = match self.revparse_single(filters.branch.as_str()) {
            Ok(head) => head,
            Err(e) => return Err(anyhow!("cannot find branch {}: {}", filters.branch, e.message())),
        };
        revwalk.push(head.id())?;
        let until = filters.until.timestamp();
        let since = filters.since.timestamp();
        let commits: Vec<Object> = revwalk
            .into_iter()
            .filter_map(|oid| oid.ok())
            .filter_map(|oid| self.revparse_single(oid.to_string().as_str()).ok())
            .filter(|o| {
                let commit = (&o).as_commit().expect("not a commit");
                let commit_ts = commit.time().seconds();
                commit_ts > since && commit_ts < until
            })
            .collect();
        let commits = Repository::sample_commits(commits, &filters.binning);
        Ok(commits)
    }

    fn sample_commits<'repo>(objects: Vec<Object<'repo>>, binning: &DateGrouping) -> Vec<Object<'repo>> {
        objects.into_iter()
            .map(|o| {
                let commit = o.as_commit().expect("not a commit");
                let time = Utc.timestamp_opt(commit.time().seconds(), 0).unwrap();
                (o, binning.get_group(&time))
            })
            .sorted_by(|x, y| Ord::cmp(&x.1, &y.1))
            .dedup_by(|x, y| x.1 == y.1)
            .map(|(o, _)| o)
            .collect::<Vec<Object<'repo>>>()
    }

    fn diff(&self, parent: &Object, child: &Object) -> Result<Diff> {
        let p_obj = parent
            .peel(ObjectType::Tree)
            .expect("valid object expected");
        let c_obj = child.peel(ObjectType::Tree).expect("valid object expected");
        let p_tree = p_obj.as_tree().unwrap();
        let c_tree = c_obj.as_tree().unwrap();

        Ok(self.diff_tree_to_tree(Some(p_tree), Some(c_tree), None)?)
    }

    fn diffs(&self, objects: &Vec<Object>, file_filters: &FileFilteringOpt) -> GroupedBetterDiffs {
        let mut diffs = GroupedBetterDiffs::new();
        let rcs: Vec<Rc<BetterCommit>> = objects.iter()
            .map(|o| o.as_commit().expect("not a commit"))
            .map(|cmt| Rc::new(BetterCommit::from(cmt)))
            .collect();
        let mut all_files = HashMap::<Rc<String>, Rc<String>>::new();
        let mut get_rc = |s: String| {
            if !all_files.contains_key(&s) {
                let rcs = Rc::new(s);
                all_files.insert(rcs.clone(), rcs.clone());
                return rcs;
            }
            return all_files.get(&s).unwrap().clone();
        };
        for i in 0..(objects.len() - 1) {
            let parent = &objects[i];
            let child = &objects[i + 1];
            let diff = match self.diff(parent, child) {
                Ok(d) => d,
                Err(_) => {
                    debug!("cannot calculate diff between [{}] and [{}]", parent.id(), child.id());
                    continue;
                }
            };
            let parent_rc = rcs[i].clone();
            let child_rc = rcs[i + 1].clone();

            let mut b_diff = BetterDiff::new(parent_rc, child_rc);
            diff.deltas()
                .for_each(|d| {
                    let old_file = d.old_file().path()
                        .map(|p| p.to_str().unwrap())
                        .unwrap_or("<unknown>")
                        .to_string();
                    if file_filters.matches(&old_file) {
                        let old_file = get_rc(old_file);
                        b_diff.old_files.push(old_file);
                        let new_file = d.new_file().path()
                            .map(|p| p.to_str().unwrap())
                            .unwrap_or("<unknown>")
                            .to_string();
                        let new_file = get_rc(new_file);
                        b_diff.new_files.push(new_file);
                    }
                });
            diffs.insert(b_diff.child.when.clone(), b_diff);
        }
        diffs
    }

    fn mine_diffs(&self, options: &BetterGitOpt) -> Result<GroupedBetterDiffs> {
        let objs = self.mine_objects(&options.commit_filters)?;
        debug!("Found {} total commits", objs.len());
        Ok(self.diffs(&objs, &options.file_filters))
    }
}

#[derive(Clone, Debug)]
pub enum DateGrouping {
    None,
    Daily,
    Weekly,
    Monthly,
}

impl FromStr for DateGrouping {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(DateGrouping::None),
            "daily" => Ok(DateGrouping::Daily),
            "weekly" => Ok(DateGrouping::Weekly),
            "monthly" => Ok(DateGrouping::Monthly),
            _ => bail!("cannot parse DateGrouping from {}", s)
        }
    }
}

impl Display for DateGrouping {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DateGrouping::None => "none",
            DateGrouping::Daily => "daily",
            DateGrouping::Weekly => "weekly",
            DateGrouping::Monthly => "monthly"
        };
        write!(f, "{s}")
    }
}
impl DateGrouping {

    pub fn get_group(&self, d: &DateTime<Utc>) -> DateTime<Utc> {
        match self {
            DateGrouping::None => d.clone(),
            DateGrouping::Daily => {
                Utc.with_ymd_and_hms(d.year(), d.month(), d.day(), 0, 0, 0).unwrap()
            },
            DateGrouping::Weekly => {
                Utc.with_ymd_and_hms(d.year(), d.month(), d.day(), 0, 0, 0).unwrap().sub(Days::new(d.weekday().num_days_from_monday() as u64))
            },
            DateGrouping::Monthly => {
                Utc.with_ymd_and_hms(d.year(), d.month(), 1, 0, 0, 0).unwrap()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use chrono::{TimeZone, Utc};
    use git2::Repository;

    use crate::bettergit::{BetterGit, BetterGitOpt, CommitFilteringOpt, DateGrouping, FileFilteringOpt};

    // TODO: reactivate test
    fn test_filtering() {
        let repo = match Repository::clone("https://github.com/GoogleCloudPlatform/microservices-demo", "/tmp/microservices-demo") {
            Ok(r) => r,
            Err(_) => Repository::open("/tmp/microservices-demo").expect("cannot open nor clone repository")
        };
        let filters = CommitFilteringOpt {
            branch: "main".to_string(),
            since: Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap(),
            until: Utc.with_ymd_and_hms(2020, 12, 31, 23, 59, 59).unwrap(),
            binning: DateGrouping::None
        };
        let commits = repo.mine_objects(&filters).expect("cannot mine");
        assert_eq!(77, commits.len());

        let no_bins = &DateGrouping::None;
        let binning = Repository::sample_commits(commits, no_bins);
        assert_eq!(77, binning.len());

        let commits = repo.mine_objects(&filters).expect("cannot mine");
        let monthly_bins = &DateGrouping::Monthly;
        let binning = Repository::sample_commits(commits, monthly_bins);
        assert_eq!(12, binning.len());
    }

    // TODO: reactivate test
    fn test_diffs(){
        let repo = match Repository::clone("https://github.com/GoogleCloudPlatform/microservices-demo", "/tmp/microservices-demo") {
            Ok(r) => r,
            Err(_) => Repository::open("/tmp/microservices-demo").expect("cannot open nor clone repository")
        };
        let opts = BetterGitOpt {
            commit_filters: CommitFilteringOpt {
                branch: "main".to_string(),
                since: Utc.with_ymd_and_hms(2020, 12, 8, 17, 14, 0).unwrap(),
                until: Utc.with_ymd_and_hms(2020, 12, 31, 23, 59, 59).unwrap(),
                binning: DateGrouping::None,
            },
            file_filters: FileFilteringOpt::accept_all()
        };
        let objs = repo.mine_objects(&opts.commit_filters).expect("cannot list commits");
        let diffs = repo.diffs(&objs, &opts.file_filters);
        let matched_files = diffs.values().into_iter().map(|d| d.new_files.clone()).flatten().collect::<Vec<Rc<String>>>();
        assert_eq!(46, matched_files.len());

        let cs_only = FileFilteringOpt::include_only(&[".*cs$"]);
        let diffs = repo.diffs(&objs, &cs_only);
        let matched_files = diffs.values().into_iter().map(|d| d.new_files.clone()).flatten().collect::<Vec<Rc<String>>>();
        matched_files.iter().for_each(|f| {
            assert!(f.ends_with(".cs"), "file doesn't end with '.cs': {}", f)
        });
    }
}
