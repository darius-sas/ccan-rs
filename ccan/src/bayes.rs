use std::rc::Rc;

use itertools::Itertools;
use log::debug;
use ndarray::{Array1, ArrayView1};

use crate::{
    changes::Changes,
    cochanges::{CCFreqsCalculator, CCMatrix, CCProbsCalculator, CoChangesOpt},
    model::Model,
    naive::NaiveModel,
    predict::{CRVector, RippleChangePredictor},
};

fn co_change(v1: ArrayView1<f64>, v2: ArrayView1<f64>) -> f64 {
    v1.iter()
        .zip_eq(v2)
        .filter(|(x, y)| **x > 0.0 && **y > 0.0)
        .count() as f64
}

pub struct BayesianModel;
impl Model for BayesianModel {}
impl CCFreqsCalculator for BayesianModel {
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
        debug!("Calculating co-change coefficient");
        for i in 0..n {
            let row_i = changes.matrix.row(i);
            for j in 0..n {
                if i == j {
                    continue;
                }
                let row_j = changes.matrix.row(j);
                cc_freq.matrix[[i, j]] = co_change(row_i, row_j);
            }
        }
        NaiveModel::filter_freqs(&mut cc_freq, opts.freq_min);
        cc_freq
    }
}

impl CCProbsCalculator for BayesianModel {
    fn calculate_probs(&self, changes: &Changes, freqs: &CCMatrix, _opts: &CoChangesOpt) -> CCMatrix {
        let mut cc_probs = CCMatrix::new(
            freqs.row_names.clone(),
            freqs.row_names.clone(),
            Some("impacted"),
            Some("changing"),
        );
        let n_vers = changes.n_vers;
        let priori = freqs.matrix.mapv(|x| x / n_vers); // P(impacted /\ changing)
        let evidence = &changes.c_prob; // P(changing)
        for i in 0..cc_probs.matrix.nrows() {
            let e1 = evidence[i];
            if e1 < 1e-6 {
                continue;
            }
            for j in 0..cc_probs.matrix.ncols() {
                let e2 = evidence[j];
                if e2 < 1e-6 {
                    continue;
                }
                cc_probs.matrix[[i, j]] = priori[[i, j]] * e1 / e2 // P(impacted | changing)
            }
        }
        return cc_probs;
    }
}

impl RippleChangePredictor for BayesianModel {
    fn predict(
        &self,
        cc: &crate::cochanges::CoChanges,
        changed_files: &Vec<String>,
        _opt: &crate::predict::PredictionOpt,
    ) -> CRVector {
        let indices: Vec<usize> = changed_files
            .clone()
            .into_iter()
            .filter_map(|c| cc.probs.index_of_col(&Rc::new(c)))
            .collect();
        let mut sum = Array1::<f64>::zeros(cc.probs.row_names.len());
        for i in indices {
            let c = cc.probs.matrix.column(i);
            sum = sum + c;
        }
        sum.into_iter()
            .enumerate()
            .map(|(i, x)| (cc.probs.row_names[i].to_string(), x))
            .collect()
    }
}

pub struct MixedModel;
impl Model for MixedModel {}

impl CCFreqsCalculator for MixedModel {
    fn calculate_freqs(&self, changes: &Changes, opts: &CoChangesOpt) -> CCMatrix {
        NaiveModel::calculate_freqs(&NaiveModel, changes, opts)
    }
}

impl CCProbsCalculator for MixedModel {
    fn calculate_probs(&self, changes: &Changes, freqs: &CCMatrix, opts: &CoChangesOpt) -> CCMatrix {
        BayesianModel::calculate_probs(&BayesianModel, changes, freqs, opts)
    }
}

impl RippleChangePredictor for MixedModel {
    fn predict(
        &self,
        cc: &crate::cochanges::CoChanges,
        changed_files: &Vec<String>,
        opts: &crate::predict::PredictionOpt,
    ) -> CRVector {
        BayesianModel::predict(&BayesianModel, cc, changed_files, opts)
    }
}
