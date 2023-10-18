use std::rc::Rc;

use chrono::{DateTime, Utc};
use log::debug;
use ndarray::Array1;

use crate::bettergit::GroupedBetterDiffs;
use crate::matrix::NamedMatrix;

pub struct Changes {
    pub freqs: NamedMatrix<Rc<String>, DateTime<Utc>>,
    pub c_freq: Array1<i32>,
    pub c_prob: Array1<f64>
}

impl Changes {
    pub fn from_diffs(diffs: GroupedBetterDiffs) -> Changes {
        let mut rows = diffs.values()
            .map(|d| d.new_files.iter().map(|f| f.clone()))
            .flatten()
            .collect::<Vec<Rc<String>>>();
        rows.sort();
        rows.dedup();
        let mut cols = diffs.keys()
            .map(|d| d.clone())
            .collect::<Vec<DateTime<Utc>>>();
        cols.sort();
        cols.dedup();
        let changes = NamedMatrix::new(
            rows,
            cols,
            Some("files"),
            Some("dates")
        );
        let n = changes.matrix.nrows();
        let c_freq= Array1::zeros(n);
        let c_prob =  Array1::zeros(n);
        let mut cc = Changes { freqs: changes, c_freq, c_prob };
        cc.calculate_changes(diffs);
        cc.calculate_c_freq_and_prob();
        cc
    }

    fn calculate_changes(&mut self, diffs: GroupedBetterDiffs) {
        debug!("Calculating changes");
        for (dates, diffs_in_commit) in diffs {
            let col = self.freqs.index_of_col(&dates);
            for new_file in diffs_in_commit.new_files {
                let row = self.freqs.index_of_row(&new_file);
                match (row, col) {
                    (Some(r), Some(c)) => {
                        self.freqs.matrix[[r, c]] += 1.0
                    }
                    (_, _) => ()
                }
            }
        }
    }

    fn calculate_c_freq_and_prob(&mut self) {
        let n = self.freqs.matrix.nrows();
        for i in 0..n {
            let r_sum = self.freqs.matrix.row(i).sum();
            self.c_freq[i] = r_sum as i32;
            self.c_prob[i] = r_sum / (n as f64);
        }
    }
}

