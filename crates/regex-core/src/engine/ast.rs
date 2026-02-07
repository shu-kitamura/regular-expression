//! AST definitions for the regex engine.
//!
//! Currently this is not wired to the parser/compiler and provides type
//! definitions only.
#![allow(dead_code)]

/// Inclusive character range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharRange {
    pub start: char,
    pub end: char,
}

impl CharRange {
    pub fn new(start: char, end: char) -> Option<Self> {
        if start <= end {
            Some(Self { start, end })
        } else {
            None
        }
    }
}

/// Character class.
///
/// `ranges` represents inclusive `[start, end]` spans.
/// If `negated` is true, this is a negated class (`[^...]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharClass {
    pub ranges: Vec<CharRange>,
    pub negated: bool,
}

impl CharClass {
    pub fn new(ranges: Vec<CharRange>, negated: bool) -> Self {
        Self { ranges, negated }
    }
}

/// Zero-width assertion kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Predicate {
    StartOfLine,
    EndOfLine,
    StartOfText,
    EndOfText,
    WordBoundary,
    NonWordBoundary,
}

/// Regex abstract syntax tree.
///
/// - Empty
/// - CharClass(..., neg)
/// - Assertion(Predicate)
/// - Capture(..., index)
/// - ZeroOrMore / OneOrMore / ZeroOrOne (greedy)
/// - Repeat(..., greedy, min, max)
/// - Concat
/// - Alternate
/// - Backreference
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    Empty,
    CharClass(CharClass),
    Assertion(Predicate),
    Capture {
        expr: Box<Ast>,
        index: usize,
    },
    ZeroOrMore {
        expr: Box<Ast>,
        greedy: bool,
    },
    OneOrMore {
        expr: Box<Ast>,
        greedy: bool,
    },
    ZeroOrOne {
        expr: Box<Ast>,
        greedy: bool,
    },
    Repeat {
        expr: Box<Ast>,
        greedy: bool,
        min: u32,
        max: Option<u32>,
    },
    Concat(Vec<Ast>),
    Alternate(Box<Ast>, Box<Ast>),
    Backreference(usize),
}
