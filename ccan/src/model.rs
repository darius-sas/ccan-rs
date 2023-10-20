use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use anyhow::{bail, Error};

use crate::{
    bayes::BayesianModel,
    ccan::{CCFreqsCalculator, CCProbsCalculator},
    naive::NaiveModel,
    predict::RippleChangePredictor,
};

pub trait Model: CCFreqsCalculator + CCProbsCalculator + RippleChangePredictor {}

#[derive(Clone, Debug, Copy)]
pub enum ModelTypes {
    Naive,
    Bayes,
    Mixed,
}

impl ModelTypes {
    pub fn get_model(&self) -> Box<dyn Model> {
        match self {
            ModelTypes::Naive => Box::new(NaiveModel),
            ModelTypes::Bayes => Box::new(BayesianModel),
            ModelTypes::Mixed => Box::new(NaiveModel),
        }
    }
}

impl Display for ModelTypes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ModelTypes::Naive => "naive",
                ModelTypes::Bayes => "bayes",
                ModelTypes::Mixed => "mixed",
            }
        )
    }
}

impl FromStr for ModelTypes {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "naive" => Ok(ModelTypes::Naive),
            "bayes" => Ok(ModelTypes::Bayes),
            "mixed" => Ok(ModelTypes::Mixed),
            _ => bail!("cannot parse DateGrouping from {}", s),
        }
    }
}
