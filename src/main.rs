mod engine;
mod error;
mod cli;
mod fileread;

use std::{fs::File, io::BufRead};

use clap::Parser;
use crate::{
    cli::Args,
    engine::match_line,
    error::FileError,
    fileread::open_file,
};

fn main() {
    let mut args: Args = Args::parse();
    let mut patterns = match args.get_patterns(){
        Ok(pattern_list) => pattern_list.clone(),
        Err(e) => {
            eprintln!("{e}");
            return
        },
    };

    println!("{:?}", patterns);

    for i in 0..patterns.len() {
        if args.ignore_case {
            patterns[i] = patterns[i].to_lowercase();
        }
    };

    println!("{:?}", patterns);

    let files = match args.get_files() {
        Ok(file_list) => file_list,
        Err(e) => {
            eprintln!("{e}");
            return
        }
    };
    println!("{:?}", files);

    let is_count = args.count;

    let ms = "ABCD".to_string();
    ignore_case(ms);

    let buf_reader = match open_file("a"){
        Ok(reader) => reader,
        Err(e) => eprintln!("{e}"),
    };
    for result in buf_reader.lines() {
        match result {
            Ok(line) => println!("{line}"),
            Err(e) => eprint!("{}", FileError::FailedRead(e.to_string()))
        }
    }
}

fn ignore_case(s:String) -> String {
    println!("{} : {}", s, s.to_lowercase());

    s.to_lowercase()
}
