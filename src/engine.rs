pub mod codegen;
pub mod evaluator;
pub mod helper;
pub mod instruction;
pub mod parser;

use crate::{
    error::RegexError,
    engine::{
        codegen::get_code,
        evaluator::eval,
        instruction::Instruction,
        parser::parse,
    }
};

fn match_string(insts: &Vec<Instruction>, string: &str) -> Result<bool, RegexError> {
    let charcters: Vec<char> = string.chars().collect();
    let match_result: bool = match eval(&insts, &charcters) {
        Ok(res) => res,
        Err(e) => return Err(RegexError::EvalError(e))
    };

    Ok(match_result)
}

pub fn match_line(pattern: &str, line: &str) -> Result<bool, RegexError> {
    let ast: parser::AST = match parse(pattern) {
        Ok(res) => res,
        Err(e) => return Err(RegexError::ParseError(e)),
    };

    let code: Vec<Instruction> = match get_code(&ast) {
        Ok(instructions) => instructions,
        Err(e) => return Err(RegexError::CodeGenError(e)),
    };

    for (i, _) in line.char_indices() {
        let is_match: bool = match match_string(&code, &line[i..]) {
            Ok(res) => res,
            Err(e) => return Err(e),
        };

        if is_match {
            println!("{line}");
            return Ok(true)
        }
    }
    Ok(false)
}

#[test]
fn test_match_string_true() {
    let insts: Vec<Instruction> = vec![
        Instruction::Char('a'),
        Instruction::Char('b'),
        Instruction::Split(3, 5),
        Instruction::Char('c'),
        Instruction::Jump(6),
        Instruction::Char('d'),
        Instruction::Match
    ];
    let actual: bool = match_string(&insts, "abc").unwrap();
    assert_eq!(actual, true);
}

#[test]
fn test_match_string_false() {
    let insts: Vec<Instruction> = vec![
        Instruction::Char('a'),
        Instruction::Char('b'),
        Instruction::Split(3, 5),
        Instruction::Char('c'),
        Instruction::Jump(6),
        Instruction::Char('d'),
        Instruction::Match
    ];
    let actual: bool = match_string(&insts, "abx").unwrap();
    assert_eq!(actual, false);
}

#[test]
fn test_match_string_eval_error() {
    use super::error::EvalError;

    let insts: Vec<Instruction> = vec![
        Instruction::Char('a'),
        Instruction::Char('b'),
        Instruction::Split(100, 200),
        Instruction::Char('c'),
        Instruction::Jump(6),
        Instruction::Char('d'),
        Instruction::Match
    ];

    let actual = match_string(&insts, "abc");    
    assert_eq!(actual, Err(RegexError::EvalError(EvalError::InvalidPC)));
}

#[test]
fn test_match_line_true() {
    let actual: bool = match_line("ab*(c|d)", "xorabbbbd").unwrap();
    assert_eq!(actual, true);
}

#[test]
fn test_match_line_false() {
    let actual: bool = match_line("ab*(c|d)", "abbbbxccd").unwrap();
    assert_eq!(actual, false);
}

#[test]
fn test_match_line_parse_error() {
    use super::error::ParseError;

    let actual = match_line("ab(c|d", "a");
    assert_eq!(actual, Err(RegexError::ParseError(ParseError::NoRightParen)));
}