//! Core functions for compiling and matching regex patterns.
mod ast;
mod compiler;
mod evaluator;
mod instruction;
mod parser;

use thiserror::Error;

use crate::engine::{
    compiler::compile,
    evaluator::{eval, eval_from_start},
    parser::parse,
};

pub use compiler::CompileError;
pub use evaluator::EvalError;
pub use instruction::Instruction;
pub use parser::ParseError;

/// Unified error type for parse, compile, and evaluation stages.
#[derive(Debug, Error, PartialEq)]
pub enum RegexError {
    /// Parsing failed.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Compilation failed.
    #[error(transparent)]
    Compile(#[from] CompileError),
    /// Runtime matching failed.
    #[error(transparent)]
    Eval(#[from] EvalError),
}

/// Trait for checked addition used to avoid overflow.
pub trait SafeAdd: Sized {
    fn safe_add(&self, n: &Self) -> Option<Self>;
}

/// `SafeAdd` implementation for `usize`.
impl SafeAdd for usize {
    fn safe_add(&self, n: &Self) -> Option<Self> {
        self.checked_add(*n)
    }
}

/// Adds `src` into `dst` using checked arithmetic.
///
/// Returns the error produced by `f` when the operation overflows.
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

/// Parse and compile a pattern.
pub fn compile_pattern(pattern: &str) -> Result<Vec<Instruction>, RegexError> {
    let ast = parse(pattern)?;
    let instructions = compile(&ast)?;
    Ok(instructions)
}

/// Match an instruction sequence against a line.
pub fn match_line(code: &[Instruction], line: &str) -> Result<bool, RegexError> {
    Ok(eval(code, line)?)
}

/// Match an instruction sequence from the start of a line.
pub fn match_line_from_start(code: &[Instruction], line: &str) -> Result<bool, RegexError> {
    Ok(eval_from_start(code, line)?)
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        CompileError, RegexError, compile_pattern, instruction::Instruction, match_line,
        match_line_from_start,
    };

    #[test]
    fn test_compile_pattern_literal() {
        let code = compile_pattern("abc").unwrap();
        assert_eq!(code.len(), 4);
        assert!(matches!(code.last(), Some(Instruction::Match)));
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
