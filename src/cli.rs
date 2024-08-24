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
    /// 大文字と小文字を区別しない
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
    /// パターンの配列を取得して返す。  
    /// パターンは位置引数と -e オプションに指定ができるが、  
    /// -e オプションが指定されている場合、位置引数に指定した値はファイル名となる。  
    /// & は、付けないと呼び出し時に所有権が移動するため、付けている。
    pub fn get_patterns(&mut self) -> Result<&Vec<String>, CommandLineError> {
        if self.patterns.is_empty() { // -e オプションなしの場合、位置引数の値を patterns に挿入する
            match &self.pattern {
                Some(p) => self.patterns.push(p.to_owned()),
                None => return Err(CommandLineError::NoPattern)
            }
        } else { // -e オプションありの場合、位置引数の値を files に挿入する。
            match &self.pattern {
                Some(file) => self.files.insert(0, file.to_owned()),
                None => return Err(CommandLineError::NoFile)
            }
        }

        Ok(&self.patterns)
    }
    
    /// 位置引数に指定したファイルの配列を返す。
    /// ファイルが指定されていない場合、エラーを返す。
    /// & は、付けないと呼び出し時に所有権が移動するため、付けている。
    pub fn get_files(&self) -> Result<&Vec<String>, CommandLineError> {
        if !self.files.is_empty() {
            Ok(&self.files)
        } else {
            Err(CommandLineError::NoFile)
        }
    }
}