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

    // 引数・オプションに指定したパターンを取得
    let patterns: Vec<String> = match args.get_patterns(){
        Ok(pattern_list) => pattern_list.clone(),
        Err(e) => {
            eprintln!("{e}");
            return
        },
    };

    // 引数に指定したファイルを取得
    let files: &Vec<String> = match args.get_files() {
        Ok(file_list) => file_list,
        Err(e) => {
            eprintln!("{e}");
            return
        }
    };

    // マッチした行数を数えるための変数
    // -c オプションが指定されたときに使う
    let mut matching_count: i32 = 0;

    for file in files {
        // ファイルをオープンする
        let buf_reader: BufReader<File> = match open_file(file) {
            Ok(reader) => reader,
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        };

        // ファイルを1行ずつ read する
        for result in buf_reader.lines() {
            let line = match result {
                Ok(line) => line,
                Err(e) => {
                    eprint!("{}", FileError::FailedRead(e.to_string(), file.to_string()));
                    break
                }
            };

            // read した行を指定したパターンとマッチ
            for pattern in &patterns {
                match match_line(pattern.to_string(), line.to_owned(), args.ignore_case, args.invert_match) {
                    Ok(is_match) => {
                        if is_match {
                            matching_count += 1;
                            if !args.count { // -c が指定されたときに、println の処理を飛ばすため。
                                print(file.to_owned(), line, true);
                            }
                            // マッチした場合はループを抜ける。
                            // 1つのパターンとマッチした時点で、残りのパターンのマッチはしないため。
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

    // -c が true の場合、行数を表示する。
    if args.count {
        println!("{matching_count}");
    }
}

fn print(filename: String, line: String, is_filename: bool) {
    if is_filename {
        println!("{filename} : {line}");
    } else {
        println!("{line}")
    }
}
