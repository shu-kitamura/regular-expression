//! コマンドの位置引数・オプションを定義

use clap::{ArgAction, Parser};
use crate::error::CommandLineError;

#[derive(Debug, Parser)]
#[command(version)]
#[clap(disable_version_flag = true, disable_help_flag = true)]
pub struct Args {

    #[arg(value_name = "PATTERN")]
    /// パターンを指定する。
    pattern: Option<String>,

    #[arg(value_name = "FILE")]
    /// ファイルを指定する。
    files: Vec<String>,

    #[arg(short = 'e', long = "regexp", value_name = "PATTERN")]
    /// パターンを指定する。このオプションを使用すれば複数のパターンを指定することができる
    patterns : Vec<String>,

    #[arg(short = 'c', long = "count")]
    /// マッチした行数のみ表示する
    pub count: bool,

    #[arg(short = 'i', long = "ignore-case")]
    /// マッチしなかった行を表示する
    pub ignore_case: bool,
    
    #[arg(short = 'v', long = "invert-match")]
    /// マッチしなかった行を表示する
    pub invert_match: bool,

    #[arg(long, action = ArgAction::Help)]
    /// help を表示する
    help: Option<bool>,

    #[arg(short = 'V', long = "version", action = ArgAction::Version)]
    /// Version を表示する
    version: Option<bool>,
}

impl Args {
    // 位置引数と -e オプションに指定したパターンを1つの配列にして返す。
    // 位置引数と -e オプションのどちらにもパターンが指定されていない場合、エラーを返す。
    // & は、付けないと呼び出し時に所有権が移動するため、付けている。
    pub fn get_patterns(&mut self) -> Result<&Vec<String>, CommandLineError> {
        match &self.pattern {
            Some(p) => self.patterns.push(p.to_owned()),
            None => {}
        };

        if self.patterns.len() != 0 {
            Ok(&self.patterns)
        } else {
            Err(CommandLineError::NoPattern)
        }
    }

    // 位置引数に指定したファイルの配列を返す。
    // ファイルが指定されていない場合、エラーを返す。
    // & は、付けないと呼び出し時に所有権が移動するため、付けている。
    pub fn get_files(&self) -> Result<&Vec<String>, CommandLineError> {
        if self.files.len() != 0 {
            Ok(&self.files)
        } else {
            Err(CommandLineError::NoFile)
        }
    }
}