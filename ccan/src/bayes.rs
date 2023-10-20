use std::rc::Rc;

use itertools::Itertools;
use log::debug;
use ndarray::ArrayView1;

use crate::{
    changes::Changes,
    cochanges::{CCFreqsCalculator, CCMatrix, CCProbsCalculator, CoChangesOpt},
    model::Model,
    naive::NaiveModel,
    predict::RippleChangePredictor,
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
        debug!("Calculating dates distance");
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
    fn calculate_probs(&self, freqs: &CCMatrix, _opts: &CoChangesOpt) -> CCMatrix {
        let mut cc_probs = CCMatrix::new(
            freqs.row_names.clone(),
            freqs.row_names.clone(),
            Some("posteriori"),
            Some("priori"),
        );
        let sum = freqs.matrix.sum();
        if sum < 1e-6 {
            return cc_probs;
        }

        let intersect = freqs.matrix.mapv(|x| x / sum);
        for i in 0..cc_probs.matrix.nrows() {
            let evidence = intersect.row(i).sum();
            if evidence < 1e-6 {
                continue;
            }
            for j in 0..cc_probs.matrix.ncols() {
                cc_probs.matrix[[i, j]] = intersect[[i, j]] / evidence;
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
        opts: &crate::predict::PredictionOpt,
    ) -> crate::predict::CRVector {
        todo!()
    }
}
