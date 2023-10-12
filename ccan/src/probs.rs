use changes::Changes;
use ccan::{CCMatrix, CCProbsCalculator, CoChangesOpt};

pub struct NaiveProbs;
impl CCProbsCalculator for NaiveProbs {
    fn calculate_probs(&self, _: &Changes, freqs: &CCMatrix, _: &CoChangesOpt) -> CCMatrix {
        let mut cc_prob = CCMatrix::new(
            freqs.row_names.clone(),
            freqs.row_names.clone(),
            Some("impacted"),
            Some("changing"),
        );
        for i in 0..freqs.matrix.ncols() {
            let col = freqs.matrix.column(i);
            let col_sum = col.sum();
            cc_prob.matrix.column_mut(i).assign(&col.mapv(|x| x / col_sum));
        }
        cc_prob
    }


}

struct BayesProbs;
impl CCProbsCalculator for BayesProbs {
    fn calculate_probs(&self, changes: &Changes, freqs: &CCMatrix, opts: &CoChangesOpt) -> CCMatrix {
        todo!()
    }
}