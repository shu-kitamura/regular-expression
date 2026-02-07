//! マッチングを行う関数を定義
mod ast;
mod compiler_v2;
mod evaluator_v2;
mod instruction_v2;
mod parser_v2;

use thiserror::Error;

use crate::engine::{
    compiler_v2::compile_v2,
    evaluator_v2::{eval_v2, eval_v2_from_start},
    parser_v2::parse,
};

pub use compiler_v2::CompileV2Error as CompileError;
pub use evaluator_v2::EvalV2Error as EvalError;
pub use instruction_v2::InstructionV2;
pub use parser_v2::ParseError;

#[derive(Debug, Error, PartialEq)]
pub enum RegexError {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Compile(#[from] CompileError),
    #[error(transparent)]
    Eval(#[from] EvalError),
}

/// オーバーフロー対策のトレイトを定義
pub trait SafeAdd: Sized {
    fn safe_add(&self, n: &Self) -> Option<Self>;
}

/// SafeAdd トレイトを実装
impl SafeAdd for usize {
    fn safe_add(&self, n: &Self) -> Option<Self> {
        self.checked_add(*n)
    }
}

pub fn safe_add<T, F, E>(dst: &mut T, src: &T, f: F) -> Result<(), E>
where
    T: SafeAdd,
    F: Fn() -> E,
{
    if let Some(n) = dst.safe_add(src) {
        *dst = n;
        Ok(())
    } else {
        Err(f())
    }
}

/// v2 パターンをパースしてコンパイルする。
pub fn compile_pattern(pattern: &str) -> Result<Vec<InstructionV2>, RegexError> {
    let ast = parse(pattern)?;
    let instructions = compile_v2(&ast)?;
    Ok(instructions)
}

/// v2 命令列と文字列のマッチングを実行する。
pub fn match_line(code: &[InstructionV2], line: &str) -> Result<bool, RegexError> {
    Ok(eval_v2(code, line)?)
}

/// v2 命令列で文字列先頭からのマッチングを実行する。
pub fn match_line_from_start(code: &[InstructionV2], line: &str) -> Result<bool, RegexError> {
    Ok(eval_v2_from_start(code, line)?)
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        CompileError, RegexError, compile_pattern, instruction_v2::InstructionV2, match_line,
        match_line_from_start,
    };

    #[test]
    fn test_compile_pattern_literal() {
        let code = compile_pattern("abc").unwrap();
        assert_eq!(code.len(), 4);
        assert!(matches!(code.last(), Some(InstructionV2::Match)));
    }

    #[test]
    fn test_compile_pattern_invalid_backreference() {
        let actual = compile_pattern("(a)\\2");
        assert_eq!(
            actual,
            Err(RegexError::Compile(CompileError::InvalidBackreference(2)))
        );
    }

    #[test]
    fn test_match_line_backreference() {
        let code = compile_pattern("(abc)\\1").unwrap();
        assert!(match_line(&code, "abcabc").unwrap());
        assert!(!match_line(&code, "abcabd").unwrap());
    }

    #[test]
    fn test_match_line_from_start() {
        let code = compile_pattern("abc").unwrap();
        assert!(match_line_from_start(&code, "abcdef").unwrap());
        assert!(!match_line_from_start(&code, "zabc").unwrap());
    }
}
