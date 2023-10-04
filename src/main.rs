use std::fs::File;
use std::time::Instant;
use csv::WriterBuilder;
use git2::Repository;
use ndarray_csv::Array2Writer;
use crate::git::SimpleGit;
use crate::ccan::CoChanges;

mod git;
mod ccan;

fn main() {
    let repo = Repository::open("/tmp/microservices-demo").unwrap();
    let branch = "main";
    let start = Instant::now();
    let diffs = repo.diffs(branch).expect("cannot get diffs");
    let mut cc = CoChanges::from_diffs(diffs);
    cc.calculate_cc_freq(3);
    let end = Instant::now();
    let stop = end - start;
    println!("Freq calc took {}ms", stop.as_millis());
    println!("Freq dim {:?}", cc.cc_freq.as_ref().unwrap().matrix.dim());
    cc.calculate_cc_prob();
    let end = Instant::now();
    let stop = end - start;
    println!("Prob calc took {}ms", stop.as_millis());

    {
        let file = File::create("cc-freq.csv").expect("cannot create file");
        let mut writer = WriterBuilder::new()
            .has_headers(false)
            .from_writer(file);
        writer.serialize_array2(&cc.cc_freq.as_ref().unwrap().matrix).expect("cannot write file");
    }

    {
        let file = File::create("cc-probs.csv").expect("cannot create file");
        let mut writer = WriterBuilder::new()
            .has_headers(false)
            .from_writer(file);
        writer.serialize_array2(&cc.cc_prob.as_ref().unwrap().matrix).expect("cannot write file");
    }

    {
        let file = File::create("cc-files.csv").expect("cannot create file");
        let mut writer = WriterBuilder::new()
            .has_headers(false)
            .from_writer(file);
        writer.serialize(&cc.cc_prob.as_ref().unwrap().row_names).expect("cannot write file");
    }
}
