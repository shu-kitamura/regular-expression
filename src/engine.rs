//! マッチングを行う関数を定義
pub mod compiler;
pub mod evaluator;
pub mod instruction;
pub mod parser;

use std::collections::BTreeSet;

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

/// パターンをパースして、コンパイルする
pub fn compile_pattern(mut pattern: &str) -> Result<(Vec<Instruction>, bool, bool), RegexError> {
    let is_caret = pattern.starts_with('^');
    if let Some(striped) = pattern.strip_prefix("^") {
        pattern = striped;
    }

    let is_dollar = pattern.ends_with('$');
    if let Some(striped) = pattern.strip_suffix("$") {
        pattern = striped;
    }

    // パターンから Ast を生成する。
    let ast: Ast = parse(pattern)?;

    // Ast から コード(Instructionの配列)を生成する。
    let instructions: Vec<Instruction> = compile(&ast)?;

    Ok((instructions, is_caret, is_dollar))
}

/// パターンと文字列のマッチングを実行する
pub fn match_line(
    code: &[Instruction],
    first_strings: &BTreeSet<String>,
    line: &str,
    is_caret: bool,
    is_dollar: bool,
) -> Result<bool, RegexError> {
    let mut is_match: bool = false;

    if is_caret {
        return match_string(code, line, is_dollar);
    }

    // 先頭リテラルがある場合、最初の文字を取得する
    if !first_strings.is_empty() {
        let mut pos = 0;
        while let Some(i) = find_index(&line[pos..], first_strings) {
            let start = pos + i;

            is_match = match_string(code, &line[start..], is_dollar)?;
            if is_match {
                break;
            }
            pos = start + 1;
        }
    } else {
        // 先頭リテラル無し → 旧ループ
        // ここに到達するのは、最初の命令が Char::Any の場合のみ
        for i in 0..line.len() {
            // abcdefg という文字列の場合、以下のように順にマッチングする。
            //     ループ1 : abcdefg
            //     ループ2 : bcdefg
            //     ・・・
            //     ループN : g
            is_match = match_string(code, &line[i..], is_dollar)?;

            // マッチングが成功した場合、ループを抜ける
            if is_match {
                break;
            }
        }
    }

    Ok(is_match)
}

/// 文字列のマッチングを実行する。
fn match_string(
    insts: &[Instruction],
    string: &str,
    is_end_dollar: bool,
) -> Result<bool, RegexError> {
    let match_result: bool = eval(insts, string, is_end_dollar)?;
    Ok(match_result)
}

fn find_index(string: &str, string_set: &BTreeSet<String>) -> Option<usize> {
    string_set
        .iter()
        .map(|s| string.find(s))
        .filter(|opt| opt.is_some())
        .min()?
}

// ----- テストコード -----

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::{
        engine::{
            compile_pattern,
            instruction::{Char, Instruction},
            match_line, match_string, safe_add,
        },
        error::{EvalError, RegexError},
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

        let actual: bool = match_string(&insts, "abcd", false).unwrap();
        assert!(actual);
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
        assert!(!actual);
    }

    #[test]
    fn test_match_string_empty() {
        // パターン "a*" と空文字列のマッチングを行うテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Split(1, 3),
            Instruction::Char(Char::Literal('a')),
            Instruction::Jump(0),
            Instruction::Match,
        ];
        let actual: bool = match_string(&insts, "", false).unwrap();
        assert!(actual);
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

    #[test]
    fn test_compile_pattern() {
        // "ab(c|d)" というパターンをコンパイルするテスト
        let expect = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Split(3, 5),
            Instruction::Char(Char::Literal('c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal('d')),
            Instruction::Match,
        ];

        let (code, is_caret, is_dollar) = compile_pattern("ab(c|d)").unwrap();
        assert_eq!(code, expect);
        assert!(!is_caret);
        assert!(!is_dollar);
    }

    #[test]
    fn test_compile_pattern_caret() {
        // "^a*" というパターンをコンパイルするテスト
        let expect = vec![
            Instruction::Split(1, 3),
            Instruction::Char(Char::Literal('a')),
            Instruction::Jump(0),
            Instruction::Match,
        ];

        let (code, is_caret, is_dollar) = compile_pattern("^a*").unwrap();
        assert_eq!(code, expect);
        assert!(is_caret);
        assert!(!is_dollar);
    }

    #[test]
    fn test_compile_pattern_dollar() {
        // "a?b$" というパターンをコンパイルするテスト
        let expect = vec![
            Instruction::Split(1, 2),
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];

        let (code, is_caret, is_dollar) = compile_pattern("a?b$").unwrap();
        assert_eq!(code, expect);
        assert!(!is_caret);
        assert!(is_dollar);
    }

    #[test]
    fn test_match_line() {
        // "ab(c|d)" というパターンに対してのテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Split(3, 5),
            Instruction::Char(Char::Literal('c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal('d')),
            Instruction::Match,
        ];
        let first_strings: BTreeSet<String> = ["ab"].iter().map(|s| s.to_string()).collect();

        // "abc" という文字列をマッチングするテスト
        let actual1: bool = match_line(&insts, &first_strings, "abc", false, false).unwrap();
        assert!(actual1);

        // "abe" という文字列をマッチングするテスト
        let actual2: bool = match_line(&insts, &first_strings, "abe", false, false).unwrap();
        assert!(!actual2);

        // "a?b" というパターンに対するテスト
        // 命令列の 1 番目が Char 以外のテスト
        let insts = vec![
            Instruction::Split(1, 2),
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];
        let first_strings: BTreeSet<String> = ["ab", "b"].iter().map(|s| s.to_string()).collect();
        let actual3 = match_line(&insts, &first_strings, "ab", false, false).unwrap();
        assert!(actual3);

        // ".abc" というパターンに対するテスト
        let insts = vec![
            Instruction::Char(Char::Any),
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Char(Char::Literal('c')),
            Instruction::Match,
        ];
        let first_strings: BTreeSet<String> = BTreeSet::new();
        let actual4 = match_line(&insts, &first_strings, "xxxabc", false, false).unwrap();
        assert!(actual4);
    }

    #[test]
    fn test_match_line_caret() {
        // "^a+b" というパターンに対してのテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Split(0, 2),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];
        let first_strings: BTreeSet<String> = ["a"].iter().map(|s| s.to_string()).collect();

        // "aab" という文字列をマッチングするテスト
        let actual1: bool = match_line(&insts, &first_strings, "aab", true, false).unwrap();
        assert!(actual1);

        // "xabcd" という文字列をマッチングするテスト
        let actual2: bool = match_line(&insts, &first_strings, "xabcd", true, false).unwrap();
        assert!(!actual2);
    }

    #[test]
    fn test_match_line_dollar() {
        // "ab$" というパターンに対してのテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];
        let first_strings: BTreeSet<String> = ["a"].iter().map(|s| s.to_string()).collect();
        // "ab" という文字列をマッチングするテスト
        let actual1: bool = match_line(&insts, &first_strings, "ab", false, true).unwrap();
        assert!(actual1);

        // "abc" という文字列をマッチングするテスト
        let actual2: bool = match_line(&insts, &first_strings, "abc", false, true).unwrap();
        assert!(!actual2);
    }
}
