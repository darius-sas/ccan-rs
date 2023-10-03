use std::time::Instant;
use chrono::{DateTime, Utc};
use git2::Repository;
use crate::git::{SimpleGit};
use crate::ccan::{NamedMatrix};
mod git;
mod ccan;

fn main() {
    let repo = Repository::open("/tmp/microservices-demo").unwrap();
    let branch = "main";
    let start = Instant::now();
    let objs = repo.list_objects(branch).expect("cannot retrieve commits");
    let _d = repo.diff(&objs[objs.len() - 1], &objs[objs.len() - 2]);
    let diffs = repo.diff_with_previous(&objs);
    let end = Instant::now();
    println!("{} commits retrieved", objs.len());
    println!("{:?} diffs in {}ms", diffs.len(), (end - start).as_millis());
    println!("{}", diffs[0]);
    println!("{}", diffs[1]);
    println!("{}", diffs[2]);
    println!("{}", diffs[3]);
    println!("{}", diffs[4]);
    println!("{}", diffs[5]);
    println!("{}", diffs[6]);
    println!("{}", diffs[7]);
    println!("{}", diffs[8]);
    println!("{}", diffs[9]);
    let mtrx= NamedMatrix::<String, DateTime<Utc>>::from_diffs(diffs);

    println!("{:?}", &mtrx.row_names[0..5]);
    println!("{:?}", &mtrx.col_names[0..5]);
}
