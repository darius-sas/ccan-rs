use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::str::FromStr;
use anyhow::{bail, Error};
use log::debug;

use changes::Changes;
use freqs::{BayesFreqs, NaiveFreqs};
use matrix::NamedMatrix;
use probs::{BayesProbs, NaiveProbs};

pub type CCMatrix = NamedMatrix<Rc<String>, Rc<String>>;

#[derive(Clone, Debug)]
pub struct CoChangesOpt {
    pub changes_min: u32,
    pub freq_min: u32,
    pub algorithm: CCAlgorithm
}

#[derive(Clone, Debug)]
pub enum CCAlgorithm {
    Naive,
    Bayes,
    Mixed
}

pub struct Calculators {
    freq_calc: Box<dyn CCFreqsCalculator>,
    prob_calc: Box<dyn CCProbsCalculator>
}

impl CoChangesOpt {
    fn get_calculators(&self) -> Calculators {
        match self.algorithm {
            CCAlgorithm::Naive => Calculators { freq_calc: Box::new(NaiveFreqs), prob_calc: Box::new(NaiveProbs) },
            CCAlgorithm::Bayes => Calculators { freq_calc: Box::new(BayesFreqs), prob_calc: Box::new(BayesProbs) },
            CCAlgorithm::Mixed => Calculators { freq_calc: Box::new(NaiveFreqs), prob_calc: Box::new(BayesProbs) }
        }
    }
}

impl FromStr for CCAlgorithm {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "naive" => Ok(CCAlgorithm::Naive),
            "bayes" => Ok(CCAlgorithm::Bayes),
            "mixed" => Ok(CCAlgorithm::Mixed),
            _ => bail!("cannot parse DateGrouping from {}", s)
        }
    }
}

impl Display for CCAlgorithm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            CCAlgorithm::Naive => "naive",
            CCAlgorithm::Bayes => "bayes",
            CCAlgorithm::Mixed => "mixed"
        })
    }
}

pub struct CoChanges {
    pub freqs: CCMatrix,
    pub probs: CCMatrix
}

pub trait CCFreqsCalculator {
    fn calculate_freqs(&self, changes: &Changes, opts: &CoChangesOpt) -> CCMatrix;
}

pub trait CCProbsCalculator {
    fn calculate_probs(&self, freqs: &CCMatrix, opts: &CoChangesOpt) -> CCMatrix;
}

impl CoChanges {
    pub fn from_changes(changes: &Changes, opts: &CoChangesOpt) -> CoChanges {
        debug!("Calculating frequencies");
        let calculators = opts.get_calculators();
        let cc_freqs = calculators.freq_calc.calculate_freqs(changes, opts);
        debug!("Calculating probabilities");
        let cc_probs = calculators.prob_calc.calculate_probs(&cc_freqs, opts);
        CoChanges { freqs: cc_freqs, probs: cc_probs }
    }
}


