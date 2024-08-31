//! 正規表現の式をパースするための型・関数  
//! 式をパースして、抽象構文木(AST)に変換する。  
//! "abc(def|ghi)"" が入力された場合、以下の AST に変換する  
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

use std::mem::take;
use crate::error::ParseError;

/// AST の型
#[derive(Debug, PartialEq)]
pub enum AST {
    Char(char),
    Period,
    Plus(Box<AST>),
    Star(Box<AST>),
    Question(Box<AST>),
    Or(Box<AST>, Box<AST>),
    Seq(Vec<AST>),
}

/// 限量子(+, *, ?)の型
#[derive(Debug, PartialEq)]
enum Qualifier {
    Plus,
    Star,
    Question,
}

/// エスケープ文字から AST を生成
fn parse_escape(pos: usize, c: char) -> Result<AST, ParseError> {
    match c {
        '\\' | '(' | ')' | '|' | '+' | '*' | '?' | '.'=> Ok(AST::Char(c)),
        _ => Err(ParseError::InvalidEscape(pos, c)),
    }
}

/// 限量子(+, *, ?)から AST を生成
fn parse_qualifier(qualifier: Qualifier, prev: AST) -> AST{
    match qualifier {
        Qualifier::Plus => AST::Plus(Box::new(prev)),
        Qualifier::Star => AST::Star(Box::new(prev)),
        Qualifier::Question => AST::Question(Box::new(prev)),
    }
}

/// char から限量子(Qualifier型)を生成
fn convert_char_to_qualifier(c: char) -> Option<Qualifier> {
    match c {
        '+' => Some(Qualifier::Plus),
        '*' => Some(Qualifier::Star),
        '?' => Some(Qualifier::Question),
        _ => None,
    }
}

/// Orを含む式から AST を生成
fn fold_or(mut seq_or: Vec<AST>) -> Option<AST> {
    if seq_or.len() > 1 {
        let mut ast: AST = seq_or.pop().unwrap();
        seq_or.reverse();
        for s in seq_or {
            ast = AST::Or(Box::new(s), Box::new(ast));
        }
        Some(ast)
    } else {
        seq_or.pop()
    }
}

/// 式をパースし、ASTを生成
pub fn parse(pattern: &str) -> Result<AST, ParseError> {
    let mut seq: Vec<AST> = Vec::new();
    let mut seq_or: Vec<AST> = Vec::new();
    let mut stack: Vec<(Vec<AST>, Vec<AST>)> = Vec::new();  // コンテキストのスタック

    let mut is_escape: bool = false;

    for (pos, c) in pattern.chars().enumerate() {
        if is_escape {
            is_escape = false;
            match parse_escape(pos, c) {
                Ok(ast) => {
                    seq.push(ast);
                    continue;
                },
                Err(e) => return Err(e)
            };
        }
        match c {
            '+' | '*' | '?' => {
                let qualifier: Qualifier = convert_char_to_qualifier(c).unwrap();
                if let Some(prev_ast) = seq.pop() {
                    let ast: AST = parse_qualifier(qualifier, prev_ast);
                    seq.push(ast);
                } else {
                    return Err(ParseError::NoPrev(pos))
                }
            },
            '(' => {
                let prev: Vec<AST> = take(&mut seq);
                let prev_or: Vec<AST> = take(&mut seq_or);
                stack.push((prev, prev_or));
            },
            ')' => {
                if let Some((mut prev, prev_or)) = stack.pop() {
                    if !seq.is_empty() {
                        seq_or.push(AST::Seq(seq));
                    }

                    if let Some(ast) = fold_or(seq_or) {
                        prev.push(ast);
                    }

                    seq = prev;
                    seq_or = prev_or;
                } else {
                    return Err(ParseError::InvalidRightParen(pos));
                }
            }
            '|' => {
                let prev: Vec<AST> = take(&mut seq);
                seq_or.push(AST::Seq(prev));
            },
            '\\' => is_escape = true,
            '.' => seq.push(AST::Period),
            _ => seq.push(AST::Char(c))
        };
    }
    // 閉じカッコが足りないエラー
    if !stack.is_empty() {
        return Err(ParseError::NoRightParen)
    }

    if !seq.is_empty() {
        seq_or.push(AST::Seq(seq));
    }

    if let Some(ast) = fold_or(seq_or) {
        println!("{:?}", ast);
        Ok(ast)
    } else {
        Err(ParseError::Empty)
    }
}

// ----- テストコード・試し -----

#[test]
fn try_take() {
    // take()の動作確認。
    // b に a の値 = Some(10) を代入。
    // その後、a に None を代入。
    let mut a: Option<i32> = Some(10);
    let b: Option<i32> = take(&mut a);

    assert_eq!(a, None); // a == None を確認するテスト
    assert_eq!(b, Some(10)); // b == Some(10) を確認するテスト
}

#[test]
fn test_parse_escape_success() {
    let expect: AST = AST::Char('\\');

    // テスト対象を実行
    let actual: AST = parse_escape(0, '\\').unwrap();
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
    let expect: AST = AST::Plus(Box::new(AST::Char('a')));

    // テスト対象を実行
    let ast: AST = AST::Char('a');
    let actual: AST = parse_qualifier(Qualifier::Plus, ast);
    assert_eq!(actual, expect);
}

