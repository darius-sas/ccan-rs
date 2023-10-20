use std::ops::{AddAssign, Div, Sub};
use std::rc::Rc;

use chrono::{DateTime, Utc};
use log::debug;
use ndarray::{Array1, Array2, ArrayView1, AssignElem};

use changes::Changes;

use crate::cochanges::{CCFreqsCalculator, CCMatrix, CCProbsCalculator, CoChanges, CoChangesOpt};
use crate::model::Model;
use crate::predict::{CRVector, PredictionOpt, RippleChangePredictor};

pub struct NaiveModel;
impl Model for NaiveModel {}
impl NaiveModel {
    pub fn dates_distance(
        dates: &Vec<DateTime<Utc>>,
        distance_smooth: fn(&mut f64) -> (),
    ) -> Array2<f64> {
        let shape = (dates.len(), dates.len());
        let mut mtrx = Array2::<f64>::zeros(shape);
        for i in 0..dates.len() {
            let d1 = dates[i];
            for j in (0..i).rev() {
                let d2 = dates[j];
                mtrx[[i, j]] = d1.sub(d2).num_days() as f64
            }
        }
        mtrx.map_inplace(|i| i.add_assign(1f64));
        mtrx.map_inplace(distance_smooth);
        mtrx.map_inplace(|i| i.assign_elem(1f64.div(*i)));
        mtrx
    }

    pub fn filter_freqs(freqs: &mut CCMatrix, min_freq: u32) {
        let min_freq = &mut (min_freq as f64);
        freqs.matrix.map_inplace(|f| {
            if f.le(&min_freq) {
                f.assign_elem(0f64);
            }
        });
    }

    pub fn cc_coefficient(
        f1: &ArrayView1<f64>,
        f2: &ArrayView1<f64>,
        dates_dist: &Array2<f64>,
    ) -> f64 {
        let mut coeff = 0f64;
        let n = f1.len();
        for i in (0..n).rev() {
            if f1[i] < 1e-5 {
                continue;
            }
            for j in (0..=i).rev() {
                if (f2[j] - 1f64).abs() < 1e-5 {
                    coeff = coeff + dates_dist[[i, j]];
                }
            }
        }
        coeff
    }
}

impl CCFreqsCalculator for NaiveModel {
    fn calculate_freqs(&self, changes: &Changes, opts: &CoChangesOpt) -> CCMatrix {
        let changes = &changes.freqs;
        let min_change_freq = opts.changes_min as f64;
        let mut filt_row_names = Vec::<Rc<String>>::new();
        for row in changes.row_names.iter() {
            if let Some(i) = changes.index_of_row(row) {
                if changes.matrix.row(i).sum() >= min_change_freq {
                    filt_row_names.push(row.clone());
                }
            }
        }

        let n = filt_row_names.len();
        let mut cc_freq = CCMatrix::new(
            filt_row_names.clone(),
            filt_row_names.clone(),
            Some("impacted"),
            Some("changed"),
        );
        debug!(
            "Calculating dates distance ({} dates)",
            changes.col_names.len()
        );
        let dates_dist = Self::dates_distance(&changes.col_names, |x| x.assign_elem(x.sqrt()));
        debug!("Calculating co-change coefficient");
        for i in 0..n {
            let row_i = changes.matrix.row(i);
            for j in 0..n {
                if i == j {
                    continue;
                }
                let row_j = changes.matrix.row(j);
                cc_freq.matrix[[i, j]] = Self::cc_coefficient(&row_i, &row_j, &dates_dist);
            }
        }
        Self::filter_freqs(&mut cc_freq, opts.freq_min);
        cc_freq
    }
}

impl CCProbsCalculator for NaiveModel {
    fn calculate_probs(&self, freqs: &CCMatrix, _: &CoChangesOpt) -> CCMatrix {
        let mut cc_prob = CCMatrix::new(
            freqs.row_names.clone(),
            freqs.row_names.clone(),
            Some("impacted"),
            Some("changing"),
        );
        for i in 0..freqs.matrix.ncols() {
            let col = freqs.matrix.column(i);
            let col_sum = col.sum();
            cc_prob
                .matrix
                .column_mut(i)
                .assign(&col.mapv(|x| x / col_sum));
        }
        cc_prob
    }
}

impl RippleChangePredictor for NaiveModel {
    fn predict(
        &self,
        cc: &CoChanges,
        changed_files: &Vec<String>,
        _opt: &PredictionOpt,
    ) -> CRVector {
        let indices: Vec<usize> = changed_files
            .clone()
            .into_iter()
            .filter_map(|c| cc.probs.index_of_col(&Rc::new(c)))
            .collect();
        let mut sum = Array1::<f64>::zeros(cc.probs.row_names.len());
        let n = (&indices).len() as f64;
        for i in indices {
            let c = cc.probs.matrix.column(i);
            sum = sum + c;
        }
        sum = sum / n;
        sum.into_iter()
            .enumerate()
            .map(|(i, x)| (cc.probs.row_names[i].to_string(), x))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    extern crate csv;
    extern crate ndarray_csv;

    use std::fs::{read_to_string, File};
    use std::str::FromStr;

    use chrono::{DateTime, Utc};
    use ndarray::{Array2, AssignElem};

    use crate::naive::NaiveModel;

    use self::csv::ReaderBuilder;
    use self::ndarray_csv::Array2Reader;

    #[test]
    fn test_dates_dist() {
        let dates: Vec<DateTime<Utc>> = read_to_string("../test-data/sampled_dates.csv")
            .unwrap()
            .lines()
            .map(|s| i64::from_str(s).unwrap())
            .map(|i| DateTime::<Utc>::from_timestamp(i, 0).unwrap())
            .collect();

        let mut actual = NaiveModel::dates_distance(&dates, |f| f.assign_elem(f.sqrt()));
        let file = File::open("../test-data/expected_dates_distance.csv").unwrap();
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b' ')
            .from_reader(file);
        let mut expected: Array2<f64> = reader
            .deserialize_array2((dates.len(), dates.len()))
            .unwrap();
        expected.map_inplace(|f| f.assign_elem((f.clone() * 1e6).trunc() / 1e6));
        actual.map_inplace(|f| f.assign_elem((f.clone() * 1e6).trunc() / 1e6));

        assert_eq!(expected, actual)
    }

    #[test]
    fn test_cc_coeff() {
        let dates: Vec<DateTime<Utc>> = read_to_string("../test-data/sampled_dates.csv")
            .unwrap()
            .lines()
            .map(|s| i64::from_str(s).unwrap())
            .map(|i| DateTime::<Utc>::from_timestamp(i, 0).unwrap())
            .collect();
        let dates_distance = NaiveModel::dates_distance(&dates, |f| f.assign_elem(f.sqrt()));
        let file = File::open("../test-data/changes.csv").unwrap();
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b' ')
            .from_reader(file);
        let changes: Array2<f64> = reader.deserialize_array2((2, dates.len())).unwrap();

        let cc_coeff =
            NaiveModel::cc_coefficient(&changes.row(0), &changes.row(1), &dates_distance);

        let expected = read_to_string("../test-data/expected_coeff.csv").unwrap();
        let expected = f64::from_str(expected.trim()).unwrap();

        assert!((cc_coeff - expected).abs() < 1e-6)
    }
}
