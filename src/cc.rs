use std::ops::{AddAssign, Div, Sub};
use std::rc::Rc;
use chrono::{DateTime, Utc};
use log::debug;
use ndarray::{Array2, ArrayView1, AssignElem};
use crate::bettergit::GroupedBetterDiffs;
use crate::matrix::NamedMatrix;

#[derive(Clone)]
pub struct CoChangesOpt {
    pub changes_min: u32,
    pub freq_min: u32,
}
pub struct CoChanges {
    pub changes: NamedMatrix<Rc<String>, DateTime<Utc>>,
    pub cc_freq: Option<NamedMatrix<Rc<String>, Rc<String>>>,
    pub cc_prob: Option<NamedMatrix<Rc<String>, Rc<String>>>
}

impl CoChanges {
    pub fn from_diffs(diffs: GroupedBetterDiffs) -> CoChanges {
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
        let mut cc = CoChanges { changes, cc_freq: None, cc_prob: None };
        cc.calculate_changes(diffs);
        cc
    }

    fn calculate_changes(&mut self, diffs: GroupedBetterDiffs) {
        for (dates, diffs_in_commit) in diffs {
            let col = self.changes.index_of_col(&dates);
            for new_file in diffs_in_commit.new_files {
                let row = self.changes.index_of_row(&new_file);
                match (row, col) {
                    (Some(r), Some(c)) => {
                        self.changes.matrix[[r, c]] += 1.0
                    }
                    (_, _) => ()
                }
            }
        }
    }

    pub fn calculate_cc_freq(&mut self, min_change_freq: u32) {
        debug!("Initiating co-change analysis for {} commits and {} files", self.changes.col_names.len(), self.changes.row_names.len());
        let min_change_freq = min_change_freq as f64;
        let mut filt_row_names = Vec::<Rc<String>>::new();
        for row in self.changes.row_names.iter() {
            if let Some(i) = self.changes.index_of_row(row) {
                if self.changes.matrix.row(i).sum() >= min_change_freq {
                    filt_row_names.push(row.clone());
                }
            }
        }

        let n = filt_row_names.len();
        let mut cc_freq = NamedMatrix::new(
            filt_row_names.clone(),
            filt_row_names.clone(),
            Some("impacted"),
            Some("changed"));
        debug!("Calculating dates distance");
        let dates_dist = CoChanges::dates_distance(&self.changes.col_names, |x| x.assign_elem(x.sqrt()));
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

    pub fn calculate_cc_prob(&mut self) {
        if let Some(cc_freq) = &self.cc_freq {
            let mut cc_prob = NamedMatrix::<Rc<String>, Rc<String>>::new(
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

    fn cc_coefficient(f1: &ArrayView1<f64>, f2: &ArrayView1<f64>, dates_dist: &Array2<f64>) -> f64 {
        let mut coeff = 0f64;
        let n = f1.len();
        for i in (0..n).rev() {
            if f1[i] < 1e-5 { continue }
            for j in (0..=i).rev() {
                if (f2[j] - 1f64).abs() < 1e-5 {
                    coeff = coeff + dates_dist[[i, j]];
                }
            }
        }
        coeff
    }

    pub fn dates_distance(dates: &Vec<DateTime<Utc>>, distance_smooth: fn(&mut f64) -> ()) -> Array2<f64> {
        let shape = (dates.len(), dates.len());
        let mut mtrx = Array2::<f64>::zeros(shape);
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

    fn predict() {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{File, read_to_string};
    use std::str::FromStr;
    use chrono::{DateTime, Utc};
    use csv::ReaderBuilder;
    use ndarray::{Array2, AssignElem};
    use ndarray_csv::Array2Reader;
    use crate::cc::CoChanges;


    #[test]
    fn test_dates_dist() {
        let dates: Vec<DateTime<Utc>> = read_to_string("test-data/sampled_dates.csv").unwrap()
            .lines()
            .map(|s| i64::from_str(s).unwrap())
            .map(|i| DateTime::<Utc>::from_timestamp(i, 0).unwrap())
            .collect();

        let mut actual = CoChanges::dates_distance(&dates, |f| f.assign_elem(f.sqrt()));
        let file = File::open("test-data/expected_dates_distance.csv").unwrap();
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b' ')
            .from_reader(file);
        let mut expected: Array2<f64> = reader
            .deserialize_array2((dates.len(), dates.len())).unwrap();
        expected.map_inplace(|f| f.assign_elem((f.clone() * 1e6).trunc() / 1e6));
        actual.map_inplace(|f| f.assign_elem((f.clone() * 1e6).trunc() / 1e6));

        assert_eq!(expected, actual)
    }
    #[test]
    fn test_cc_coeff() {
        let dates: Vec<DateTime<Utc>> = read_to_string("test-data/sampled_dates.csv").unwrap()
            .lines()
            .map(|s| i64::from_str(s).unwrap())
            .map(|i| DateTime::<Utc>::from_timestamp(i, 0).unwrap())
            .collect();
        let dates_distance = CoChanges::dates_distance(&dates, |f| f.assign_elem(f.sqrt()));
        let file = File::open("test-data/changes.csv").unwrap();
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b' ')
            .from_reader(file);
        let changes: Array2<f64> = reader
            .deserialize_array2((2, dates.len())).unwrap();

        let cc_coeff = CoChanges::cc_coefficient(&changes.row(0), &changes.row(1), &dates_distance);

        let expected = read_to_string("test-data/expected_coeff.csv").unwrap();
        let expected = f64::from_str(expected.trim()).unwrap();

        assert!((cc_coeff - expected).abs() < 1e-6)

    }
}