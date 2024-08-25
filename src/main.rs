mod engine;
mod error;
mod cli;
mod fileread;

use std::{
    fs::File,
    io::{BufRead, BufReader}
};
use clap::Parser;
use error::CommandLineError;
use crate::{
    cli::Args,
    engine::match_line,
    error::FileError,
    fileread::open_file,
};

fn main() {
    let mut args: Args = Args::parse();

    // -h, -H が同時に指定されている場合、エラーを表示して return する
    if args.with_filename && args.no_filename {
        eprintln!("{}", CommandLineError::DuplicateFilenameOption);
        return
    }

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

    let is_print_filename: bool = is_print_filename(files.len(), args.no_filename, args.with_filename);

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
                            if !args.count { // -c が指定されたときに、print の処理を飛ばすため。
                                print(file.to_owned(), line, is_print_filename);
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

/// 行を表示する関数  
/// ファイル名を表示する・しないで処理が分岐するため、関数を分けた。
fn print(filename: String, line: String, is_filename: bool) {
    if is_filename {
        println!("{filename} : {line}");
    } else {
        println!("{line}")
    }
}

/// ファイル名を表示する・しないを判定するための関数  
/// ファイル数が 1 の場合、 -H オプションに従う。  
/// ファイル数が 2 以上の場合、 -h オプションに従う。  
fn is_print_filename(file_count: usize, no_filename: bool, with_filename: bool) -> bool {
    if file_count == 1 {
        with_filename
    } else {
        !no_filename
    }
}
