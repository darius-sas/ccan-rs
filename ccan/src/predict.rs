use std::fmt::{Display, Formatter};

use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::debug;
use ndarray::s;

use changes::Changes;
use cochanges::CoChanges;

use crate::model::ModelTypes;

#[derive(Clone)]
pub struct PredictionOpt {
    pub skip: bool,
    pub since_changes: DateTime<Utc>,
    pub until_changes: DateTime<Utc>,
    pub algorithm: ModelTypes,
}

pub type CRVector = Vec<(String, f64)>;
pub struct RippleChangeProbabilities {
    pub changing_files: Vec<String>,
    pub ripples: CRVector,
}

impl RippleChangeProbabilities {
    fn new() -> RippleChangeProbabilities {
        RippleChangeProbabilities {
            ripples: Vec::new(),
            changing_files: Vec::new(),
        }
    }

    pub fn from(
        cc: &CoChanges,
        changes: &Changes,
        opt: &PredictionOpt,
    ) -> RippleChangeProbabilities {
        if opt.skip {
            return RippleChangeProbabilities::new();
        }
        let indices = changes
            .freqs
            .col_names
            .iter()
            .enumerate()
            .filter(|(_i, d)| d.clone() >= &opt.since_changes && d.clone() <= &opt.until_changes)
            .map(|(i, _d)| i)
            .collect::<Vec<usize>>();
        if indices.is_empty() {
            return RippleChangeProbabilities::new();
        }
        let (start, end) = (indices[0], indices[indices.len() - 1]);
        let mut changing_files = Vec::new();
        for i in 0..changes.freqs.row_names.len() {
            let x = changes.freqs.matrix.row(i).slice(s![start..end]).sum();
            if x > 0.0 {
                changing_files.push(changes.freqs.row_names[i].clone().to_string())
            }
        }

        let model = opt.algorithm.get_model();
        debug!(
            "Calculating ripple change probability from {} files using '{}' algorithm",
            &changing_files.len(),
            opt.algorithm
        );
        let ripples = model.predict(cc, &changing_files, opt);
        RippleChangeProbabilities {
            changing_files,
            ripples,
        }
    }

    pub fn get_probabilities(&self) -> Vec<f64> {
        self.ripples.iter().map(|r| r.1).collect()
    }
}

impl Display for RippleChangeProbabilities {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Changing files in period: {:?}", &self.changing_files)?;
        writeln!(f, "Change Probability     File")?;
        let sorted = self
            .ripples
            .iter()
            .filter(|p| p.1 >= 1e-2)
            .sorted_by(|x, y| y.1.total_cmp(&x.1))
            .collect::<Vec<&(String, f64)>>();
        for prediction in sorted {
            writeln!(f, "              {:0.2}     {}", prediction.1, prediction.0)?
        }
        Ok(())
    }
}

pub trait RippleChangePredictor {
    fn predict(
        &self,
        cc: &CoChanges,
        changed_files: &Vec<String>,
        opts: &PredictionOpt,
    ) -> CRVector;
}
