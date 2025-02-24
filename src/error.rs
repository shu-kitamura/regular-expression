//! このソフトウェアで使用するエラーの型を定義

use std::{
    error::Error,
    fmt::{self, Display},
};

/// パースエラーを表す型
/// 
/// 正規表現パターンの解析（パース）中に発生するエラーを表現する
/// 各エラーケースは、入力されたパターンのどの部分でどのような問題があったかを示すために、
/// 位置情報や不正な文字などの補足情報を含む。
#[derive(Debug, PartialEq)]
pub enum ParseError {
    InvalidEscape(usize, char), // 誤ったエスケープシーケンスが入力された場合
    InvalidRightParen(usize),   // ')' に対応する '(' が存在しない場合
    NoPrev(usize),              // '+', '*', '?', '|' の前に式がない場合
    NoRightParen,               // ')' が存在しない場合
    Empty,                      // 空のパターンが入力された場合
}

/// ParseErrorを表示するため、Displayトレイトを実装
impl Display for ParseError {
    fn fmt (&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidEscape(pos, c) => {
                write!(f, "ParseError: invalid escape : position = {pos}, character = '{c}'")
            }
            ParseError::InvalidRightParen(pos) => {
                write!(f, "ParseError: invalid right parenthesis : position = {pos}")
            }
            ParseError::NoPrev(pos) => {
                write!(f, "ParseError: no prevous expression : position = {pos}")
            }
            ParseError::NoRightParen => {
                write!(f, "ParseError: No right parenthesis")
            }
            ParseError::Empty => {
                write!(f, "ParseError: empty expression")
            }
        }
    }
}

/// エラー用にErrorトレイトを実装
impl Error for ParseError {} // デフォルト実装を使うだけの場合、これだけでいい


/// コンパイルエラーを示す型
/// 
/// AST から命令コードへの変換（コンパイル）時に発生するエラーを表現する。
/// 命令の生成中にリソースがオーバーフローした場合や、特定の演算子の変換に失敗した場合に使用される。
#[derive(Debug, PartialEq)]
pub enum CompileError {
    PCOverFlow,   // コンパイルにオーバーフローが起きた場合のエラー
    FailStar,     // * のコンパイルエラー
    FailQuestion, // ? のコンパイルエラー
    FailOr,       // | のコンパイルエラー
}

/// CompileErrorを表示するため、Displayトレイトを実装
impl Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CompileError: {:?}", self)
    }
}

impl Error for CompileError {}

/// コード評価時のエラーを表す型
///
/// コンパイルされた命令コードを実行する際に発生するエラーを表現する。
/// 具体的には、プログラムカウンタ (PC) や Char の Index のオーバーフロー、
/// 不正な命令ポインタの参照などが含まれます。
#[derive(Debug, PartialEq)]
pub enum EvalError {
    PCOverFlow,        // PC がオーバーフローした場合のエラー
    CharIndexOverFlow, // Char の Index がオーバーフローした場合のエラー
    InvalidPC,         // 不正な PC が指定された場合のエラー
}

/// EvalErrorを表示するため、Displayトレイトを実装
impl Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EvalError: {:?}", self)
    }
}

impl Error for EvalError {}


/// engine.rs で使用する3種類のエラー(Parse, Compile, Eval)を統合するための型
#[derive(Debug, PartialEq)]
pub enum RegexError {
    CompileError(CompileError),
    EvalError(EvalError),
    ParseError(ParseError),    
}

impl Error for RegexError {}

/// RegexErrorを表示するため、Displayトレイトを実装
impl Display for RegexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegexError::CompileError(e) => write!(f, "{e}"),
            RegexError::EvalError(e) => write!(f, "{e}"),
            RegexError::ParseError(e) => write!(f, "{e}")
        }
    }
}

// 各種エラー型間の変換を可能にするため、From トレイトを実装
impl From<EvalError> for RegexError {
    fn from(value: EvalError) -> Self {
        RegexError::EvalError(value)
    }
}

impl From<CompileError> for RegexError {
    fn from(value: CompileError) -> Self {
        RegexError::CompileError(value)
    }
}

impl From<ParseError> for RegexError {
    fn from(value: ParseError) -> Self {
        RegexError::ParseError(value)
    }
}
