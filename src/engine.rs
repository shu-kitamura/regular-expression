//! マッチングを行う関数を定義
pub mod compiler;
pub mod evaluator;
pub mod instruction;
pub mod parser;

use crate::{
    engine::{
        compiler::compile,
        evaluator::eval,
        instruction::{Char, Instruction},
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
    line: &str,
    is_caret: bool,
    is_dollar: bool,
    is_invert_match: bool,
) -> Result<bool, RegexError> {
    let mut is_match: bool = false;
    // パターンの1文字目が ^ の場合、行頭からのマッチのみ実行する
    if is_caret {
        is_match = match_string(code, line, is_dollar)?;
    } else {
        for (i, ch) in line.char_indices() {
            // code の最初の文字と異なる場合、スキップする（どうせマッチしないため）
            // 無駄なマッチングを減らして高速化するため
            if let Some(Instruction::Char(Char::Literal(first_ch))) = code.first() {
                if ch != *first_ch {
                    continue;
                }
            }
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

    Ok(is_match ^ is_invert_match)
}
/// 文字列のマッチングを実行する。
fn match_string(insts: &[Instruction], str: &str, is_end_dollar: bool) -> Result<bool, RegexError> {
    let charcters: Vec<char> = str.chars().collect();
    let match_result: bool = eval(insts, &charcters, is_end_dollar)?;
    Ok(match_result)
}

// ----- テストコード・試し -----

#[cfg(test)]
mod tests {
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
    fn test_match_string_empty() {
        // パターン "a*" と空文字列のマッチングを行うテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Split(1, 3),
            Instruction::Char(Char::Literal('a')),
            Instruction::Jump(0),
            Instruction::Match,
        ];
        let actual: bool = match_string(&insts, "", false).unwrap();
        assert_eq!(actual, true);
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
        assert_eq!(is_caret, false);
        assert_eq!(is_dollar, false);
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
        assert_eq!(is_caret, true);
        assert_eq!(is_dollar, false);
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
        assert_eq!(is_caret, false);
        assert_eq!(is_dollar, true);
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
        // "abc" という文字列をマッチングするテスト
        let actual1: bool = match_line(&insts, "abc", false, false, false).unwrap();
        assert_eq!(actual1, true);

        // "abe" という文字列をマッチングするテスト
        let actual2: bool = match_line(&insts, "abe", false, false, false).unwrap();
        assert_eq!(actual2, false);

        // "a?b$" というパターンに対するテスト
        // 命令列の 1 番目が Char 以外のテスト
        let insts = vec![
            Instruction::Split(1, 2),
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];
        let actual3 = match_line(&insts, "ab", false, false, false).unwrap();
        assert_eq!(actual3, true);
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
        // "aab" という文字列をマッチングするテスト
        let actual1: bool = match_line(&insts, "aab", true, false, false).unwrap();
        assert_eq!(actual1, true);

        // "xabcd" という文字列をマッチングするテスト
        let actual2: bool = match_line(&insts, "xabcd", true, false, false).unwrap();
        assert_eq!(actual2, false);
    }

    #[test]
    fn test_match_line_dollar() {
        // "ab$" というパターンに対してのテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];
        // "ab" という文字列をマッチングするテスト
        let actual1: bool = match_line(&insts, "ab", false, true, false).unwrap();
        assert_eq!(actual1, true);

        // "abc" という文字列をマッチングするテスト
        let actual2: bool = match_line(&insts, "abc", false, true, false).unwrap();
        assert_eq!(actual2, false);
    }

    #[test]
    fn test_match_line_invert() {
        // "a+b" というパターンに対してのテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Split(0, 2),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];

        // "ab" という文字列をマッチングするテスト
        let actual1: bool = match_line(&insts, "abc", false, false, true).unwrap();
        assert_eq!(actual1, false);

        // "abc" という文字列をマッチングするテスト
        let actual2: bool = match_line(&insts, "acd", false, false, true).unwrap();
        assert_eq!(actual2, true);
    }
}
