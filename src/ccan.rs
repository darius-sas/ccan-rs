use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{AddAssign, Div, Sub};

use chrono::{DateTime, Utc};
use ndarray::{Array2, ArrayView1, AssignElem};

use crate::git::Diffs;

#[derive(Debug)]
pub struct NamedMatrix<R, C>
    where
        R: PartialEq + Eq + Hash + Clone,
        C: PartialEq + Eq + Hash + Clone {
    pub matrix: Array2<f64>,
    pub row_names: Vec<R>,
    pub col_names: Vec<C>,
    row_index: HashMap<R, usize>,
    col_index: HashMap<C, usize>,
    pub row_dimname: Option<String>,
    pub col_dimname: Option<String>,
}

impl<R: PartialEq + Eq + Hash + Clone, C: PartialEq + Eq + Hash + Clone> NamedMatrix<R, C> {
    pub fn new(row_names: Vec<R>, col_names: Vec<C>,
               row_dimname: Option<&str>, col_dimname: Option<&str>) -> NamedMatrix<R, C> {
        let n = row_names.len();
        let m = col_names.len();
        let row_index: HashMap<R, usize> = row_names.iter().enumerate().map(|(i, e)| ((*e).clone(), i)).collect();
        let col_index: HashMap<C, usize> = col_names.iter().enumerate().map(|(i, e)| ((*e).clone(), i)).collect();
        NamedMatrix {
            matrix: Array2::<f64>::zeros((n, m)),
            row_names,
            col_names,
            row_index,
            col_index,
            row_dimname: row_dimname.map(String::from),
            col_dimname: col_dimname.map(String::from)
        }
    }

    pub fn index_of_col(&self, col: &C) -> Option<usize> {
        self.col_index.get(col).map(|u| *u)
    }

    pub fn index_of_row(&self, row: &R) -> Option<usize> {
        self.row_index.get(row).map(|u| *u)
    }
}

pub struct CoChanges {
    pub changes: NamedMatrix<String, DateTime<Utc>>,
    pub cc_freq: Option<NamedMatrix<String, String>>,
    pub cc_prob: Option<NamedMatrix<String, String>>
}

impl CoChanges {

    pub fn from_diffs(diffs: Diffs) -> CoChanges {
        let mut rows = diffs.values()
            .flatten()
            .map(|d| d.new_file.clone())
            .collect::<Vec<String>>();
        rows.sort();
        rows.dedup();
        let mut cols = diffs.keys()
            .map(|d| d.when.clone())
            .collect::<Vec<DateTime<Utc>>>();
        cols.sort();
        cols.dedup();
        let mut changes = NamedMatrix::new(
            rows,
            cols,
            Some("files"),
            Some("dates")
        );
        CoChanges::calculate_changes(diffs, &mut changes);
        CoChanges { changes, cc_freq: None, cc_prob: None }
    }

    pub fn calculate_changes(diffs: Diffs, changes: &mut NamedMatrix<String, DateTime<Utc>>) {
        for pair in diffs {
            let commit = pair.0;
            let diffs_in_commit = pair.1;
            for diff in diffs_in_commit {
                let file = diff.new_file;
                let row = changes.index_of_row(&file);
                let col = changes.index_of_col(&commit.when);
                match (row, col) {
                    (Some(r), Some(c)) => {
                        changes.matrix[[r, c]] += 1.0
                    }
                    (_, _) => ()
                }
            }
        }
    }
    pub fn dates_distance(&self, distance_smooth: fn(&mut f64) -> ()) -> Array2<f64> {
        let dates = &self.changes.col_names;
        let mut mtrx = Array2::<f64>::zeros((dates.len(), dates.len()));
        for i in 0..dates.len() {
            let d1 = dates[i];
            for j in (0..i).rev() {
                let d2 = dates[j];
                mtrx[[i, j]] = d1.sub(d2).num_days() as f64
            };
        };
        mtrx.map_inplace(|i| i.add_assign(1f64));
        mtrx.map_inplace(distance_smooth);
        mtrx.map_inplace(|i| i.assign_elem(1f64.div(*i)));
        mtrx
    }
    pub fn calculate_cc_freq(&mut self, min_change_freq: u32) {
        let min_change_freq = min_change_freq as f64;
        let mut filt_row_names = Vec::<String>::new();
        for row in self.changes.row_names.iter() {
            if let Some(i) = self.changes.index_of_row(row) {
                if self.changes.matrix.row(i).sum() >= min_change_freq {
                    filt_row_names.push((*row).clone());
                }
            }
        }

        let n = filt_row_names.len();
        let mut cc_freq = NamedMatrix::new(
            filt_row_names.clone(),
            filt_row_names.clone(),
            Some("impacted"),
            Some("changed"));
        let dates_dist = self.dates_distance(|x| x.assign_elem(x.sqrt()));
        for i in 0..n {
            let row_i = self.changes.matrix.row(i);
            for j in 0..n {
                if i == j { continue }
                let row_j = self.changes.matrix.row(j);
                cc_freq.matrix[[i, j]] = CoChanges::cc_coefficient(&row_i, &row_j, &dates_dist);
            }
        }
        self.cc_freq = Some(cc_freq);
    }

    pub fn filter_freqs(&mut self, min_freq: u32) {
        if let Some(cc_freqs) = &mut self.cc_freq {
            let min_freq = &mut (min_freq as f64);
            cc_freqs.matrix
                .map_inplace(|f| if f.le(&min_freq) {
                    f.assign_elem(0f64);
                })
        }
    }

