use std::fs::File;
use std::path::Path;

use anyhow::{anyhow, Result};
use csv::WriterBuilder;
use ndarray::Array2;
use ndarray_csv::Array2Writer;
use serde::Serialize;


pub fn p(dir: &String, file: &str) -> Result<String> {
   Path::new(dir).join(file).as_path().to_str()
       .map(String::from)
       .ok_or(anyhow!("cannot create path in directory {}", dir))
}
pub fn write_matrix<A: Serialize>(path: &String, matrix: &Array2<A>) -> Result<()>{
    let file = File::create(path)?;
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .from_writer(file);
    Ok(writer.serialize_array2(matrix)?)
}

pub fn write_arr<A: Serialize>(path: &String, matrix: &Vec<A>) -> Result<()>{
    let file = File::create(path)?;
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .from_writer(file);
    Ok(writer.serialize(matrix)?)
}
