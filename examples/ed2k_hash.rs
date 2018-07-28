extern crate anidb;

use anidb::ed2k::Ed2kHash;
use std::env;
use std::path;

fn main() {
    let filename = env::args().nth(1).unwrap();
    let path = path::Path::new(&filename);
    let hex = Ed2kHash::from_file(&path).unwrap().hex;
    println!("{}", hex);
}
