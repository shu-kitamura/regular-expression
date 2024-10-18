//! マッチングを行う関数を定義
pub mod compiler;
pub mod evaluator;
pub mod instruction;
pub mod parser;

use crate::{
    error::RegexError,
    engine::{
        compiler::compile,
        evaluator::eval,
        instruction::Instruction,
        parser::{AST, parse},
    }
};

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

pub fn safe_add<T, F, E>(dst :&mut T, src: &T, f: F) -> Result<(), E> 
where
    T: SafeAdd,
    F: Fn() -> E,
{       
    {
        if let Some(n) = dst.safe_add(src) {
            *dst = n;
            Ok(())
        } else {
            Err(f())
        }
    }
}

/// 文字列のマッチングを実行する。
fn match_string(insts: &Vec<Instruction>, string: &str, is_end_doller: bool) -> Result<bool, RegexError> {
    let charcters: Vec<char> = string.chars().collect();
    let match_result: bool = eval(&insts, &charcters, is_end_doller)?;
    Ok(match_result)
}

/// パターンと文字列のマッチングを実行する
/// 
/// # 引数
/// 
/// * pattern -> 正規表現のパターン
/// * line -> マッチング対象の文字列
/// * is_ignore_case -> 大小文字の区別をするかどうか
/// * is_invert_match -> マッチングの結果を逆にする
/// 
/// # 返り値
/// 
/// エラーなく実行でき、マッチングに成功した場合 true を返す。  
/// エラーなく実行でき、マッチングに失敗した場合 false を返す。  
/// ※ is_invert_match に true が指定されている場合は マッチング結果が反対になる。  
pub fn match_line(
    mut pattern: String,
    mut line: String,
    is_ignore_case: bool,
    is_invert_match: bool
    ) -> Result<bool, RegexError> {
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
    let ast: AST = parse(pattern.as_str())?;

    // AST から コード(Instructionの配列)を生成する。
    let code: Vec<Instruction> = compile(&ast)?;

    let mut is_match: bool = false;
    // パターンの1文字目が ^ の場合、行頭からのマッチのみ実行する
    if is_caret {
        is_match = match_string(&code, &line, is_doller)?;
    } else {
        for (i, _) in line.char_indices() {
            // abcdefg という文字列の場合、以下のように順にマッチングする。
            //     ループ1 : abcdefg
            //     ループ2 : bcdefg
            //     ・・・
            //     ループN : g
            is_match = match_string(&code, &line[i..], is_doller)?;

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

#[cfg(test)]
mod tests {
    use crate::{
        engine::{
            instruction::{Instruction, Char},
            match_string, match_line, invert_match_result, is_beginning_caret, is_end_doller, safe_add
        },
        error::{RegexError, EvalError, ParseError}
    };

    #[test]
    fn test_match_string_true() {
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Split(3, 5),
            Instruction::Char(Char::Literal('c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal('d')),
            Instruction::Match
        ];
        let actual: bool = match_string(&insts, "abc", false).unwrap();
        assert_eq!(actual, true);
    }

    #[test]
    fn test_match_string_false() {
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Split(3, 5),
            Instruction::Char(Char::Literal('c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal('d')),
            Instruction::Match
        ];
        let actual: bool = match_string(&insts, "abx", false).unwrap();
        assert_eq!(actual, false);
    }

    #[test]
    fn test_match_string_eval_error() {
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Split(100, 200),
            Instruction::Char(Char::Literal('c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal('d')),
            Instruction::Match
        ];

        let actual = match_string(&insts, "abc", false);    
        assert_eq!(actual, Err(RegexError::EvalError(EvalError::InvalidPC)));
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
        let actual = match_line(
            "ab(c|d".to_string(),
            "a".to_string(),
            false,
            false
        );
        assert_eq!(actual, Err(RegexError::ParseError(ParseError::NoRightParen)));
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

    #[test]
    fn test_safe_add_success() {
        use crate::error::CompileError;
        let mut u: usize = 1;
        let _ = safe_add(&mut u, &1, || RegexError::CompileError(CompileError::PCOverFlow));
        assert_eq!(u, 2);
    }

    #[test]
    fn test_safe_add_failure() {
        use crate::error::CompileError;

        let expect = RegexError::CompileError(CompileError::PCOverFlow);
        let mut u: usize = usize::MAX;
        let actual: RegexError = safe_add(&mut u, &1, || RegexError::CompileError(CompileError::PCOverFlow)).unwrap_err();
        assert_eq!(actual, expect);
    }
}