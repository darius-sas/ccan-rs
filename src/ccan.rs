use std::ops::{AddAssign, Div, Sub};

use chrono::{DateTime, Utc};
use ndarray::{Array2, AssignElem};

use crate::git::{Diff};

#[derive(Debug)]
pub struct NamedMatrix<R, C> {
    pub matrix: Array2<f64>,
    pub row_names: Vec<R>,
    pub col_names: Vec<C>,
    pub row_dimname: Option<String>,
    pub col_dimname: Option<String>,
}

impl<R, C> NamedMatrix<R, C> {
    pub fn new(row_names: Vec<R>, col_names: Vec<C>,
               row_dimname: Option<&str>, col_dimname: Option<&str>) -> NamedMatrix<R, C> {
        NamedMatrix {
            matrix: Array2::<f64>::zeros((row_names.len(), col_names.len())),
            row_names,
            col_names,
            row_dimname: row_dimname.map(String::from),
            col_dimname: col_dimname.map(String::from)
        }
    }

    pub fn from_diffs(diffs: Vec<Diff>) -> NamedMatrix<String, DateTime<Utc>> {
        let mut rows = diffs.iter()
            .map(|d| d.new_file.clone())
            .collect::<Vec<String>>();
        rows.sort();
        rows.dedup();
        let mut cols = diffs.iter()
            .map(|d| d.child.when.clone())
            .collect::<Vec<DateTime<Utc>>>();
        cols.sort();
        cols.dedup();
        NamedMatrix::new(rows, cols, Some("files"), Some("dates"))
    }
}

pub fn dates_distance(dates: Vec<DateTime<Utc>>, distance_smooth: fn(&mut f64) -> ()) -> Array2<f64> {
    let mut mtrx = Array2::<f64>::zeros((dates.len(), dates.len()));
    for i in 0..dates.len() {
        let d1 = dates[i];
        for j in (0..i).rev() {
            let d2 = dates[j];
            mtrx[[i, j]] = d1.sub(d2).num_days() as f64
        };
    };
    println!("{mtrx}");
    mtrx.map_inplace(|i| i.add_assign(1f64));
    mtrx.map_inplace(distance_smooth);
    mtrx.map_inplace(|i| i.assign_elem(1f64.div(*i)));
    mtrx
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use ndarray::AssignElem;

    use crate::ccan::{dates_distance, NamedMatrix};

    #[test]
    fn test_matrix() {
        let rows = vec!["file1", "file2", "file3"].into_iter().map(String::from).collect::<Vec<_>>();
        let cols = vec!["v1", "v2", "v3"].into_iter().map(String::from).collect::<Vec<_>>();
        let m = NamedMatrix::new(rows, cols, None, None);
        println!("{:?}", m)
    }

    #[test]
    fn test_dates_diff() {
        let dates = ["2018-06-01T21:26:03Z", "2018-07-01T21:26:55Z", "2018-07-15T22:00:54Z", "2018-08-01T22:09:57Z", "2018-08-02T17:42:24Z"];
        let dates: Vec<DateTime<Utc>> = dates.iter()
            .filter_map(|d| match DateTime::parse_from_rfc3339(*d) {
                Ok(d) => {Some(d)}
                Err(e) => {println!("{}", e); None}
            })
            .map(|d| d.naive_utc().and_utc())
            .collect();
        println!("{:?}", dates_distance(dates, |f| f.assign_elem(f.sqrt())))
    }
}