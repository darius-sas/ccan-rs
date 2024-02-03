use log::debug;

use crate::{changes::Changes, cochanges::{CCFreqsCalculator, CCMatrix, CCProbsCalculator, CoChangesOpt}, model::Model, predict::{CRVector, RippleChangePredictor}};


pub struct NopModel;
impl Model for NopModel {}


impl CCFreqsCalculator for NopModel {
    fn calculate_freqs(&self, _changes: &Changes, _opts: &CoChangesOpt) -> CCMatrix {
        debug!("Skipping frequency calculation since 'nop' algorithm.");
        CCMatrix::new(Vec::new(), Vec::new(), None, None)
    }
}

impl CCProbsCalculator for NopModel {
    fn calculate_probs(&self, _freqs: &CCMatrix, _opts: &CoChangesOpt) -> CCMatrix {
        debug!("Skipping probs calculation since 'nop' algorithm.");
        CCMatrix::new(Vec::new(), Vec::new(), None, None)
    }
}

impl RippleChangePredictor for NopModel {
    fn predict(
        &self,
        _cc: &crate::cochanges::CoChanges,
        _changed_files: &Vec<String>,
        _opts: &crate::predict::PredictionOpt,
    ) -> crate::predict::CRVector {
        CRVector::new()
    }
}