use clap::{ArgAction, Parser};
use regular_expression::Regex;
use std::{
    fs::File,
    io::{stdin, BufRead, BufReader, Stdin},
};
use thiserror::Error;

// 入力ファイルが stdin の場合、ファイル名を (standard input) とする。
// grep コマンドでパイプ使用時にファイル名を表示したら、(standard input)なるのでそれに合わせている。
const STDIN_FILENAME: &str = "(standard input)";

#[derive(Debug, Parser)]
#[command(version)]
#[clap(disable_version_flag = true, disable_help_flag = true)]
pub struct Args {
    #[arg(value_name = "PATTERN")]
    /// パターンを指定する。
    pattern: Option<String>,

    #[arg(value_name = "FILE")]
    /// ファイルを指定する。
    pub files: Vec<String>,

    #[arg(short = 'e', long = "regexp", value_name = "PATTERN")]
    /// パターンを指定する。このオプションを使用すれば複数のパターンを指定することができる
    patterns: Vec<String>,

    #[arg(short = 'c', long = "count")]
    /// マッチした行数のみ表示する
    pub count: bool,

    #[arg(short = 'i', long = "ignore-case")]
    /// 大文字と小文字を区別しない
    pub ignore_case: bool,

    #[arg(short = 'v', long = "invert-match")]
    /// マッチしなかった行を表示する
    pub invert_match: bool,

    #[arg(short = 'h', long = "no-filename")]
    /// 出力する行の前にファイル名を付けない。検索ファイルが1つの場合、こちらがデフォルト
    pub no_filename: bool,

    #[arg(short = 'H', long = "with-filename")]
    /// 出力する行の前にファイル名を付ける。検索ファイルが2つ以上の場合、こちらがデフォルト
    pub with_filename: bool,

    #[arg(short = 'n', long = "line-number")]
    /// 入力ファイル内での行番号を表示する
    pub line_number: bool,

    #[arg(long, action = ArgAction::Help)]
    /// help を表示する
    help: Option<bool>,

    #[arg(short = 'V', long = "version", action = ArgAction::Version)]
    /// Version を表示する
    version: Option<bool>,
}

impl Args {
    /// パターンの配列を取得して返す。  
    /// パターンは位置引数と -e オプションに指定ができるが、  
    /// -e オプションが指定されている場合、位置引数に指定した値はファイル名となる。  
    /// & は、付けないと呼び出し時に所有権が移動するため、付けている。
    pub fn get_patterns(&mut self) -> Result<&Vec<String>, CommandLineError> {
        if self.patterns.is_empty() {
            // -e オプションなしの場合、位置引数の値を patterns に挿入する
            match &self.pattern {
                Some(p) => self.patterns.push(p.to_owned()),
                None => return Err(CommandLineError::NoPattern),
            }
        } else {
            // -e オプションありの場合、位置引数の値を files に挿入する。
            if let Some(file) = &self.pattern {
                self.files.insert(0, file.to_owned())
            }
        }

        Ok(&self.patterns)
    }
}

/// コマンドラインの指定に不正があった場合に出力するエラーの型
#[derive(Debug, Error)]
pub enum CommandLineError {
    #[error("CommandLineError : no pattern specified.")]
    NoPattern,
    #[error("CommandLineError : -h, -H options are specified at the same time.")]
    DuplicateFilenameOption,
}

fn main() {
    let mut args: Args = Args::parse();

    // -h, -H が同時に指定されている場合、エラーを表示してプログラムを終了する（終了コード 1）
    if args.with_filename && args.no_filename {
        eprintln!("{}", CommandLineError::DuplicateFilenameOption);
        std::process::exit(1);
    }

    // 引数・オプションに指定したパターンを取得
    let patterns: Vec<String> = match args.get_patterns() {
        Ok(pattern_list) => pattern_list.clone(),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let regexes: Vec<Regex> = patterns
        .iter()
        .map(|p| {
            Regex::new(p, args.ignore_case, args.invert_match).unwrap_or_else(|e| {
                eprintln!("RegexError: {e}");
                std::process::exit(1);
            })
        })
        .collect();

    // マッチした行数を数えるための変数
    // -c オプションが指定されたときに使う
    let mut matching_count: usize = 0;

    if args.files.is_empty() {
        let stdin: Stdin = stdin();
        let mut buf_reader: BufReader<Stdin> = BufReader::new(stdin);

        // 標準入力を1行ずつ read し、マッチングを実行する
        if let Some(c) = match_file(&mut buf_reader, STDIN_FILENAME, &regexes, &args) {
            matching_count += c
        }
    } else {
        for file in &args.files {
            // ファイルをオープンする
            let mut buf_reader: BufReader<File> = match File::open(file) {
                Ok(reader) => BufReader::new(reader),
                Err(e) => {
                    eprintln!("{e}");
                    continue;
                }
            };

            // ファイルを1行ずつ read し、マッチングを実行する
            if let Some(c) = match_file(&mut buf_reader, file, &regexes, &args) {
                matching_count += c
            };
        }
    }
    // -c が true の場合、行数を表示する。
    if args.count {
        println!("{matching_count}");
    }
}

/// ファイルもしくは、標準入力を1行ずつ read し、マッチングを実行する関数
fn match_file<T: BufRead>(
    buf_reader: T,
    file: &str,
    regexes: &[Regex],
    args: &Args,
) -> Option<usize> {
    let is_filename = is_print_filename(args.files.len(), args.no_filename, args.with_filename);
    let is_count = args.count;
    let is_line_number = args.line_number;

    let mut matching_count: usize = 0;
    for (i, result) in buf_reader.lines().enumerate() {
        let line = match result {
            Ok(line) => line,
            Err(e) => {
                eprint!("{e}");
                break;
            }
        };

        // read した行を指定したパターンとマッチ
        for regex in regexes {
            match regex.is_match(&line) {
                Ok(true) => {
                    matching_count += 1;
                    if !is_count {
                        // -c が指定されたときに、print の処理を飛ばすため。
                        print(file, &line, i + 1, is_filename, is_line_number);
                    }
                    // マッチした場合はループを抜ける。
                    // 1つのパターンとマッチした時点で、残りのパターンのマッチはしないため。
                    break;
                }
                Ok(false) => continue,
                Err(e) => {
                    eprintln!("Following error is occured in matching.\n{e}");
                    return None;
                }
            }
        }
    }

    Some(matching_count)
}

/// 行を表示する関数
/// 以下の2点で処理が分岐するため、関数を分けている。
///
/// * 行数を表示する・しない  
/// * ファイル名を表示する・しない。
fn print(filename: &str, line: &str, line_number: usize, is_filename: bool, is_line_number: bool) {
    match (is_filename, is_line_number) {
        (true, true) => println!("{filename}:{line_number}:{line}"),
        (true, false) => println!("{filename}:{line}"),
        (false, true) => println!("{line_number}:{line}"),
        (false, false) => println!("{line}"),
    }
}

/// ファイル名を表示する・しないを判定するための関数  
/// ファイル数が 1 の場合、 -H オプションに従う。  
/// ファイル数が 2 以上の場合、 -h オプションに従う。  
fn is_print_filename(file_count: usize, no_filename: bool, with_filename: bool) -> bool {
    if file_count <= 1 {
        with_filename
    } else {
        !no_filename
    }
}

// ----- テストコード -----

#[cfg(test)]
mod tests {
    use crate::is_print_filename;

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
}
