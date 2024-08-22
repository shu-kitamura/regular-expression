pub mod evaluator;
pub mod parser;
pub mod instruction;
pub mod codegen;

use super::error::RegexError;
use codegen::get_code;
use evaluator::eval;
use parser::parse;

pub fn do_match(pattern:&str, line: &str) -> Result<bool, RegexError> {
    let ast: parser::AST = match parse(pattern) {
        Ok(res) => res,
        Err(e) => return Err(RegexError::ParseError(e)),
    };

    let code: Vec<instruction::Instruction> = match get_code(&ast) {
        Ok(instructions) => instructions,
        Err(e) => return Err(RegexError::CodeGenError(e)),
    };

    let charcters: Vec<char> = line.chars().collect();
    let match_result: bool = match eval(&code, &charcters) {
        Ok(res) => res,
        Err(e) => return Err(RegexError::EvalError(e))
    };

    Ok(match_result)
}

#[test]
fn test_do_match_true() {
    let actual: bool = do_match("ab(c|d)", "abc").unwrap();
    assert_eq!(actual, true);
}

#[test]
fn test_do_match_false() {
    let actual: bool = do_match("ab(c|d)", "abx").unwrap();
    assert_eq!(actual, false);
}

#[test]
fn test_do_match_parse_error() {
    use super::error::ParseError;

    let actual = do_match("ab(c|d", "abc");    
    assert_eq!(actual, Err(RegexError::ParseError(ParseError::NoRightParen)));
}