    fn cc_coefficient(f1: &ArrayView1<f64>, f2: &ArrayView1<f64>, dates_dist: &Array2<f64>) -> f64 {
        let mut coeff = 0f64;
        let n = f1.len();
        for i in (0..n).rev() {
            if f1[i] < 1e-5 { continue }
            for j in (0..n).rev() {
                if (f2[j] - 1f64).abs() < 1e-5 {
                    coeff = coeff + dates_dist[[i, j]];
                }
            }
        }
        coeff
    }

    pub fn calculate_cc_prob(&mut self) {
        if let Some(cc_freq) = &self.cc_freq {
            let mut cc_prob = NamedMatrix::<String, String>::new(
                cc_freq.row_names.clone(),
                cc_freq.row_names.clone(),
                Some("impacted"),
                Some("changing"),
            );
            for i in 0..cc_freq.matrix.ncols() {
                let col = cc_freq.matrix.column(i);
                let col_sum = col.sum();
                cc_prob.matrix.column_mut(i).assign(&col.mapv(|x| x / col_sum));
            }
            self.cc_prob = Some(cc_prob);
        }
    }
}


#[cfg(test)]
mod tests {
    use std::ops::Sub;

    use chrono::{Days, Utc};
    use git2::Repository;
    use ndarray::{array, Array2, s};

    use crate::ccan::{CoChanges, NamedMatrix};
    use crate::git::{Commit, Diff, Diffs, SimpleGit};

    #[test]
    fn test_matrix() {
        let rows = vec!["file1", "file2", "file3"].into_iter().map(String::from).collect::<Vec<_>>();
        let cols = vec!["v1", "v2", "v3"].into_iter().map(String::from).collect::<Vec<_>>();
        let m = NamedMatrix::new(rows, cols, None, None);
        println!("{:?}", m)
    }

    #[test]
    fn test_cochanges() {
        let repo = Repository::open("/tmp/microservices-demo").unwrap();
        let branch = "main";
        let diffs = repo.diffs(branch).expect("cannot get diffs");

        let changes = CoChanges::from_diffs(diffs);

        assert!(changes.changes.matrix.nrows() > 0);
        assert!(changes.changes.matrix.ncols() > 0);

        println!("{}", changes.changes.matrix.slice(s![0..10, 0..10]))
    }

    #[test]
    fn test_changes_calc() {
        let c1 = Commit::new(String::from("sha_abc1"), String::from("author1"), String::from("author1@email.com"), String::from("message1"), Utc::now().sub(Days::new(3)).timestamp());
        let c2 = Commit::new(String::from("sha_abc2"), String::from("author2"), String::from("author2@email.com"), String::from("message2"), Utc::now().sub(Days::new(2)).timestamp());
        let c3 = Commit::new(String::from("sha_abc3"), String::from("author3"), String::from("author3@email.com"), String::from("message3"), Utc::now().sub(Days::new(1)).timestamp());
        let d1 = Diff { parent: c1.clone(), child: c2.clone(), old_file: String::from("my/file.txt"), new_file: String::from("my/file.txt") };
        let d2 = Diff { parent: c1.clone(), child: c2.clone(), old_file: String::from("my/file2.txt"), new_file: String::from("my/file2.txt") };
        let d3 = Diff { parent: c1.clone(), child: c2.clone(), old_file: String::from("my/file3.txt"), new_file: String::from("my/file3.txt") };
        let d4 = Diff { parent: c2.clone(), child: c3.clone(), old_file: String::from("my/file.txt"), new_file: String::from("my/file.txt") };
        let d5 = Diff { parent: c2.clone(), child: c3.clone(), old_file: String::from("my/file3.txt"), new_file: String::from("my/file3.txt") };
        let mut diffs = Diffs::new();
        diffs.insert(c2, vec![d1, d2, d3].clone());
        diffs.insert(c3, vec![d4, d5].clone());

        let mut cc = CoChanges::from_diffs(diffs);
        let mut expected = Array2::<f64>::ones((3, 2));
        expected[[1, 1]] = 0f64;
        assert_eq!(expected, cc.changes.matrix);
        cc.calculate_cc_freq(0);
        let expected = array![[0.0, 1.7071067811865475, 3.7071067811865475], [2.0, 0.0, 2.0], [3.7071067811865475, 1.7071067811865475, 0.0]];
        assert_eq!(expected, cc.cc_freq.as_ref().unwrap().matrix);
        cc.calculate_cc_prob();
        let expected = array![[0.0, 0.5, 0.6495597372397182], [0.3504402627602818, 0.0, 0.3504402627602818], [0.6495597372397182, 0.5, 0.0]];
        assert_eq!(expected, cc.cc_prob.as_ref().unwrap().matrix);
    }

    // #[test]
    // fn test_dates_diff() {
    //     let dates = ["2018-06-01T21:26:03Z", "2018-07-01T21:26:55Z", "2018-07-15T22:00:54Z", "2018-08-01T22:09:57Z", "2018-08-02T17:42:24Z"];
    //     let dates: Vec<DateTime<Utc>> = dates.iter()
    //         .filter_map(|d| match DateTime::parse_from_rfc3339(*d) {
    //             Ok(d) => {Some(d)}
    //             Err(e) => {println!("{}", e); None}
    //         })
    //         .map(|d| d.naive_utc().and_utc())
    //         .collect();
    //     println!("{:?}", dates_distance(dates, |f| f.assign_elem(f.sqrt())))
    // }
}