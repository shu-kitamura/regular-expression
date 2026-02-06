//! マッチングを行う関数を定義
pub mod compiler;
pub mod evaluator;
pub mod instruction;
pub mod parser;
pub mod search_plan;

use crate::{
    engine::{
        compiler::compile,
        evaluator::{EvalOptions, EvalScratch, eval, eval_from},
        instruction::Instruction,
        parser::{Ast, parse},
        search_plan::SearchPlan,
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

    // 空のパターン（例: "^$" が入力され、アンカーが除去された場合）を処理する。
    // アンカーが存在する場合のみ、空のパターンを許可する。
    // 空のパターンは空の文字列にマッチする必要があるため、Match 命令のみを含む命令列を返す。
    // この Match 命令は、アンカー条件（行頭/行末）が満たされた場合に即座に成功する。
    if pattern.is_empty() && (is_caret || is_dollar) {
        return Ok((vec![Instruction::Match], is_caret, is_dollar));
    }

    // パターンから Ast を生成する。
    let ast: Ast = parse(pattern)?;

    // Ast から コード(Instructionの配列)を生成する。
    let instructions: Vec<Instruction> = compile(&ast)?;

    Ok((instructions, is_caret, is_dollar))
}

pub fn build_search_plan(code: &[Instruction]) -> SearchPlan {
    SearchPlan::build(code)
}

/// パターンとバイト列のマッチングを実行する
pub fn match_line(
    code: &[Instruction],
    search_plan: &SearchPlan,
    line: &[u8],
    is_ignore_case: bool,
    is_caret: bool,
    is_dollar: bool,
) -> Result<bool, RegexError> {
    let mut scratch = EvalScratch::new();

    if is_caret {
        return match_from(code, line, 0, is_ignore_case, is_dollar, &mut scratch);
    }

    if search_plan.can_match_empty && !is_dollar {
        return Ok(true);
    }

    for start in 0..=line.len() {
        if start == line.len() {
            if !search_plan.can_match_empty {
                continue;
            }
        } else {
            if !search_plan.accepts_first_byte(line[start], is_ignore_case) {
                continue;
            }

            if let Some(literal) = search_plan.leading_literal.as_deref()
                && !starts_with_literal_at(line, start, literal, is_ignore_case)
            {
                continue;
            }
        }

        if match_from(code, line, start, is_ignore_case, is_dollar, &mut scratch)? {
            return Ok(true);
        }
    }

    Ok(false)
}

/// バイト列のマッチングを実行する。
fn match_from(
    insts: &[Instruction],
    input: &[u8],
    start_index: usize,
    is_ignore_case: bool,
    is_end_dollar: bool,
    scratch: &mut EvalScratch,
) -> Result<bool, RegexError> {
    if start_index == 0 && !is_ignore_case {
        let match_result: bool = eval(insts, input, is_end_dollar)?;
        return Ok(match_result);
    }

    let options = EvalOptions {
        is_end_dollar,
        ignore_case_ascii: is_ignore_case,
    };
    let match_result: bool = eval_from(insts, input, start_index, options, scratch)?;
    Ok(match_result)
}

#[cfg(test)]
fn match_string(
    insts: &[Instruction],
    input: &[u8],
    is_end_dollar: bool,
) -> Result<bool, RegexError> {
    let mut scratch = EvalScratch::new();
    match_from(insts, input, 0, false, is_end_dollar, &mut scratch)
}

fn starts_with_literal_at(
    input: &[u8],
    start: usize,
    literal: &[u8],
    ignore_case_ascii: bool,
) -> bool {
    if literal.is_empty() {
        return true;
    }

    let end = start.saturating_add(literal.len());
    if end > input.len() {
        return false;
    }

    if ignore_case_ascii {
        input[start..end]
            .iter()
            .zip(literal.iter())
            .all(|(&input_b, &pat_b)| input_b.eq_ignore_ascii_case(&pat_b))
    } else {
        &input[start..end] == literal
    }
}

// ----- テストコード -----

#[cfg(test)]
mod tests {
    use crate::{
        engine::{
            build_search_plan, compile_pattern,
            instruction::{Char, Instruction},
            match_line, match_string, safe_add,
            search_plan::SearchPlan,
        },
        error::{EvalError, RegexError},
    };

    fn plan(insts: &[Instruction]) -> SearchPlan {
        build_search_plan(insts)
    }

    #[test]
    fn test_match_string_true() {
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Char(Char::Literal(b'b')),
            Instruction::Split(3, 5),
            Instruction::Char(Char::Literal(b'c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal(b'd')),
            Instruction::Match,
        ];

        let actual: bool = match_string(&insts, b"abcd", false).unwrap();
        assert!(actual);
    }

    #[test]
    fn test_match_string_false() {
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Char(Char::Literal(b'b')),
            Instruction::Split(3, 5),
            Instruction::Char(Char::Literal(b'c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal(b'd')),
            Instruction::Match,
        ];
        let actual: bool = match_string(&insts, b"abx", false).unwrap();
        assert!(!actual);
    }

    #[test]
    fn test_match_string_empty() {
        // パターン "a*" と空文字列のマッチングを行うテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Split(1, 3),
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Jump(0),
            Instruction::Match,
        ];
        let actual: bool = match_string(&insts, b"", false).unwrap();
        assert!(actual);
    }

    #[test]
    fn test_match_string_eval_error() {
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Char(Char::Literal(b'b')),
            Instruction::Split(100, 200),
            Instruction::Char(Char::Literal(b'c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal(b'd')),
            Instruction::Match,
        ];
        let actual = match_string(&insts, b"abc", false);
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
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Char(Char::Literal(b'b')),
            Instruction::Split(3, 5),
            Instruction::Char(Char::Literal(b'c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal(b'd')),
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
            Instruction::Char(Char::Literal(b'a')),
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
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Char(Char::Literal(b'b')),
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
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Char(Char::Literal(b'b')),
            Instruction::Split(3, 5),
            Instruction::Char(Char::Literal(b'c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal(b'd')),
            Instruction::Match,
        ];
        let search_plan = plan(&insts);

        // "abc" という文字列をマッチングするテスト
        let actual1: bool = match_line(&insts, &search_plan, b"abc", false, false, false).unwrap();
        assert!(actual1);

        // "abe" という文字列をマッチングするテスト
        let actual2: bool = match_line(&insts, &search_plan, b"abe", false, false, false).unwrap();
        assert!(!actual2);

        // "a?b" というパターンに対するテスト
        // 命令列の 1 番目が Char 以外のテスト
        let insts = vec![
            Instruction::Split(1, 2),
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Char(Char::Literal(b'b')),
            Instruction::Match,
        ];
        let search_plan = plan(&insts);
        let actual3 = match_line(&insts, &search_plan, b"ab", false, false, false).unwrap();
        assert!(actual3);

        // ".abc" というパターンに対するテスト
        let insts = vec![
            Instruction::Char(Char::Any),
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Char(Char::Literal(b'b')),
            Instruction::Char(Char::Literal(b'c')),
            Instruction::Match,
        ];
        let search_plan = plan(&insts);
        let actual4 = match_line(&insts, &search_plan, b"xxxabc", false, false, false).unwrap();
        assert!(actual4);
    }

    #[test]
    fn test_match_line_caret() {
        // "^a+b" というパターンに対してのテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Split(0, 2),
            Instruction::Char(Char::Literal(b'b')),
            Instruction::Match,
        ];
        let search_plan = plan(&insts);

        // "aab" という文字列をマッチングするテスト
        let actual1: bool = match_line(&insts, &search_plan, b"aab", false, true, false).unwrap();
        assert!(actual1);

        // "xabcd" という文字列をマッチングするテスト
        let actual2: bool = match_line(&insts, &search_plan, b"xabcd", false, true, false).unwrap();
        assert!(!actual2);
    }

    #[test]
    fn test_match_line_dollar() {
        // "ab$" というパターンに対してのテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Char(Char::Literal(b'b')),
            Instruction::Match,
        ];
        let search_plan = plan(&insts);
        // "ab" という文字列をマッチングするテスト
        let actual1: bool = match_line(&insts, &search_plan, b"ab", false, false, true).unwrap();
        assert!(actual1);

        // "abc" という文字列をマッチングするテスト
        let actual2: bool = match_line(&insts, &search_plan, b"abc", false, false, true).unwrap();
        assert!(!actual2);
    }

    #[test]
    fn test_compile_pattern_empty_with_anchors() {
        // "^$" というパターンをコンパイルするテスト（空行にマッチ）
        // この機能は以前 ParseError::Empty を返していた問題を修正したもの
        let expect = vec![Instruction::Match];

        let (code, is_caret, is_dollar) = compile_pattern("^$").unwrap();
        assert_eq!(code, expect);
        assert!(is_caret);
        assert!(is_dollar);

        // "^" というパターンをコンパイルするテスト（行頭にマッチ）
        let (code2, is_caret2, is_dollar2) = compile_pattern("^").unwrap();
        assert_eq!(code2, vec![Instruction::Match]);
        assert!(is_caret2);
        assert!(!is_dollar2);

        // "$" というパターンをコンパイルするテスト（行末にマッチ）
        let (code3, is_caret3, is_dollar3) = compile_pattern("$").unwrap();
        assert_eq!(code3, vec![Instruction::Match]);
        assert!(!is_caret3);
        assert!(is_dollar3);
    }

    #[test]
    fn test_match_empty_line() {
        // "^$" というパターンで空行をマッチングするテスト
        let (code, is_caret, is_dollar) = compile_pattern("^$").unwrap();
        let search_plan = build_search_plan(&code);

        // 空文字列とマッチするテスト
        let actual1: bool =
            match_line(&code, &search_plan, b"", false, is_caret, is_dollar).unwrap();
        assert!(actual1);

        // 非空文字列とマッチしないテスト
        let actual2: bool =
            match_line(&code, &search_plan, b"test", false, is_caret, is_dollar).unwrap();
        assert!(!actual2);

        // スペースを含む文字列とマッチしないテスト
        let actual3: bool =
            match_line(&code, &search_plan, b" ", false, is_caret, is_dollar).unwrap();
        assert!(!actual3);
    }

    #[test]
    fn test_match_line_ignore_case_ascii() {
        let (code, is_caret, is_dollar) = compile_pattern("ab").unwrap();
        let search_plan = build_search_plan(&code);

        let actual = match_line(&code, &search_plan, b"AB", true, is_caret, is_dollar).unwrap();
        assert!(actual);
    }

    #[test]
    fn test_regression_or_branches() {
        let (code, is_caret, is_dollar) = compile_pattern("a|b|c").unwrap();
        let search_plan = build_search_plan(&code);

        assert!(match_line(&code, &search_plan, b"a", false, is_caret, is_dollar).unwrap());
        assert!(match_line(&code, &search_plan, b"b", false, is_caret, is_dollar).unwrap());
        assert!(match_line(&code, &search_plan, b"c", false, is_caret, is_dollar).unwrap());
    }

    #[test]
    fn test_regression_empty_match_non_anchored() {
        let (star_code, star_caret, star_dollar) = compile_pattern("a*").unwrap();
        let star_plan = build_search_plan(&star_code);
        assert!(match_line(&star_code, &star_plan, b"", false, star_caret, star_dollar).unwrap());
        assert!(
            match_line(
                &star_code,
                &star_plan,
                b"bbb",
                false,
                star_caret,
                star_dollar
            )
            .unwrap()
        );

        let (question_code, question_caret, question_dollar) = compile_pattern("a?").unwrap();
        let question_plan = build_search_plan(&question_code);
        assert!(
            match_line(
                &question_code,
                &question_plan,
                b"",
                false,
                question_caret,
                question_dollar
            )
            .unwrap()
        );
    }

    #[test]
    fn test_match_line_non_utf8_input() {
        let (code, is_caret, is_dollar) = compile_pattern("ab").unwrap();
        let search_plan = build_search_plan(&code);
        let input = [0xFF, b'a', b'b'];
        let actual = match_line(&code, &search_plan, &input, false, is_caret, is_dollar).unwrap();
        assert!(actual);
    }

    #[test]
    #[ignore]
    fn test_perf_match_line_cases() {
        use std::{hint::black_box, time::Instant};

        fn bench_case(pattern: &str, input: &[u8], loops: usize) {
            let (code, is_caret, is_dollar) = compile_pattern(pattern).unwrap();
            let search_plan = build_search_plan(&code);

            let start = Instant::now();
            let mut matched = 0usize;
            for _ in 0..loops {
                if match_line(&code, &search_plan, input, false, is_caret, is_dollar).unwrap() {
                    matched += 1;
                }
            }
            let elapsed = start.elapsed();
            eprintln!(
                "[perf] pattern={pattern:?} loops={loops} matched={matched} elapsed_ms={}",
                elapsed.as_millis()
            );
            black_box((matched, elapsed));
        }

        let long_input = vec![b'x'; 20_000];
        bench_case("abcde", &long_input, 200);
        bench_case("a|b|c|d|e|f|g|h|i|j", &long_input, 200);

        let mut binary_input = vec![0xFF; 20_000];
        binary_input.extend_from_slice(b"ab");
        bench_case("ab$", &binary_input, 200);
    }
}
