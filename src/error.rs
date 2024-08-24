//! このソフトウェアで使用するエラーの型を定義

use std::{
    error::Error,
    fmt::{self, Display},
};

/// パースエラーを表す型
#[derive(Debug, PartialEq)]
pub enum ParseError {
    InvalidEscape(usize, char), // 誤ったエスケープシーケンス
    InvalidRightParen(usize),   // 開きカッコ無し
    NoPrev(usize),              // +,*,?,| の前に式がない
    NoRightParen,               // 閉じカッコ無し
    Empty,                      // 空のパターン
}

/// ParseErrorを表示するため、Displayトレイトを実装
impl Display for ParseError {
    fn fmt (&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidEscape(pos, c) => {
                write!(f, "ParseError : invalid escape : position = {pos}, character = '{c}'")
            }
            ParseError::InvalidRightParen(pos) => {
                write!(f, "ParseError : invalid right parenthesis : position = {pos}")
            }
            ParseError::NoPrev(pos) => {
                write!(f, "ParseError : no prevous expression : position = {pos}")
            }
            ParseError::NoRightParen => {
                write!(f, "ParseError : No right parenthesis")
            }
            ParseError::Empty => {
                write!(f, "ParseError : empty expression")
            }
        }
    }
}

/// エラー用にErrorトレイトを実装
impl Error for ParseError {} // デフォルト実装を使うだけの場合、これだけでいい


/// コード生成エラーを示す型
#[derive(Debug, PartialEq)]
pub enum CodeGenError {
    PCOverFlow,   // コード生成中にオーバーフローが起きた場合のエラー
    FailStar,     // * のコード生成エラー
    FailOr,       // | のコード生成エラー
    FailQuestion, // ? のコード生成エラー
}

/// CodeGenErrorを表示するため、Displayトレイトを実装
impl Display for CodeGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CodeGenError: {:?}", self)
    }
}

impl Error for CodeGenError {}

/// コード評価時のエラーを表す型
#[derive(Debug, PartialEq)]
pub enum EvalError {
    PCOverFlow,     // PC がオーバーフローした場合のエラー
    CharIndexOverFlow,     // SP がオーバーフローした場合のエラー
    InvalidPC,
}

/// EvalErrorを表示するため、Displayトレイトを実装
impl Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EvalError: {:?}", self)
    }
}

impl Error for EvalError {}


/// engine.rs で使用する3種類のエラー(Parse, CodeGen, Eval)を扱うための型
#[derive(Debug, PartialEq)]
pub enum RegexEngineError {
    CodeGenError(CodeGenError),
    EvalError(EvalError),
    ParseError(ParseError),    
}

impl Error for RegexEngineError {}

/// RegexEngineErrorを表示するため、Displayトレイトを実装
impl Display for RegexEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegexEngineError::CodeGenError(e) => write!(f, "{e}"),
            RegexEngineError::EvalError(e) => write!(f, "{e}"),
            RegexEngineError::ParseError(e) => write!(f, "{e}")
        }
    }
}

/// コマンドラインの指定に不正があった場合に出力するエラーの型
#[derive(Debug)]
pub enum CommandLineError {
    NoPattern,
    NoFile,
}

/// CommandLineErrorを表示するため、Displayトレイトを実装
impl Display for CommandLineError {
    fn fmt (&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandLineError::NoPattern => write!(f, "CommandLineError : No pattern specified."),
            CommandLineError::NoFile => write!(f, "CommandLineError : No file specified.")
        }
    }
}

impl Error for CommandLineError {}

/// ファイルを read する処理で使用するエラーの型
#[derive(Debug, PartialEq)]
pub enum FileError {
    FailedOpen(String, String),
    FailedRead(String, String),
}

/// FileReadErrorを表示するため、Displayトレイトを実装
impl Display for FileError {
    fn fmt (&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::FailedOpen(msg, filename) => write!(f, "FileError : Failed to open '{filename}' with following message.\n{msg}."),
            FileError::FailedRead(msg, filename) => write!(f, "FileError : Failed to read '{filename}' with following message.\n{msg}.")
        }
    }
}