//! マッチングを行う関数を定義
pub mod compiler;
pub mod evaluator;
pub mod instruction;
pub mod parser;

use crate::{
    engine::{
        compiler::compile,
        evaluator::eval,
        instruction::Instruction,
        parser::{parse, Ast},
    },
    error::RegexError,
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

/// 文字列のマッチングを実行する。
fn match_string(insts: &[Instruction], str: &str, is_end_dollar: bool) -> Result<bool, RegexError> {
    let charcters: Vec<char> = str.chars().collect();
    let match_result: bool = eval(insts, &charcters, is_end_dollar)?;
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
    is_invert_match: bool,
) -> Result<bool, RegexError> {
    // パターンが ^ で始まるかどうか。
    // 始まる場合、行頭からのマッチのみ実行する。始まらない場合、行頭以外のマッチも実行する。
    // どちらか判定するために使う。
    let is_caret: bool = pattern.starts_with('^');
    if is_caret {
        // パターンが ^ で始まる場合、^ を取り除く。
        // Ast に ^ が含まれないようにするための処理。
        pattern = pattern.strip_prefix("^").unwrap().to_string();
    }

    // パターンが $ で終わるかどうか。
    // 始まる場合、行末かどうかチェックをマッチに含める。
    let is_dollar: bool = pattern.ends_with('$');
    if is_dollar {
        // パターンが $ で終わる場合、$ を取り除く。
        // Ast に $ が含まれないようにするための処理。
        pattern = pattern.strip_suffix("$").unwrap().to_string();
    }

    // -i が指定された場合の処理
    // パターンと行を小文字にすることで、区別をしないようにする
    if is_ignore_case {
        pattern = pattern.to_lowercase();
        line = line.to_lowercase();
    }

    // パターンから Ast を生成する。
    let ast: Ast = parse(pattern.as_str())?;

    // Ast から コード(Instructionの配列)を生成する。
    let code: Vec<Instruction> = compile(&ast)?;

    let mut is_match: bool = false;
    // パターンの1文字目が ^ の場合、行頭からのマッチのみ実行する
    if is_caret {
        is_match = match_string(&code, &line, is_dollar)?;
    } else {
        for (i, _) in line.char_indices() {
            // abcdefg という文字列の場合、以下のように順にマッチングする。
            //     ループ1 : abcdefg
            //     ループ2 : bcdefg
            //     ・・・
            //     ループN : g
            is_match = match_string(&code, &line[i..], is_dollar)?;

            // マッチングが成功した場合、ループを抜ける
            if is_match {
                break;
            }
        }
    }

    Ok(is_match ^ is_invert_match)
}

// ----- テストコード・試し -----

#[cfg(test)]
mod tests {
    use crate::{
        engine::{
            instruction::{Char, Instruction},
            match_line, match_string, safe_add,
        },
        error::{EvalError, ParseError, RegexError},
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
            Instruction::Match,
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
            Instruction::Match,
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
            Instruction::Match,
        ];

        let actual = match_string(&insts, "abc", false);
        assert_eq!(actual, Err(RegexError::Eval(EvalError::InvalidPC)));
    }

    #[test]
    fn test_match_line_true() {
        let actual: bool = match_line(
            "ab*(c|d)".to_string(),
            "xorabbbbd".to_string(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(actual, true);
    }

    #[test]
    fn test_match_line_false() {
        let actual: bool =
            match_line("Ab*(c|d)".to_string(), "abbbbxccd".to_string(), true, false).unwrap();
        assert_eq!(actual, false);
    }

    #[test]
    fn test_match_invert() {
        let actual: bool =
            match_line("Ab*(c|d)".to_string(), "abbbbxccd".to_string(), true, true).unwrap();
        assert_eq!(actual, true);
    }

    #[test]
    fn test_match_line_beginning_caret() {
        // a で始まり、bの0回以上の繰り返し、 c があるので、マッチすることを期待。
        // (true を期待するケース)
        let actual1: bool = match_line(
            "^ab*(c|d)".to_string(),
            "abbbbccd".to_string(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(actual1, true);

        // a で始まっていないので、マッチしないことを期待。
        // (false を期待するケース)
        let actual2: bool =
            match_line("^b*(c|d)".to_string(), "abbbbccd".to_string(), false, false).unwrap();
        assert_eq!(actual2, false);
    }

    #[test]
    fn test_match_line_is_end_dollar() {
        // パターンと一致する部分(abd)が行末なので、マッチすることを期待。
        // (true を期待するケース)
        let actual1: bool =
            match_line("ab(c|d)$".to_string(), "asdfabd".to_string(), false, false).unwrap();
        assert_eq!(actual1, true);

        // パターンと一致する部分(abc)が行末ではないので、マッチしないことを期待。
        // (false を期待するケース)
        let actual2: bool = match_line(
            "ab(c|d)$".to_string(),
            "asdfabdxxx".to_string(),
            false,
            false,
        )
        .unwrap();
        assert_eq!(actual2, false);
    }

    #[test]
    fn test_match_line_parse_error() {
        let actual = match_line("ab(c|d".to_string(), "a".to_string(), false, false);
        assert_eq!(actual, Err(RegexError::Parse(ParseError::NoRightParen)));
    }

    #[test]
    fn test_safe_add_success() {
        use crate::error::CompileError;
        let mut u: usize = 1;
        let _ = safe_add(&mut u, &1, || RegexError::Compile(CompileError::PCOverFlow));
        assert_eq!(u, 2);
    }

    #[test]
    fn test_safe_add_failure() {
        use crate::error::CompileError;

        let expect = RegexError::Compile(CompileError::PCOverFlow);
        let mut u: usize = usize::MAX;
        let actual: RegexError =
            safe_add(&mut u, &1, || RegexError::Compile(CompileError::PCOverFlow)).unwrap_err();
        assert_eq!(actual, expect);
    }
}
