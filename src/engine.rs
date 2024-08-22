pub mod evaluator;
pub mod parser;
pub mod instruction;
pub mod codegen;

use codegen::get_code;
use evaluator::eval;
use parser::parse;

pub fn do_match(pattern:&str, line: &str) -> bool {
    let ast = match parse(pattern) {
        Ok(res) => res,
        Err(e) => return false
    };

    let code = match get_code(&ast) {
        Ok(instructions) => instructions,
        Err(e) => return false
    };

    let charcters: Vec<char> = line.chars().collect();
    let match_result = match eval(&code, &charcters) {
        Ok(res) => res,
        Err(e) => return false
    };

    match_result
}

#[test]
fn test_do_match_true() {
    let actual: bool = do_match("ab(c|d)", "abc");
    assert_eq!(actual, true);
}

#[test]
fn test_do_match_false() {
    let actual: bool = do_match("ab(c|d)", "abx");
    assert_eq!(actual, false);
}