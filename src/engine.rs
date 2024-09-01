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
fn match_string(insts: &Vec<Instruction>, string: &str, is_end_doller: bool) -> Result<bool, RegexEngineError> {
    let charcters: Vec<char> = string.chars().collect();
    let match_result: bool = match eval(&insts, &charcters, is_end_doller) {
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
pub fn match_line(
    mut pattern: String,
    mut line: String,
    is_ignore_case: bool,
    is_invert_match: bool
    ) -> Result<bool, RegexEngineError> {
    // パターンが ^ で始まるかどうか。
    // 始まる場合、行頭からのマッチのみ実行する。始まらない場合、行頭以外のマッチも実行する。
    // どちらか判定するために使う。
    let is_caret: bool = is_beginning_caret(&pattern);
    if is_caret {
        // パターンが ^ で始まる場合、^ を取り除く。
        // AST に ^ が含まれないようにするための処理。
        pattern = pattern
                    .get(1..)
                    .unwrap()
                    .to_string();
    }

    // パターンが $ で終わるかどうか。
    // 始まる場合、行末かどうかチェックをマッチに含める。
    let is_doller: bool = is_end_doller(&pattern);
    if is_doller {
        // パターンが $ で終わる場合、$ を取り除く。
        // AST に $ が含まれないようにするための処理。
        pattern = pattern
                        .get(..pattern.len()-1)
                        .unwrap()
                        .to_string();
    }

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

    let mut is_match: bool = false;
    // パターンの1文字目が ^ の場合、行頭からのマッチのみ実行する
    if is_caret {
        is_match = match match_string(&code, &line, is_doller) {
            Ok(res) => res,
            Err(e) => return Err(e),
        };
    } else {
        for (i, _) in line.char_indices() {
            // abcdefg という文字列の場合、以下のように順にマッチングする。
            //     ループ1 : abcdefg
            //     ループ2 : bcdefg
            //     ・・・
            //     ループN : g
            is_match = match match_string(&code, &line[i..], is_doller) {
                Ok(res) => res,
                Err(e) => return Err(e),
            };

            // マッチングが成功した場合、ループを抜ける
            if is_match {
                break
            }
        }
    }

    Ok(invert_match_result(is_match, is_invert_match))
}

/// パターンが ^ で始まるかどうかを返す関数
fn is_beginning_caret(pattern: &str) -> bool {
    if let Some(beginning) = pattern.get(..1) {
        "^" == beginning
    } else {
        false
    }
}

/// パターンが $ で終わるかどうかを返す関数
fn is_end_doller(pattern: &str) -> bool {
    let length: usize = pattern.len();
    if let Some(end) = pattern.get(length-1..length) {
        "$" == end
    } else {
        false
    }
}

/// マッチ結果を反転させる関数  
/// -v オプションが指定された場合、反転させる必要がある。  
fn invert_match_result(match_result: bool, is_invert: bool) -> bool {
    if is_invert {
        !match_result
    } else {
        match_result
    }
}

#[test]
fn test_match_string_true() {
    let insts: Vec<Instruction> = vec![
        Instruction::Char(instruction::Char::Literal('a')),
        Instruction::Char(instruction::Char::Literal('b')),
        Instruction::Split(3, 5),
        Instruction::Char(instruction::Char::Literal('c')),
        Instruction::Jump(6),
        Instruction::Char(instruction::Char::Literal('d')),
        Instruction::Match
    ];
    let actual: bool = match_string(&insts, "abc", false).unwrap();
    assert_eq!(actual, true);
}

#[test]
fn test_match_string_false() {
    let insts: Vec<Instruction> = vec![
        Instruction::Char(instruction::Char::Literal('a')),
        Instruction::Char(instruction::Char::Literal('b')),
        Instruction::Split(3, 5),
        Instruction::Char(instruction::Char::Literal('c')),
        Instruction::Jump(6),
        Instruction::Char(instruction::Char::Literal('d')),
        Instruction::Match
    ];
    let actual: bool = match_string(&insts, "abx", false).unwrap();
    assert_eq!(actual, false);
}

#[test]
fn test_match_string_eval_error() {
    use super::error::EvalError;

    let insts: Vec<Instruction> = vec![
        Instruction::Char(instruction::Char::Literal('a')),
        Instruction::Char(instruction::Char::Literal('b')),
        Instruction::Split(100, 200),
        Instruction::Char(instruction::Char::Literal('c')),
        Instruction::Jump(6),
        Instruction::Char(instruction::Char::Literal('d')),
        Instruction::Match
    ];

    let actual = match_string(&insts, "abc", false);    
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
fn test_match_line_biginning_caret() {
    // a で始まり、bの0回以上の繰り返し、 c があるので、マッチすることを期待。
    // (true を期待するケース)
    let actual1: bool = match_line(
        "^ab*(c|d)".to_string(),
        "abbbbccd".to_string(),
        false,
        false,
    ).unwrap();
    assert_eq!(actual1, true);

    // a で始まっていないので、マッチしないことを期待。
    // (false を期待するケース)
    let actual2: bool = match_line(
        "^b*(c|d)".to_string(),
        "abbbbccd".to_string(),
        false,
        false,
    ).unwrap();
    assert_eq!(actual2, false);
}

#[test]
fn test_match_line_is_end_doller() {
    // パターンと一致する部分(abd)が行末なので、マッチすることを期待。
    // (true を期待するケース)
    let actual1: bool = match_line(
        "ab(c|d)$".to_string(),
        "asdfabd".to_string(),
        false,
        false,
    ).unwrap();
    assert_eq!(actual1, true);

    // パターンと一致する部分(abc)が行末ではないので、マッチしないことを期待。
    // (false を期待するケース)
    let actual2: bool = match_line(
        "ab(c|d)$".to_string(),
        "asdfabdxxx".to_string(),
        false,
        false,
    ).unwrap();
    assert_eq!(actual2, false);
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

#[test]
fn test_is_beginning_caret_true() {
    let actual: bool = is_beginning_caret("^pattern");
    assert_eq!(actual, true);
}

#[test]
fn test_is_beginning_caret_false() {
    let actual: bool = is_beginning_caret("pattern");
    assert_eq!(actual, false);
}

#[test]
fn test_is_end_doller_true() {
    let actual: bool = is_end_doller("pattern$");
    assert_eq!(actual, true);
}

#[test]
fn test_is_end_doller_false() {
    let actual: bool = is_end_doller("pattern");
    assert_eq!(actual, false);
}

#[test]
fn test_invert_match_result_true() {
    let actual: bool = invert_match_result(true, false);
    assert_eq!(actual, true);

    let actual: bool = invert_match_result(false, true);
    assert_eq!(actual, true);
}

#[test]
fn test_invert_match_result_false() {
    let actual: bool = invert_match_result(true, true);
    assert_eq!(actual, false);

    let actual: bool = invert_match_result(false, false);
    assert_eq!(actual, false);
}