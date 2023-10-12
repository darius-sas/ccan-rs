use std::rc::Rc;
use log::debug;

use changes::Changes;
use matrix::NamedMatrix;

pub type CCMatrix = NamedMatrix<Rc<String>, Rc<String>>;

#[derive(Clone, Debug)]
pub struct CoChangesOpt {
    pub changes_min: u32,
    pub freq_min: u32,
    pub freqs_calculator: &'a dyn CCFreqsCalculator,
    pub probs_calculator: &'a dyn CCProbsCalculator
}

pub struct CoChanges {
    pub cc_freqs: CCMatrix,
    pub cc_probs: CCMatrix
}

pub trait CCFreqsCalculator {
    fn calculate_freqs(&self, changes: &Changes, opts: &CoChangesOpt) -> CCMatrix;
}

pub trait CCProbsCalculator {
    fn calculate_probs(&self, changes: &Changes, freqs: &CCMatrix, opts: &CoChangesOpt) -> CCMatrix;
}

pub struct CCCalculator<'a>{
    pub changes: &'a Changes,
    pub opts: CoChangesOpt<'a>
}

impl<'a> CCCalculator<'a> {
    pub fn calculate(&self, opts: &CoChangesOpt) -> CoChanges {
        debug!("Calculating frequencies");
        let cc_freqs = self.freqs_calculator.calculate_freqs(self.changes, opts);
        debug!("Calculating probabilities");
        let cc_probs = self.probs_calculator.calculate_probs(self.changes, &cc_freqs, opts);
        CoChanges { cc_freqs, cc_probs }
    }
}
