//! Parse regular expression.

use std::{
    mem::take,
};
use crate::error::ParseError;

#[derive(Debug, PartialEq)]
enum AST {
    Char(char),
    Plus(Box<AST>),
    Star(Box<AST>),
    Question(Box<AST>),
    Or(Box<AST>, Box<AST>),
    Seq(Vec<AST>),
}

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

fn parse_qualifier(seq: &mut Vec<AST>, pos: usize, qualifier: Qualifier) -> Result<(), ParseError>{
    if let Some(prev) = seq.pop() {
        let ast: AST = match qualifier {
            Qualifier::Plus => AST::Plus(Box::new(prev)),
            Qualifier::Star => AST::Star(Box::new(prev)),
            Qualifier::Question => AST::Question(Box::new(prev)),
        };
        seq.push(ast);
        Ok(())
    } else {
        Err(ParseError::NoPrev(pos))
    }
}

fn _parse_qualifier(qualifier: Qualifier, prev: AST) -> AST{
    match qualifier {
        Qualifier::Plus => AST::Plus(Box::new(prev)),
        Qualifier::Star => AST::Star(Box::new(prev)),
        Qualifier::Question => AST::Question(Box::new(prev)),
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
fn test_parse_qualifier_success() {
    // パターン "abc+" を想定し、データ準備
    let mut actual: Vec<AST> = Vec::new();
    actual.push(AST::Char('a'));
    actual.push(AST::Char('b'));
    actual.push(AST::Char('c'));

    // abc+ をパースした場合、以下の配列ができる
    // [ AST::Char('a'), AST::Char('b'), AST::Plus(AST::Char('c')) ]
    // 上記の配列を用意するため、定義・データ挿入
    let mut expect: Vec<AST> = Vec::new();
    expect.push(AST::Char('a'));
    expect.push(AST::Char('b'));
    expect.push(AST::Plus(Box::new(AST::Char('c'))));

    // テスト対象を実行
    parse_qualifier(&mut actual , 0, Qualifier::Plus).unwrap();

    assert_eq!(actual, expect);
}

#[test]
fn test_parse_qualifier_failure() {
    // 空の Vector を準備
    let mut vec: Vec<AST> = Vec::new();
    // テスト対象を実行
    let actual: Result<(), ParseError> = parse_qualifier(&mut vec , 1, Qualifier::Plus);
    let expect:Result<_, ParseError>= Err(ParseError::NoPrev(1));

    assert_eq!(actual, expect);
}

#[test]
fn test_fold_or_if_true() {
    // パターン "a|b|c" を想定し、データ準備
    let mut seq: Vec<AST> = Vec::new();
    seq.push(AST::Char('a'));
    seq.push(AST::Char('b'));
    seq.push(AST::Char('c'));

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