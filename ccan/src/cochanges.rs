use std::rc::Rc;

use log::debug;

use changes::Changes;
use matrix::NamedMatrix;

use crate::model::ModelTypes;

pub type CCMatrix = NamedMatrix<Rc<String>, Rc<String>>;

#[derive(Clone, Debug)]
pub struct CoChangesOpt {
    pub changes_min: u32,
    pub freq_min: u32,
    pub algorithm: ModelTypes,
}

pub struct CoChanges {
    pub freqs: CCMatrix,
    pub probs: CCMatrix,
}

pub trait CCFreqsCalculator {
    fn calculate_freqs(&self, changes: &Changes, opts: &CoChangesOpt) -> CCMatrix;
}

pub trait CCProbsCalculator {
    fn calculate_probs(&self, changes: &Changes, freqs: &CCMatrix, opts: &CoChangesOpt) -> CCMatrix;
}

impl CoChanges {
    pub fn from_changes(changes: &Changes, opts: &CoChangesOpt) -> CoChanges {
        debug!(
            "Calculating frequency of changes for {} files in {} commits",
            changes.freqs.row_names.len(),
            changes.freqs.col_names.len()
        );
        let model = opts.algorithm.get_model();
        let cc_freqs = model.calculate_freqs(changes, opts);
        debug!(
            "Calculating cochange probabilities for {} remaining files",
            cc_freqs.row_names.len()
        );
        let cc_probs = model.calculate_probs(&changes, &cc_freqs, opts);
        CoChanges {
            freqs: cc_freqs,
            probs: cc_probs,
        }
    }
}
