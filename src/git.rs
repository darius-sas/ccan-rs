use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Sub;

use anyhow::{anyhow, Result};
use chrono::{Datelike, DateTime, Days, TimeZone, Utc};
use git2::{DiffDelta, Object, ObjectType, Repository, Sort};
use itertools::Itertools;
use log::debug;

pub trait SimpleGit {
    fn list_objects(&self, branch: &str) -> Result<Vec<Object>>;
    fn diff(&self, parent: &Object, child: &Object) -> Vec<Diff>;
    fn diff_with_previous(&self, objs: &Vec<Object>, binning: &DateGrouping) -> Diffs;
    fn diffs(&self, branch: &str, binning: &DateGrouping) -> Result<Diffs>;
    fn diffs_max(&self, branch: &str, binning: &DateGrouping, max_commits: usize) -> Result<Diffs>;
}

#[derive(Debug, Clone, Hash)]
pub struct Commit {
    pub sha1: String,
    pub author: String,
    pub email: String,
    pub message: String,
    pub when: DateTime<Utc>,
}

#[derive(Debug, Clone, Hash)]
pub struct Diff {
    pub parent: Commit,
    pub child: Commit,
    pub old_file: String,
    pub new_file: String,
}

impl Commit {
    pub fn from_object(obj: &git2::Object) -> Commit {
        Commit::from_commit(obj.as_commit().unwrap())
    }
    pub fn from_commit(commit: &git2::Commit) -> Commit {
        Commit::new(
            commit.id().to_string(),
            commit
                .author()
                .name()
                .unwrap_or("<no-author-name>")
                .to_string(),
            commit
                .author()
                .email()
                .unwrap_or("<no-author-email")
                .to_string(),
            commit.message().unwrap_or("<no-message>").to_string(),
            commit.time().seconds(),
        )
    }
    pub fn new(
        sha1: String,
        author: String,
        email: String,
        message: String,
        timestamp: i64,
    ) -> Commit {
        Commit {
            sha1,
            author,
            email,
            message,
            when: Utc.timestamp_opt(timestamp, 0).unwrap(),
        }
    }
}

impl Display for Commit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:.6}] {} ({}) {} - {:.20}",
            self.sha1, self.author, self.email, self.when, self.message
        )
    }
}

impl Display for Diff {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:.6}]:{} -> [{:.6}]:{}", self.parent.sha1, self.old_file, self.child.sha1, self.new_file)
    }
}
impl Diff {
    pub fn from(p_obj: &Object, c_obj: &Object, delta: &DiffDelta) -> Diff {
        Diff {
            parent: Commit::from_object(p_obj),
            child: Commit::from_object(c_obj),
            old_file: delta
                .old_file()
                .path()
                .map(|f| f.to_str().unwrap())
                .unwrap_or_else(|| "<unknown>")
                .to_string(),
            new_file: delta
                .new_file()
                .path()
                .map(|f| f.to_str().unwrap())
                .unwrap_or_else(|| "<unknown>")
                .to_string(),
        }
    }
}

impl Eq for Commit {}

impl PartialEq<Self> for Commit {
    fn eq(&self, other: &Self) -> bool {
        self.sha1 == other.sha1
    }
}

impl PartialOrd<Self> for Commit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Commit {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.sha1 == other.sha1 {
            Ordering::Equal
        } else {
            self.when.cmp(&other.when)
        }
    }
}

pub type Diffs = HashMap<DateTime<Utc>, Vec<Diff>>;

impl SimpleGit for Repository {
    fn list_objects(&self, branch: &str) -> Result<Vec<Object>> {
        let mut revwalk = self.revwalk()?;
        revwalk.set_sorting(Sort::REVERSE | Sort::TIME | Sort::TOPOLOGICAL)?;
        let head = match self.revparse_single(branch) {
            Ok(head) => head,
            Err(e) => return Err(anyhow!("cannot find branch {}: {}", branch, e.message())),
        };
        revwalk.push(head.id())?;

        let commits: Vec<Object> = revwalk
            .into_iter()
            .filter_map(|oid| oid.ok())
            .filter_map(|oid| self.revparse_single(oid.to_string().as_str()).ok())
            .collect();
        Ok(commits)
    }

