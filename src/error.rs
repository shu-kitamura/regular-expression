//! このソフトウェアで使用するエラーの型を定義

use thiserror::Error;

/// パースエラーを表す型
/// 
/// 正規表現パターンの解析（パース）中に発生するエラーを表現する
/// 各エラーケースは、入力されたパターンのどの部分でどのような問題があったかを示すために、
/// 位置情報や不正な文字などの補足情報を含む。
#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("ParseError: invalid escape : position = {0}, character = '{1}'")]
    InvalidEscape(usize, char),
    #[error("ParseError: invalid right parenthesis : position = {0}")]
    InvalidRightParen(usize),
    #[error("ParseError: no previous expression : position = {0}")]
    NoPrev(usize),
    #[error("ParseError: No right parenthesis")]
    NoRightParen,
    #[error("ParseError: empty expression")]
    Empty,
}

/// コンパイルエラーを示す型
/// 
/// AST から命令コードへの変換（コンパイル）時に発生するエラーを表現する。
/// 命令の生成中にリソースがオーバーフローした場合や、特定の演算子の変換に失敗した場合に使用される。
#[derive(Debug, Error, PartialEq)]
pub enum CompileError {
    #[error("CompileError: PCOverFlow")]
    PCOverFlow,
    #[error("CompileError: FailStar")]
    FailStar,
    #[error("CompileError: FailQuestion")]
    FailQuestion,
    #[error("CompileError: FailOr")]
    FailOr,
}

/// コード評価時のエラーを表す型
///
/// コンパイルされた命令コードを実行する際に発生するエラーを表現する。
/// 具体的には、プログラムカウンタ (PC) や Char の Index のオーバーフロー、
/// 不正な命令ポインタの参照などが含まれます。
#[derive(Debug, Error, PartialEq)]
pub enum EvalError {
    #[error("EvalError: PCOverFlow")]
    PCOverFlow,
    #[error("EvalError: CharIndexOverFlow")]
    CharIndexOverFlow,
    #[error("EvalError: InvalidPC")]
    InvalidPC,
}

/// engine.rs で使用する3種類のエラー(Parse, Compile, Eval)を統合するための型
#[derive(Debug, Error, PartialEq)]
pub enum RegexError {
    #[error(transparent)]
    CompileError(#[from] CompileError),
    #[error(transparent)]
    EvalError(#[from] EvalError),
    #[error(transparent)]
    ParseError(#[from] ParseError),
}
