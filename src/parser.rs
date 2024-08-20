//! Parse regular expression.

use std::mem::take;
use crate::error::ParseError;

#[derive(Debug, PartialEq)]
pub enum AST {
    Char(char),
    Plus(Box<AST>),
    Star(Box<AST>),
    Question(Box<AST>),
    Or(Box<AST>, Box<AST>),
    Seq(Vec<AST>),
}

#[derive(Debug, PartialEq)]
enum Qualifier {
    Plus,
    Star,
    Question,
}

/// 特殊文字をエスケープする関数
fn parse_escape(pos: usize, c: char) -> Result<AST, ParseError> {
    match c {
        '\\' | '(' | ')' | '|' | '+' | '*' | '?' => Ok(AST::Char(c)),
        _ => Err(ParseError::InvalidEscape(pos, c)),
    }
}

fn parse_qualifier(qualifier: Qualifier, prev: AST) -> AST{
    match qualifier {
        Qualifier::Plus => AST::Plus(Box::new(prev)),
        Qualifier::Star => AST::Star(Box::new(prev)),
        Qualifier::Question => AST::Question(Box::new(prev)),
    }
}

fn convert_char_to_qualifier(c: char) -> Option<Qualifier> {
    match c {
        '+' => Some(Qualifier::Plus),
        '*' => Some(Qualifier::Star),
        '?' => Some(Qualifier::Question),
        _ => None,
    }
}

/// Or(|) を結合された複数の式をASTに変換する関数
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
                let prev = take(&mut seq);
                let prev_or = take(&mut seq_or);
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
        Ok(ast)
    } else {
        Err(ParseError::Empty)
    }
}

// ----- test and try code -----

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
    // テスト対象を実行
    let actual: AST = parse_escape(0, '\\').unwrap();
    let expect: AST = AST::Char('\\');

    assert_eq!(actual, expect);
}

#[test]
fn test_parse_escape_failure() {
    // テスト対象を実行
    let actual = parse_escape(0, 'a');
    let expect = Err(ParseError::InvalidEscape(0, 'a'));

    assert_eq!(actual, expect);
}

#[test]
fn test_parse_qualifier_plus() {
    // テスト対象を実行
    let ast: AST = AST::Char('a');
    let actual: AST = parse_qualifier(Qualifier::Plus, ast);

    let expect: AST = AST::Plus(Box::new(AST::Char('a')));

    assert_eq!(actual, expect);
}

#[test]
fn test_parse_qualifier_star() {
    // テスト対象を実行
    let ast: AST = AST::Char('a');
    let actual: AST = parse_qualifier(Qualifier::Star, ast);

    let expect: AST = AST::Star(Box::new(AST::Char('a')));

    assert_eq!(actual, expect);
}

#[test]
fn test_parse_qualifier_question() {
    // テスト対象を実行
    let ast: AST = AST::Char('a');
    let actual: AST = parse_qualifier(Qualifier::Question, ast);

    let expect: AST = AST::Question(Box::new(AST::Char('a')));

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
    let plus: Qualifier = convert_char_to_qualifier('+').unwrap();
    assert_eq!(plus, Qualifier::Plus);
}

#[test]
fn test_convert_char_to_qualifier_star() {
    let star: Qualifier = convert_char_to_qualifier('*').unwrap();
    assert_eq!(star, Qualifier::Star);
}

#[test]
fn test_convert_char_to_qualifier_question() {
    let question: Qualifier = convert_char_to_qualifier('?').unwrap();
    assert_eq!(question, Qualifier::Question);
}

#[test]
fn test_convert_char_to_qualifier_none() {
    let none = convert_char_to_qualifier('c');
    assert_eq!(none, None);
}

#[test]
fn test_parse_normal_string() {
    // ----- "abc" が入力されたケース -----
    let expect1: AST = AST::Seq(vec![AST::Char('a'), AST::Char('b'), AST::Char('c')]);

    let pattern1: &str = "abc";
    let actual1: AST = parse(pattern1).unwrap();
    assert_eq!(actual1, expect1);

    // ----- "abc+" が入力されたケース -----
    let expect2: AST = AST::Seq(vec![AST::Char('a'), AST::Char('b'), AST::Plus(Box::new(AST::Char('c')))]);

    let pattern2: &str = "abc+";
    let actual2: AST = parse(pattern2).unwrap();
    assert_eq!(actual2, expect2);

    // ----- "abc|def|ghi" が入力されたケース-----
    let abc: AST = AST::Seq(vec![AST::Char('a'), AST::Char('b'), AST::Char('c')]);
    let def: AST = AST::Seq(vec![AST::Char('d'), AST::Char('e'), AST::Char('f')]);
    let ghi: AST = AST::Seq(vec![AST::Char('g'), AST::Char('h'), AST::Char('i')]);

    let expect3: AST = AST::Or(
        Box::new(abc),
        Box::new(AST::Or(
            Box::new(def),
            Box::new(ghi),
        ))
    );

    let pattern3: &str= "abc|def|ghi";
    let actual3: AST = parse(pattern3).unwrap();
    assert_eq!(actual3, expect3);

    // ----- "abc(def|ghi)" が入力されたケース-----
    let expect4: AST = AST::Seq(vec![
        AST::Char('a'),
        AST::Char('b'),
        AST::Char('c'),
        AST::Or(
            Box::new(AST::Seq(vec![AST::Char('d'), AST::Char('e'), AST::Char('f')])),
            Box::new(AST::Seq(vec![AST::Char('g'), AST::Char('h'), AST::Char('i')]))
        )
    ]);
    let pattern4: &str = "abc(def|ghi)";
    let actual4: AST = parse(pattern4).unwrap();

    assert_eq!(actual4, expect4);
}