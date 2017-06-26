extern crate anidb;

use std::env;
use std::path;
use anidb::ed2k::Ed2kHash;

fn main () {
    let filename = env::args().nth(1).unwrap();
    let path = path::Path::new(&filename);
    let hex = Ed2kHash::from_file(&path).unwrap().hex;
    println!("{}", hex);
}
