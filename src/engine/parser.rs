//! 正規表現の式をパースするための型・関数  
//! 式をパースして、抽象構文木(Ast)に変換する。  
//! "abc(def|ghi)"" が入力された場合、以下の Ast に変換する  
//!
//! ```text
//! Seq(
//!     Char(a),
//!     Char(b),
//!     Char(c),
//!     Or(
//!         Seq(
//!             Char(d),
//!             Char(e),
//!             Char(f)
//!         ),
//!         Seq(
//!             Char(g),
//!             Char(h),
//!             Char(i)
//!         )
//!     )
//! )
//! ```

use crate::error::ParseError;
use std::mem::take;

// エスケープ文字を定義
const ESCAPE_CHARS: [char; 8] = ['\\', '(', ')', '|', '+', '*', '?', '.'];

/// Ast の型
#[derive(Debug, PartialEq)]
pub enum Ast {
    AnyChar,                // '.'に対応する型
    Char(char),             // 通常の文字に対応する型
    Plus(Box<Ast>),         // '+'に対応する型
    Star(Box<Ast>),         // '*'に対応する型
    Question(Box<Ast>),     // '?'に対応する型
    Or(Box<Ast>, Box<Ast>), // '|'に対応する型
    Seq(Vec<Ast>),          // 連結に対応する型
}

/// エスケープ文字から Ast を生成
fn parse_escape(pos: usize, c: char) -> Result<Ast, ParseError> {
    if ESCAPE_CHARS.contains(&c) {
        Ok(Ast::Char(c))
    } else {
        Err(ParseError::InvalidEscape(pos, c))
    }
}

/// `+`,`*`,`?`から Ast を生成
fn parse_qualifier(qualifier: char, prev: Ast) -> Ast {
    match qualifier {
        '+' => Ast::Plus(Box::new(prev)),
        '*' => Ast::Star(Box::new(prev)),
        '?' => Ast::Question(Box::new(prev)),
        _ => unreachable!(), // 呼び出し方から、到達しないことが確定している
    }
}

/// `|` を含む式から Ast を生成
///
/// 入力されたAstが [Ast1, Ast2, Ast3] の場合、以下の Ast を生成する
/// ```text
/// Ast::Or(
///     Ast1,
///     Ast::Or(
///         Ast2,
///         Ast3
///     )
/// )
/// ```
///
fn fold_or(mut seq_or: Vec<Ast>) -> Option<Ast> {
    if seq_or.len() > 1 {
        let mut ast: Ast = seq_or.pop().unwrap();
        // Ast を逆順で結合するため、reverse メソッドを呼び出す
        seq_or.reverse();
        for s in seq_or {
            ast = Ast::Or(Box::new(s), Box::new(ast));
        }
        Some(ast)
    } else {
        seq_or.pop()
    }
}

/// 式をパースし、Astを生成
pub fn parse(pattern: &str) -> Result<Ast, ParseError> {
    let mut seq: Vec<Ast> = Vec::new();
    let mut seq_or: Vec<Ast> = Vec::new();
    let mut stack: Vec<(Vec<Ast>, Vec<Ast>)> = Vec::new();
    let mut is_escape: bool = false;

    for (pos, c) in pattern.chars().enumerate() {
        if is_escape {
            is_escape = false;
            seq.push(parse_escape(pos, c)?);
            continue;
        }

        match c {
            '+' | '*' | '?' => {
                let prev_ast: Ast = seq.pop().ok_or(ParseError::NoPrev(pos))?;
                let ast: Ast = parse_qualifier(c, prev_ast);
                seq.push(ast);
            }
            '(' => {
                let prev: Vec<Ast> = take(&mut seq);
                let prev_or: Vec<Ast> = take(&mut seq_or);
                stack.push((prev, prev_or));
            }
            ')' => {
                let (mut prev, prev_or) = stack.pop().ok_or(ParseError::InvalidRightParen(pos))?;
                if !seq.is_empty() {
                    seq_or.push(Ast::Seq(seq));
                }

                if let Some(ast) = fold_or(seq_or) {
                    prev.push(ast);
                }

                seq = prev;
                seq_or = prev_or;
            }
            '|' => {
                let prev: Vec<Ast> = take(&mut seq);
                seq_or.push(Ast::Seq(prev));
            }
            '\\' => is_escape = true,
            '.' => seq.push(Ast::AnyChar),
            _ => seq.push(Ast::Char(c)),
        };
    }
    // 閉じカッコが足りないエラー
    if !stack.is_empty() {
        return Err(ParseError::NoRightParen);
    }

    // seq が残っている場合、seq_or に追加
    if !seq.is_empty() {
        seq_or.push(Ast::Seq(seq));
    }

    // 最後に seq_or を fold して、Ast を生成
    if let Some(ast) = fold_or(seq_or) {
        Ok(ast)
    } else {
        Err(ParseError::Empty)
    }
}

// ----- テストコード・試し -----

#[cfg(test)]
mod tests {
    use crate::{
        engine::parser::{fold_or, parse, parse_escape, parse_qualifier, Ast},
        error::ParseError,
    };

