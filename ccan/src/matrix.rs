use std::collections::HashMap;
use std::hash::Hash;

use ndarray::Array2;

#[derive(Debug)]
pub struct NamedMatrix<R, C>
    where
        R: PartialEq + Eq + Hash + Clone,
        C: PartialEq + Eq + Hash + Clone {
    pub matrix: Array2<f64>,
    pub row_names: Vec<R>,
    pub col_names: Vec<C>,
    row_index: HashMap<R, usize>,
    col_index: HashMap<C, usize>,
    pub row_dimname: Option<String>,
    pub col_dimname: Option<String>,
}

impl<R: PartialEq + Eq + Hash + Clone, C: PartialEq + Eq + Hash + Clone> NamedMatrix<R, C> {
    pub fn new(row_names: Vec<R>, col_names: Vec<C>,
               row_dimname: Option<&str>, col_dimname: Option<&str>) -> NamedMatrix<R, C> {
        let n = row_names.len();
        let m = col_names.len();
        let row_index: HashMap<R, usize> = row_names.iter().enumerate().map(|(i, e)| ((*e).clone(), i)).collect();
        let col_index: HashMap<C, usize> = col_names.iter().enumerate().map(|(i, e)| ((*e).clone(), i)).collect();
        NamedMatrix {
            matrix: Array2::<f64>::zeros((n, m)),
            row_names,
            col_names,
            row_index,
            col_index,
            row_dimname: row_dimname.map(String::from),
            col_dimname: col_dimname.map(String::from)
        }
    }

    pub fn index_of_col(&self, col: &C) -> Option<usize> {
        self.col_index.get(col).map(|u| *u)
    }

    pub fn index_of_row(&self, row: &R) -> Option<usize> {
        self.row_index.get(row).map(|u| *u)
    }

    pub fn slice_columns<'a, I>(&self, col_names: I) -> Vec<usize>
    where I: Iterator<Item=C>
    {
        col_names.filter_map(|c| self.col_index.get(&c))
            .map(|c|*c).collect()
    }
}
