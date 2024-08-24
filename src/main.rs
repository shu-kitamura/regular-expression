mod engine;
mod error;
mod cli;
mod fileread;

use std::{
    fs::File,
    io::{BufRead, BufReader}
};
use clap::Parser;
use crate::{
    cli::Args,
    engine::match_line,
    error::FileError,
    fileread::open_file,
};

fn main() {
    let mut args: Args = Args::parse();
    let patterns = match args.get_patterns(){
        Ok(pattern_list) => pattern_list.clone(),
        Err(e) => {
            eprintln!("{e}");
            return
        },
    };

    let files: &Vec<String> = match args.get_files() {
        Ok(file_list) => file_list,
        Err(e) => {
            eprintln!("{e}");
            return
        }
    };

    for file in files {
    
        let buf_reader: BufReader<File> = match open_file(file) {
            Ok(reader) => reader,
            Err(e) => {
                eprintln!("{e}");
                return
            }
        };

        for result in buf_reader.lines() {
            let line = match result {
                Ok(line) => line,
                Err(e) => {
                    eprint!("{}", FileError::FailedRead(e.to_string()));
                    return
                }
            };

            for pattern in &patterns {
                match match_line(pattern.to_string(), line.to_owned(), args.ignore_case, args.invert_match) {
                    Ok(is_match) => {
                        if is_match {
                            println!("{line}");
                            break
                        }
                    },
                    Err(e) => {
                        eprintln!("Following error is occured in matching, pattern = '{pattern}', line = '{line}'\n{e}");
                        return
                    }
                }
            }
        }
    }
}
