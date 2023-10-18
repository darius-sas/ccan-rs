use std::rc::Rc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use ndarray::{Array1, s};

use ccan::CoChanges;
use changes::Changes;

#[derive(Clone)]
pub struct PredictionOpt {
    pub since_changes: DateTime<Utc>,
    pub until_changes: DateTime<Utc>,
}

pub struct Prediction {
    file: String,
    prob: f64
}
pub trait ChangePredictor {
    fn predict0(cc: &CoChanges, changes: &Changes, opt: &PredictionOpt) -> Vec<Prediction> {
        let indices = changes.freqs.col_names.iter().enumerate()
            .filter(|(i, d)| d.clone() >= &opt.since_changes && d.clone() <= &opt.until_changes)
            .map(|(i, d)| i)
            .collect::<Vec<usize>>();
        if indices.is_empty() {
            return Vec::new();
        }
        let (start, end) = (indices[0], indices[indices.len()]);
        let mut changed_files = Vec::new();
        for i in 0..changes.freqs.row_names.len() {
            let x = changes.freqs.matrix.row(i).slice(s![start..end]).sum();
            if x > 0.0 {
                changed_files.push(changes.freqs.row_names[i].clone().to_string())
            }
        }
        Self::predict(cc, changed_files)
    }

    fn predict(cc: &CoChanges, changed_files: Vec<String>) -> Vec<Prediction>;
}

pub struct NaivePrediction;
impl ChangePredictor for NaivePrediction {
    fn predict(cc: &CoChanges, changed_files: Vec<String>) -> Vec<Prediction> {
        let indices: Vec<usize> = changed_files.into_iter()
            .filter_map(|c| cc.probs.index_of_col(&Rc::new(c)))
            .collect();
        let mut sum = Array1::<f64>::zeros(cc.probs.row_names.len());
        let n = (&indices).len() as f64;
        for i in indices {
            let c = cc.probs.matrix.column(i);
            sum = sum + c;
        }
        sum = sum / n;
        sum.into_iter().enumerate()
            .map(|(i, x)| Prediction {
                file: cc.probs.row_names[i].to_string(),
                prob: x
            })
            .collect::<Vec<Prediction>>()
    }
}