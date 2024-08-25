mod engine;
mod error;
mod cli;
mod fileread;

use std::{
    fs::File,
    io::{
        BufRead,
        BufReader,
        Stdin,
        stdin
    }
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
    let files: &Vec<String> = &args.files;

    let is_print_filename: bool = is_print_filename(files.len(), args.no_filename, args.with_filename);

    // マッチした行数を数えるための変数
    // -c オプションが指定されたときに使う
    let mut matching_count: i32 = 0;

    if files.is_empty() {
        let stdin: Stdin = stdin();
        let mut buf_reader: BufReader<Stdin> = BufReader::new(stdin);

        // 標準入力を1行ずつ read し、マッチングを実行する
        match match_file(
            &mut buf_reader,
            "",
            &patterns,
            args.ignore_case,
            args.invert_match,
            false,
            args.count
        ) {
            Some(c) => matching_count += c,
            None => {}
        }
    } else {
        for file in files {
            // ファイルをオープンする
            let mut buf_reader: BufReader<File> = match open_file(file) {
                Ok(reader) => reader,
                Err(e) => {
                    eprintln!("{e}");
                    continue;
                }
            };

            // ファイルを1行ずつ read し、マッチングを実行する
            match match_file(
                &mut buf_reader,
                file,
                &patterns,
                args.ignore_case,
                args.invert_match,
                is_print_filename,
                args.count
            ) {
                Some(c) => matching_count += c,
                None => {}
            };

        }
    }
    // -c が true の場合、行数を表示する。
    if args.count {
        println!("{matching_count}");
    }
}

/// ファイルもしくは、標準入力を1行ずつ read し、マッチングを実行する関数
fn match_file(
    buf_reader: &mut dyn BufRead,
    file: &str,
    patterns: &Vec<String>,
    ignore_case: bool,
    invert_match: bool,
    is_filename: bool,
    is_count: bool
) -> Option<i32> {
    let mut matching_count: i32 = 0;
    for result in buf_reader.lines() {
        let line = match result {
            Ok(line) => line,
            Err(e) => {
                eprint!("{}", FileError::FailedRead(e.to_string(), file.to_string()));
                break
            }
        };

        // read した行を指定したパターンとマッチ
        for pattern in patterns {
            match match_line(pattern.to_string(), line.to_owned(), ignore_case, invert_match) {
                Ok(is_match) => {
                    if is_match {
                        matching_count += 1;
                        if !is_count { // -c が指定されたときに、print の処理を飛ばすため。
                            print(file.to_owned(), line, is_filename);
                        }
                        // マッチした場合はループを抜ける。
                        // 1つのパターンとマッチした時点で、残りのパターンのマッチはしないため。
                        break
                    }
                },
                Err(e) => {
                    eprintln!("Following error is occured in matching, pattern = '{pattern}', line = '{line}'\n{e}");
                    return None
                }
            }
        }
    }

    Some(matching_count)
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

// ----- テストコード -----
#[test]
fn test_is_print_filename() {
    // ファイル数が 1 で、オプションなし
    assert_eq!(is_print_filename(1, false, false), false);
    // ファイル数が 1 で、-h オプションあり
    assert_eq!(is_print_filename(1, true, false), false);
    // ファイル数が 1 で、-H オプションあり
    assert_eq!(is_print_filename(1, false, true), true);
    // ファイル数が 2(≒ 2以上) で、オプションなし
    assert_eq!(is_print_filename(2, false, false), true);
    // ファイル数が 2(≒ 2以上) で、-h オプションあり
    assert_eq!(is_print_filename(2, true, false), false);
    // ファイル数が 2(≒ 2以上) で、-H オプションあり
    assert_eq!(is_print_filename(2, false, true), true);
}