    #[test]
    fn test_parse_escape_success() {
        let expect: Ast = Ast::Char('\\');

        // テスト対象を実行
        let actual: Ast = parse_escape(0, '\\').unwrap();
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_escape_failure() {
        let expect = Err(ParseError::InvalidEscape(0, 'a'));

        // テスト対象を実行
        let actual = parse_escape(0, 'a');
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_qualifier_plus() {
        let expect: Ast = Ast::Plus(Box::new(Ast::Char('a')));

        // テスト対象を実行
        let ast: Ast = Ast::Char('a');
        let actual: Ast = parse_qualifier('+', ast);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_qualifier_star() {
        let expect: Ast = Ast::Star(Box::new(Ast::Char('a')));

        // テスト対象を実行
        let ast: Ast = Ast::Char('a');
        let actual: Ast = parse_qualifier('*', ast);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_qualifier_question() {
        let expect: Ast = Ast::Question(Box::new(Ast::Char('a')));

        // テスト対象を実行
        let ast: Ast = Ast::Char('a');
        let actual: Ast = parse_qualifier('?', ast);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_fold_or_if_true() {
        // パターン "a|b|c" を想定し、データ準備
        let seq: Vec<Ast> = vec![Ast::Char('a'), Ast::Char('b'), Ast::Char('c')];

        // a|b|c をパースした場合、以下のAstができる
        // Ast::Or(Ast::Char('a'), Ast::Or(Ast::Char('b'), Ast::Char('c')))
        // 上記のAstを用意するため、データを定義
        let left: Ast = Ast::Char('a');
        let right: Ast = Ast::Or(Box::new(Ast::Char('b')), Box::new(Ast::Char('c')));
        let expect: Ast = Ast::Or(Box::new(left), Box::new(right));

        let actual: Ast = fold_or(seq).unwrap();

        assert_eq!(actual, expect);
    }

    #[test]
    fn test_fold_or_if_false() {
        // 長さ 1 の配列を準備
        let seq: Vec<Ast> = vec![Ast::Char('a')];

        let expect: Ast = Ast::Char('a');

        // テスト対象を実行
        let actual: Ast = fold_or(seq).unwrap();

        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_normal_string() {
        // ----- "abc" が入力されたケース -----
        let expect: Ast = Ast::Seq(vec![Ast::Char('a'), Ast::Char('b'), Ast::Char('c')]);
        // テスト対象を実行
        let pattern: &str = "abc";
        let actual: Ast = parse(pattern).unwrap();
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_contain_qualifier() {
        // ----- "abc+" が入力されたケース -----
        let expect: Ast = Ast::Seq(vec![
            Ast::Char('a'),
            Ast::Char('b'),
            Ast::Plus(Box::new(Ast::Char('c'))),
        ]);
        // テスト対象を実行
        let pattern: &str = "abc+";
        let actual: Ast = parse(pattern).unwrap();
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_contain_or() {
        // ----- "abc|def|ghi" が入力されたケース-----
        let abc: Ast = Ast::Seq(vec![Ast::Char('a'), Ast::Char('b'), Ast::Char('c')]);
        let def: Ast = Ast::Seq(vec![Ast::Char('d'), Ast::Char('e'), Ast::Char('f')]);
        let ghi: Ast = Ast::Seq(vec![Ast::Char('g'), Ast::Char('h'), Ast::Char('i')]);

        let expect: Ast = Ast::Or(
            Box::new(abc),
            Box::new(Ast::Or(Box::new(def), Box::new(ghi))),
        );
        // テスト対象を実行
        let pattern: &str = "abc|def|ghi";
        let actual: Ast = parse(pattern).unwrap();
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_contain_paran() {
        // ----- "abc(def|ghi)" が入力されたケース-----
        let expect: Ast = Ast::Seq(vec![
            Ast::Char('a'),
            Ast::Char('b'),
            Ast::Char('c'),
            Ast::Or(
                Box::new(Ast::Seq(vec![
                    Ast::Char('d'),
                    Ast::Char('e'),
                    Ast::Char('f'),
                ])),
                Box::new(Ast::Seq(vec![
                    Ast::Char('g'),
                    Ast::Char('h'),
                    Ast::Char('i'),
                ])),
            ),
        ]);
        // テスト対象を実行
        let pattern: &str = "abc(def|ghi)";
        let actual: Ast = parse(pattern).unwrap();

        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_contain_period() {
        // ----- "a.c" が入力されたケース-----
        let expect: Ast = Ast::Seq(vec![Ast::Char('a'), Ast::AnyChar, Ast::Char('c')]);
        // テスト対象を実行
        let pattern: &str = "a.c";
        let actual: Ast = parse(pattern).unwrap();

        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_contain_escape() {
        // ----- "a\*b" が入力されたケース -----
        let expect: Ast = Ast::Seq(vec![Ast::Char('a'), Ast::Char('*'), Ast::Char('b')]);
        // テスト対象を実行
        let pattern: &str = "a\\*b";
        let actual: Ast = parse(pattern).unwrap();
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_return_err() {
        // ----- "abc(def|ghi" が入力されたケース -----
        let expect = Err(ParseError::NoRightParen);

        // テスト対象を実行
        let pattern: &str = "abc(def|ghi";
        let actual = parse(pattern);
        assert_eq!(actual, expect);

        // ----- "abc(def|ghi))" が入力されたケース -----
        let expect = Err(ParseError::InvalidRightParen(12));
        // テスト対象を実行
        let pattern: &str = "abc(def|ghi))";
        let actual = parse(pattern);
        assert_eq!(actual, expect);

        // ----- "*abc" が入力されたケース -----
        let expect = Err(ParseError::NoPrev(0));
        // テスト対象を実行
        let pattern: &str = "*abc";
        let actual = parse(pattern);
        assert_eq!(actual, expect);

        // ----- "" が入力されたケース -----
        let expect = Err(ParseError::Empty);
        // テスト対象を実行
        let pattern: &str = "";
        let actual = parse(pattern);
        assert_eq!(actual, expect);

        // ----- "a\bc" が入力されたケース -----
        let expect = Err(ParseError::InvalidEscape(2, 'b'));
        // テスト対象を実行
        let pattern: &str = "a\\bc";
        let actual = parse(pattern);
        assert_eq!(actual, expect);
    }
}
