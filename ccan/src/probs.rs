use ccan::{CCMatrix, CCProbsCalculator, CoChangesOpt};

pub struct NaiveProbs;
impl CCProbsCalculator for NaiveProbs {
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
            cc_prob.matrix.column_mut(i).assign(&col.mapv(|x| x / col_sum));
        }
        cc_prob
    }


}

pub struct BayesProbs;
impl CCProbsCalculator for BayesProbs {
    fn calculate_probs(&self, freqs: &CCMatrix, _opts: &CoChangesOpt) -> CCMatrix {
        let mut cc_probs = CCMatrix::new(
            freqs.row_names.clone(),
            freqs.row_names.clone(),
            Some("posteriori"),
            Some("priori")
        );
        let sum = freqs.matrix.sum();
        if sum < 1e-6 { return cc_probs }

        let intersect = freqs.matrix.mapv(|x| x / sum);
        for i in 0..cc_probs.matrix.nrows(){
            let evidence = intersect.row(i).sum();
            if evidence < 1e-6 { continue }
            for j in 0..cc_probs.matrix.ncols() {
                cc_probs.matrix[[i, j]] = intersect[[i, j]] / evidence;
            }
        }
        return cc_probs;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
    }
}