#[test]
fn test_parse_qualifier_star() {
    let expect: AST = AST::Star(Box::new(AST::Char('a')));

    // テスト対象を実行
    let ast: AST = AST::Char('a');
    let actual: AST = parse_qualifier(Qualifier::Star, ast);
    assert_eq!(actual, expect);
}

#[test]
fn test_parse_qualifier_question() {
    let expect: AST = AST::Question(Box::new(AST::Char('a')));

    // テスト対象を実行
    let ast: AST = AST::Char('a');
    let actual: AST = parse_qualifier(Qualifier::Question, ast);
    assert_eq!(actual, expect);
}

#[test]
fn test_fold_or_if_true() {
    // パターン "a|b|c" を想定し、データ準備
    let seq: Vec<AST> = vec![AST::Char('a'), AST::Char('b'), AST::Char('c')];

    // a|b|c をパースした場合、以下のASTができる
    // AST::Or(AST::Char('a'), AST::Or(AST::Char('b'), AST::Char('c')))
    // 上記のASTを用意するため、データを定義
    let left: AST = AST::Char('a');
    let right: AST = AST::Or(Box::new(AST::Char('b')), Box::new(AST::Char('c')));
    let expect: AST = AST::Or(Box::new(left), Box::new(right));

    let actual: AST = fold_or(seq).unwrap();

    assert_eq!(actual, expect);
}

#[test]
fn test_fold_or_if_false() {
    // 長さ 1 の配列を準備
    let mut seq: Vec<AST> = Vec::new();
    seq.push(AST::Char('a'));

    let expect: AST = AST::Char('a');

    // テスト対象を実行
    let actual = fold_or(seq).unwrap();

    assert_eq!(actual, expect);
}

#[test]
fn test_convert_char_to_qualifier_plus() {
    let expect: Qualifier = Qualifier::Plus;
    // テスト対象を実行
    let actual: Qualifier = convert_char_to_qualifier('+').unwrap();
    assert_eq!(actual, expect);
}

#[test]
fn test_convert_char_to_qualifier_star() {
    let expect: Qualifier = Qualifier::Star;
    // テスト対象を実行
    let actual: Qualifier = convert_char_to_qualifier('*').unwrap();
    assert_eq!(actual, expect);
}

#[test]
fn test_convert_char_to_qualifier_question() {
    let expect: Qualifier = Qualifier::Question;
    // テスト対象を実行
    let actual: Qualifier = convert_char_to_qualifier('?').unwrap();
    assert_eq!(actual, expect);
}

#[test]
fn test_convert_char_to_qualifier_none() {
    let none = convert_char_to_qualifier('c');
    assert_eq!(none, None);
}

#[test]
fn test_parse_normal_string() {
    // ----- "abc" が入力されたケース -----
    let expect: AST = AST::Seq(vec![AST::Char('a'), AST::Char('b'), AST::Char('c')]);
    // テスト対象を実行
    let pattern: &str = "abc";
    let actual: AST = parse(pattern).unwrap();
    assert_eq!(actual, expect);
}

#[test]
fn test_parse_contain_qualifier() {
    // ----- "abc+" が入力されたケース -----
    let expect: AST = AST::Seq(vec![
        AST::Char('a'),
        AST::Char('b'),
        AST::Plus(Box::new(AST::Char('c')))
    ]);
    // テスト対象を実行
    let pattern: &str = "abc+";
    let actual: AST = parse(pattern).unwrap();
    assert_eq!(actual, expect);    
}

#[test]
fn test_parse_contain_or() {
    // ----- "abc|def|ghi" が入力されたケース-----
    let abc: AST = AST::Seq(vec![AST::Char('a'), AST::Char('b'), AST::Char('c')]);
    let def: AST = AST::Seq(vec![AST::Char('d'), AST::Char('e'), AST::Char('f')]);
    let ghi: AST = AST::Seq(vec![AST::Char('g'), AST::Char('h'), AST::Char('i')]);

    let expect: AST = AST::Or(
        Box::new(abc),
        Box::new(AST::Or(
            Box::new(def),
            Box::new(ghi),
        ))
    );
    // テスト対象を実行
    let pattern: &str= "abc|def|ghi";
    let actual: AST = parse(pattern).unwrap();
    assert_eq!(actual, expect);
}

#[test]
fn test_parse_contain_paran() {
    // ----- "abc(def|ghi)" が入力されたケース-----
    let expect: AST = AST::Seq(vec![
        AST::Char('a'),
        AST::Char('b'),
        AST::Char('c'),
        AST::Or(
            Box::new(AST::Seq(vec![AST::Char('d'), AST::Char('e'), AST::Char('f')])),
            Box::new(AST::Seq(vec![AST::Char('g'), AST::Char('h'), AST::Char('i')]))
        )
    ]);
    // テスト対象を実行
    let pattern: &str = "abc(def|ghi)";
    let actual: AST = parse(pattern).unwrap();

    assert_eq!(actual, expect);
}

#[test]
fn test_parse_contain_period() {
    // ----- "a.c" が入力されたケース-----
    let expect: AST = AST::Seq(vec![
        AST::Char('a'),
        AST::Period,
        AST::Char('c'),
    ]);
    // テスト対象を実行
    let pattern: &str = "a.c";
    let actual: AST = parse(pattern).unwrap();

    assert_eq!(actual, expect);
}