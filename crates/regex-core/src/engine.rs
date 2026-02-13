//! Core functions for compiling and matching regex patterns.
mod ast;
mod compiler;
mod evaluator;
mod instruction;
mod parser;

use thiserror::Error;

use crate::engine::{compiler::compile, evaluator::eval, parser::parse};

pub(crate) use ast::{Ast, extract_must_literals};
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

/// Parse, extract must literals, and compile a pattern.
pub(crate) fn compile_pattern_with_must_literals(
    pattern: &str,
) -> Result<(Vec<Instruction>, Vec<String>), RegexError> {
    let ast: Ast = parse(pattern)?;
    let must_literals = extract_must_literals(&ast);
    let instructions = compile(&ast)?;
    Ok((instructions, must_literals))
}

/// Match an instruction sequence against a line.
pub fn match_line(code: &[Instruction], line: &str) -> Result<bool, RegexError> {
    Ok(eval(code, line)?)
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        CompileError, RegexError, compile_pattern_with_must_literals, instruction::Instruction,
        match_line,
    };

    #[test]
    fn test_compile_pattern_literal() {
        let (code, _) = compile_pattern_with_must_literals("abc").unwrap();
        assert_eq!(code.len(), 4);
        assert!(matches!(code.last(), Some(Instruction::Match)));
    }

    #[test]
    fn test_compile_pattern_invalid_backreference() {
        let actual = compile_pattern_with_must_literals("(a)\\2");
        assert_eq!(
            actual,
            Err(RegexError::Compile(CompileError::InvalidBackreference(2)))
        );
    }

    #[test]
    fn test_match_line_backreference() {
        let (code, _) = compile_pattern_with_must_literals("(abc)\\1").unwrap();
        assert!(match_line(&code, "abcabc").unwrap());
        assert!(!match_line(&code, "abcabd").unwrap());
    }

    #[test]
    fn test_compile_pattern_with_must_literals() {
        let (_code, must_literals) = compile_pattern_with_must_literals(".*abc.*").unwrap();
        assert_eq!(must_literals, vec!["abc".to_string()]);
    }
}
