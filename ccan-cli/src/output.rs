use std::fs::File;
use std::path::PathBuf;
use std::rc::Rc;
use std::{fs, path::Path};

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use csv::WriterBuilder;
use itertools::Itertools;
use ndarray::Array2;
use ndarray_csv::Array2Writer;
use serde::Serialize;

use ccan::matrix::NamedMatrix;

use crate::args::Args;

pub fn output_dir(args: &Args) -> String {
    let basename = Path::new(args.repository.as_str())
        .file_name()
        .map_or_else(|| "repo", |p| p.to_str().unwrap());
    create_path(&[args.output_dir.as_str(), "ccan-output", basename])
}

pub fn csv_file_name(args: &Args, prefix: &str) -> String {
    let output_dir = output_dir(args);
    let a = &args.algorithm;
    let d = &args.date_binning;
    let c = args.changes_min;
    let f = args.freq_min;
    create_path(&[
        output_dir.as_str(),
        format!("{prefix}-a{a}-d{d}-c{c}-f{f}.csv").as_str(),
    ])
}

pub fn create_path(names: &[&str]) -> String {
    names
        .iter()
        .map(PathBuf::from)
        .coalesce(|x, y| Ok(x.join(y)))
        .into_iter()
        .map(|p| String::from(p.to_str().unwrap()))
        .join("")
}

pub fn mkdir(output_dir: &String) -> Result<()> {
    match fs::create_dir_all(&output_dir) {
        Err(_) => bail!("Cannot create output dir {}", output_dir),
        _ => Ok(()),
    }
}
pub fn write_matrix<A: Serialize>(path: &String, matrix: &Array2<A>) -> Result<()> {
    if matrix.is_empty() {
        return Ok(());
    }
    let file = File::create(path)?;
    let mut writer = WriterBuilder::new().has_headers(false).from_writer(file);
    Ok(writer.serialize_array2(matrix)?)
}

pub fn write_arr<A: Serialize>(path: &String, matrix: &Vec<A>) -> Result<()> {
    if matrix.is_empty() {
        return Ok(());
    }
    let file = File::create(path)?;
    let mut writer = WriterBuilder::new().has_headers(false).from_writer(file);
    Ok(writer.serialize(matrix)?)
}

pub fn write_named_matrix(
    path: &String,
    matrix: &NamedMatrix<Rc<String>, DateTime<Utc>>,
) -> Result<()> {
    if matrix.matrix.is_empty() {
        return Ok(());
    }
    let file = File::create(path)?;
    let mut writer = WriterBuilder::new().has_headers(false).from_writer(file);
    let columns = matrix
        .col_names
        .iter()
        .map(|d| d.clone().to_string())
        .collect::<Vec<String>>();
    writer.write_field("")?;
    writer.write_record(columns)?;
    for (i, row_name) in matrix.row_names.iter().enumerate() {
        writer.write_field(row_name.to_string())?;
        let row = matrix
            .matrix
            .row(i)
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        writer.write_record(row)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::output::create_path;

    #[test]
    fn test_paths() {
        let path = create_path(&["/tmp", "ccan-rs", "repo"]);
        println!("{}", path)
    }
}
