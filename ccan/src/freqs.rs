use std::ops::{AddAssign, Div, Sub};
use std::rc::Rc;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::debug;
use ndarray::{Array2, ArrayView1, AssignElem};
use changes::Changes;
use ccan::{CCFreqsCalculator, CCMatrix, CoChangesOpt};

pub struct NaiveFreqs;
impl NaiveFreqs {
    fn dates_distance(dates: &Vec<DateTime<Utc>>, distance_smooth: fn(&mut f64) -> ()) -> Array2<f64> {
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

    fn filter_freqs(freqs: &mut CCMatrix, min_freq: u32) {
        let min_freq = &mut (min_freq as f64);
        freqs.matrix
            .map_inplace(|f| if f.le(&min_freq) {
                f.assign_elem(0f64);
            });
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
}
impl CCFreqsCalculator for NaiveFreqs {
    fn calculate_freqs(&self, changes: &Changes, opts: &CoChangesOpt) -> CCMatrix {
        let changes = &changes.changes;
        debug!("Initiating co-change analysis for {} commits and {} files", changes.col_names.len(), changes.row_names.len());
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
            Some("changed"));
        debug!("Calculating dates distance");
        let dates_dist = Self::dates_distance(&changes.col_names, |x| x.assign_elem(x.sqrt()));
        debug!("Calculating co-change coefficient");
        for i in 0..n {
            let row_i = changes.matrix.row(i);
            for j in 0..n {
                if i == j { continue }
                let row_j = changes.matrix.row(j);
                cc_freq.matrix[[i, j]] = Self::cc_coefficient(&row_i, &row_j, &dates_dist);
            }
        }
        Self::filter_freqs(&mut cc_freq, opts.freq_min);
        cc_freq
    }
}

pub struct BayesFreqs;

fn co_change(v1: ArrayView1<f64>, v2: ArrayView1<f64>) -> f64 {
    v1.iter().zip_eq(v2).filter(|(x, y)| **x > 0.0 && **y > 0.0).count() as f64
}

impl CCFreqsCalculator for BayesFreqs {
    fn calculate_freqs(&self, changes: &Changes, opts: &CoChangesOpt) -> CCMatrix {
        let changes = &changes.changes;
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
            Some("changed"));
        debug!("Calculating dates distance");
        debug!("Calculating co-change coefficient");
        for i in 0..n {
            let row_i = changes.matrix.row(i);
            for j in 0..n {
                if i == j { continue }
                let row_j = changes.matrix.row(j);
                cc_freq.matrix[[i, j]] = co_change(row_i, row_j);
            }
        }
        NaiveFreqs::filter_freqs(&mut cc_freq, opts.freq_min);
        cc_freq
    }
}

#[cfg(test)]
mod tests {
    extern crate csv;
    extern crate ndarray_csv;

    use std::fs::{File, read_to_string};
    use std::str::FromStr;
    use chrono::{DateTime, Utc};
    use self::csv::ReaderBuilder;
    use ndarray::{Array2, AssignElem};
    use freqs::NaiveFreqs;
    use self::ndarray_csv::Array2Reader;

    #[test]
    fn test_dates_dist() {
        let dates: Vec<DateTime<Utc>> = read_to_string("../test-data/sampled_dates.csv").unwrap()
            .lines()
            .map(|s| i64::from_str(s).unwrap())
            .map(|i| DateTime::<Utc>::from_timestamp(i, 0).unwrap())
            .collect();

        let mut actual = NaiveFreqs::dates_distance(&dates, |f| f.assign_elem(f.sqrt()));
        let file = File::open("../test-data/expected_dates_distance.csv").unwrap();
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
        let dates: Vec<DateTime<Utc>> = read_to_string("../test-data/sampled_dates.csv").unwrap()
            .lines()
            .map(|s| i64::from_str(s).unwrap())
            .map(|i| DateTime::<Utc>::from_timestamp(i, 0).unwrap())
            .collect();
        let dates_distance = NaiveFreqs::dates_distance(&dates, |f| f.assign_elem(f.sqrt()));
        let file = File::open("../test-data/changes.csv").unwrap();
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b' ')
            .from_reader(file);
        let changes: Array2<f64> = reader
            .deserialize_array2((2, dates.len())).unwrap();

        let cc_coeff = NaiveFreqs::cc_coefficient(&changes.row(0), &changes.row(1), &dates_distance);

        let expected = read_to_string("../test-data/expected_coeff.csv").unwrap();
        let expected = f64::from_str(expected.trim()).unwrap();

        assert!((cc_coeff - expected).abs() < 1e-6)

    }
}