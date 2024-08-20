//! エラーの型を定義する

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

impl Display for CodeGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CodeGenError: {:?}", self)
    }
}

impl Error for CodeGenError {}