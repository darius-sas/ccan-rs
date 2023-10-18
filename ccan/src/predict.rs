use std::fmt::{Display, Formatter};
use std::rc::Rc;

use chrono::{DateTime, Utc};
use itertools::Itertools;
use ndarray::{Array1, s};

use ccan::CoChanges;
use changes::Changes;

#[derive(Clone)]
pub struct PredictionOpt {
    pub since_changes: DateTime<Utc>,
    pub until_changes: DateTime<Utc>,
}

pub struct ChangeRippleProbabilities {
    pub changing_files: Vec<String>,
    pub predictions: Vec<(String, f64)>
}

impl ChangeRippleProbabilities {
    fn new(changing_files: Vec<String>) -> ChangeRippleProbabilities {
        ChangeRippleProbabilities {
            predictions: Vec::new(),
            changing_files: changing_files
        }
    }
}
impl Display for ChangeRippleProbabilities {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Changing files in period: {:?}", &self.changing_files)?;
        writeln!(f, "Change Probability     File")?;
        let sorted = self.predictions.iter()
            .filter(|p| p.1 >= 1e-2)
            .sorted_by(|x, y| y.1.total_cmp(&x.1))
            .collect::<Vec<&(String, f64)>>();
        for prediction in sorted {
            writeln!(f, "              {:0.2}     {}", prediction.1, prediction.0)?
        }
        Ok(())
    }
}
pub trait ChangePredictor {
    fn predict0(cc: &CoChanges, changes: &Changes, opt: &PredictionOpt) -> ChangeRippleProbabilities {
        let indices = changes.freqs.col_names.iter().enumerate()
            .filter(|(_i, d)| d.clone() >= &opt.since_changes && d.clone() <= &opt.until_changes)
            .map(|(i, _d)| i)
            .collect::<Vec<usize>>();
        if indices.is_empty() {
            return ChangeRippleProbabilities::new(vec![]);
        }
        let (start, end) = (indices[0], indices[indices.len() - 1]);
        let mut changed_files = Vec::new();
        for i in 0..changes.freqs.row_names.len() {
            let x = changes.freqs.matrix.row(i).slice(s![start..end]).sum();
            if x > 0.0 {
                changed_files.push(changes.freqs.row_names[i].clone().to_string())
            }
        }
        Self::predict(cc, changed_files)
    }

    fn predict(cc: &CoChanges, changed_files: Vec<String>) -> ChangeRippleProbabilities;
}

pub struct NaivePrediction;
impl ChangePredictor for NaivePrediction {
    fn predict(cc: &CoChanges, changed_files: Vec<String>) -> ChangeRippleProbabilities {
        let indices: Vec<usize> = changed_files.clone().into_iter()
            .filter_map(|c| cc.probs.index_of_col(&Rc::new(c)))
            .collect();
        let mut sum = Array1::<f64>::zeros(cc.probs.row_names.len());
        let n = (&indices).len() as f64;
        for i in indices {
            let c = cc.probs.matrix.column(i);
            sum = sum + c;
        }
        sum = sum / n;
        let mut predictions = ChangeRippleProbabilities::new(changed_files);
        sum.into_iter().enumerate()
            .map(|(i, x)| (cc.probs.row_names[i].to_string(), x))
            .for_each(|p| predictions.predictions.push(p));

        predictions
    }
}