    fn diff(&self, parent: &Object, child: &Object) -> Vec<Diff> {
        let p_obj = parent
            .peel(ObjectType::Tree)
            .expect("valid object expected");
        let c_obj = child.peel(ObjectType::Tree).expect("valid object expected");
        let p_tree = p_obj.as_tree().unwrap();
        let c_tree = c_obj.as_tree().unwrap();

        let diff = self
            .diff_tree_to_tree(Some(p_tree), Some(c_tree), None)
            .expect("failed to diff given objects");
        diff.deltas()
            .map(|d| Diff::from(&parent, &child, &d))
            .collect()
    }
    fn diff_with_previous(&self, objs: &Vec<Object>, binning: &DateGrouping) -> Diffs {
        let n = objs.len() - 1;
        let mut diffs = Vec::with_capacity(n + 2);
        debug!("Calculating diffs");
        for i in 0..n {
            let parent = &objs[i];
            let child = &objs[i + 1];
            self.diff(parent, child).into_iter().for_each(|d| diffs.push(d))
        }
        debug!("Grouping commits using {} strategy", binning);
        let grouped_diffs: HashMap<DateTime<Utc>, Vec<Diff>> = diffs.into_iter()
            .group_by(|d| binning.get_group(&d.child.when))
            .into_iter()
            .map(|(commit, group)| (commit, group.into_iter().collect()))
            .collect();
        grouped_diffs
    }

    fn diffs_max(&self, branch: &str, binning: &DateGrouping, max_commits: usize) -> Result<Diffs> {
        let mut objs = self.list_objects(branch)?;
        let n_skip = (objs.len() as i32) - (max_commits as i32);
        if n_skip > 0 {
            objs = objs.into_iter().skip(n_skip as usize).collect();
        }
        debug!("Mined {} commits on branch {}", objs.len(), branch);
        Ok(self.diff_with_previous(&objs, binning))
    }

    fn diffs(&self, branch: &str, binning: &DateGrouping) -> Result<Diffs> {
        self.diffs_max(branch, binning, usize::MAX)
    }
}


#[derive(clap::ValueEnum, Clone, Debug)]
pub enum DateGrouping {
    None,
    Daily,
    Weekly,
    Monthly,
}

impl Display for DateGrouping{
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
                Utc.with_ymd_and_hms(
                    d.year(),
                    d.month(),
                    d.day(),
                    0,
                    0,
                    0
                ).unwrap()
            },
            DateGrouping::Weekly => {
                Utc.with_ymd_and_hms(
                    d.year(),
                    d.month(),
                    d.day(),
                    0,
                    0,
                    0
                ).unwrap().sub(Days::new(d.weekday().num_days_from_monday() as u64))
            },
            DateGrouping::Monthly => {
                Utc.with_ymd_and_hms(
                    d.year(),
                    d.month(),
                    1,
                    0,
                    0,
                    0
                ).unwrap()
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use crate::git::DateGrouping;

    #[test]
    fn test_test_grouping() {
        let daily = DateGrouping::Daily;
        let date1 = Utc.with_ymd_and_hms(2023, 10, 30, 11, 00, 00).unwrap();
        let date2 = Utc.with_ymd_and_hms(2023, 10, 30, 12, 30, 00).unwrap();
        let date3 = Utc.with_ymd_and_hms(2023, 10, 31, 11, 00, 00).unwrap();

        assert_eq!(daily.get_group(&date1), daily.get_group(&date2));
        assert_ne!(daily.get_group(&date1), daily.get_group(&date3));
        assert_ne!(daily.get_group(&date2), daily.get_group(&date3));

        let weekly = DateGrouping::Weekly;
        let date1 = Utc.with_ymd_and_hms(2023, 10, 30, 12, 00, 01).unwrap();
        let date2 = Utc.with_ymd_and_hms(2023, 10, 31, 12, 30, 00).unwrap();
        let date3 = Utc.with_ymd_and_hms(2023, 10, 28, 11, 00, 00).unwrap();

        assert_eq!(weekly.get_group(&date1), weekly.get_group(&date2));
        assert_ne!(weekly.get_group(&date1), weekly.get_group(&date3));
        assert_ne!(weekly.get_group(&date2), weekly.get_group(&date3));

        let monthly = DateGrouping::Monthly;
        let date1 = Utc.with_ymd_and_hms(2023, 10, 30, 12, 00, 01).unwrap();
        let date2 = Utc.with_ymd_and_hms(2023, 10, 3, 12, 30, 00).unwrap();
        let date3 = Utc.with_ymd_and_hms(2023, 9, 28, 11, 00, 00).unwrap();

        assert_eq!(monthly.get_group(&date1), monthly.get_group(&date2));
        assert_ne!(monthly.get_group(&date1), monthly.get_group(&date3));
        assert_ne!(monthly.get_group(&date2), monthly.get_group(&date3));
    }
}