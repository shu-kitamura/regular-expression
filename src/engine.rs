//! マッチングを行う関数を定義

pub mod codegen;
pub mod evaluator;
pub mod helper;
pub mod instruction;
pub mod parser;

use crate::{
    error::RegexEngineError,
    engine::{
        codegen::get_code,
        evaluator::eval,
        instruction::Instruction,
        parser::{AST, parse},
    }
};

/// 文字列のマッチングを実行する。
fn match_string(insts: &Vec<Instruction>, string: &str) -> Result<bool, RegexEngineError> {
    let charcters: Vec<char> = string.chars().collect();
    let match_result: bool = match eval(&insts, &charcters) {
        Ok(res) => res,
        Err(e) => return Err(RegexEngineError::EvalError(e))
    };

    Ok(match_result)
}

/// パターンと文字列のマッチングを実行する
/// 
/// # 引数
/// 
/// * pattern -> 正規表現のパターン
/// * line -> マッチング対象の文字列
/// * is_ignore_case -> 大小文字の区別をするかどうか。-c オプションのために使用
/// * is_invert_match -> 結果を逆にする(マッチ成功時に false、失敗時に true)。-v オプションのために使用
/// 
/// # 返り値
/// 
/// エラーなく実行でき、マッチングに成功した場合 Ok(true) を返す。  
/// エラーなく実行でき、マッチングに失敗した場合 Ok(false) を返す。  
/// ※ -v オプションが指定されている場合は true/false が反対になる。  
/// 
/// エラーが発生した場合 Err を返す。
pub fn match_line(mut pattern: String, mut line: String, is_ignore_case: bool, is_invert_match: bool) -> Result<bool, RegexEngineError> {
    // -i が指定された場合の処理
    // パターンと行を小文字にすることで、区別をしないようにする
    if is_ignore_case {
        pattern = pattern.to_lowercase();
        line = line.to_lowercase();
    }

    // パターンから AST を生成する。
    let ast: AST = match parse(pattern.as_str()) {
        Ok(res) => res,
        Err(e) => return Err(RegexEngineError::ParseError(e)),
    };

    // AST から コード(Instructionの配列)を生成する。
    let code: Vec<Instruction> = match get_code(&ast) {
        Ok(instructions) => instructions,
        Err(e) => return Err(RegexEngineError::CodeGenError(e)),
    };

    for (i, _) in line.char_indices() {
        // abcdefg という文字列の場合、以下のように順にマッチングする。
        //     ループ1 : abcdefg
        //     ループ2 : bcdefg
        //     ・・・
        //     ループN : g
        let is_match: bool = match match_string(&code, &line[i..]) {
            Ok(res) => res,
            Err(e) => return Err(e),
        };

        // マッチングが成功した場合の処理
        if is_match {
            // -v が指定されている場合は false、指定されていない場合は true を返す 
            if is_invert_match {
                return Ok(false)
            } else {
                return Ok(true)
            }
        }
    }

    // for文を抜ける = マッチングが失敗する のため、
    // -v が指定されている場合は true、指定されていない場合は false を返す
    if is_invert_match {
        Ok(true)
    } else {
        Ok(false)
    }
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
    assert_eq!(actual, Err(RegexEngineError::EvalError(EvalError::InvalidPC)));
}

#[test]
fn test_match_line_true() {
    let actual: bool = match_line(
        "ab*(c|d)".to_string(),
        "xorabbbbd".to_string(),
        false,
        false
    ).unwrap();
    assert_eq!(actual, true);
}

#[test]
fn test_match_line_false() {
    let actual: bool = match_line(
        "Ab*(c|d)".to_string(),
        "abbbbxccd".to_string(),
        true,
        false,
    ).unwrap();
    assert_eq!(actual, false);
}

#[test]
fn test_match_invert() {
    let actual: bool = match_line(
        "Ab*(c|d)".to_string(),
        "abbbbxccd".to_string(),
        true,
        true,
    ).unwrap();
    assert_eq!(actual, true);
}

#[test]
fn test_match_line_parse_error() {
    use super::error::ParseError;

    let actual = match_line(
        "ab(c|d".to_string(),
        "a".to_string(),
        false,
        false
    );
    assert_eq!(actual, Err(RegexEngineError::ParseError(ParseError::NoRightParen)));
}