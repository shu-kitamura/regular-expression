use thiserror::Error;

/// コマンドラインの指定に不正があった場合に出力するエラーの型
#[derive(Debug, Error, PartialEq)]
pub enum CommandLineError {
    #[error("CommandLineError : no pattern specified.")]
    NoPattern,
    #[error("CommandLineError : -h, -H options are specified at the same time.")]
    DuplicateFilenameOption,
}
