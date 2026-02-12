//! AST definitions for the regex engine.

use std::{cmp::Ordering, collections::BTreeSet};

/// Maximum number of must literals to retain.
pub(crate) const MUST_LITERAL_LIMIT: usize = 16;

/// Inclusive character range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharRange {
    /// Inclusive start character.
    pub start: char,
    /// Inclusive end character.
    pub end: char,
}

/// Character class.
///
/// `ranges` represents inclusive `[start, end]` spans.
/// If `negated` is true, this is a negated class (`[^...]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharClass {
    /// Inclusive character ranges that belong to this class.
    pub ranges: Vec<CharRange>,
    /// Whether the class is negated (`[^...]`).
    pub negated: bool,
}

impl CharClass {
    /// Creates a character class from ranges and a negation flag.
    pub fn new(ranges: Vec<CharRange>, negated: bool) -> Self {
        Self { ranges, negated }
    }
}

/// Zero-width assertion kinds.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Predicate {
    /// Line start assertion (`^`).
    StartOfLine,
    /// Line end assertion (`$`).
    EndOfLine,
    /// Text start assertion.
    StartOfText,
    /// Text end assertion.
    EndOfText,
    /// Word-boundary assertion.
    WordBoundary,
    /// Non-word-boundary assertion.
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
    /// Empty expression.
    Empty,
    /// Character class match node.
    CharClass(CharClass),
    /// Zero-width assertion node.
    Assertion(Predicate),
    /// Capturing group node.
    Capture {
        /// Inner expression.
        expr: Box<Ast>,
        /// Capture group index (1-based).
        index: usize,
    },
    /// Greedy `*` quantifier node.
    ZeroOrMore {
        /// Inner expression.
        expr: Box<Ast>,
        /// Greedy flag.
        greedy: bool,
    },
    /// Greedy `+` quantifier node.
    OneOrMore {
        /// Inner expression.
        expr: Box<Ast>,
        /// Greedy flag.
        greedy: bool,
    },
    /// Greedy `?` quantifier node.
    ZeroOrOne {
        /// Inner expression.
        expr: Box<Ast>,
        /// Greedy flag.
        greedy: bool,
    },
    /// Repeat quantifier node (`{m}`, `{m,n}`, `{m,}`).
    Repeat {
        /// Inner expression.
        expr: Box<Ast>,
        /// Greedy flag.
        greedy: bool,
        /// Minimum repetition count.
        min: u32,
        /// Optional maximum repetition count.
        max: Option<u32>,
    },
    /// Concatenation node.
    Concat(Vec<Ast>),
    /// Alternation node (`|`).
    Alternate(Box<Ast>, Box<Ast>),
    /// Backreference node (`\1`, `\2`, ...).
    Backreference(usize),
}

/// Extracts conservative must substrings from an AST.
///
/// Each returned string is guaranteed to appear in every successful match.
/// The result is trimmed to at most `MUST_LITERAL_LIMIT` entries by:
/// 1. longer byte length first
/// 2. lexicographic byte order for equal lengths
pub(crate) fn extract_must_literals(ast: &Ast) -> Vec<String> {
    let mut must = extract_must_literal_set(ast);
    prune_must_literal_set(&mut must);
    must_literal_set_to_vec(must)
}

fn extract_must_literal_set(ast: &Ast) -> BTreeSet<String> {
    match ast {
        Ast::Empty | Ast::Assertion(_) | Ast::Backreference(_) => BTreeSet::new(),
        Ast::CharClass(class) => {
            let mut must = BTreeSet::new();
            if let Some(literal) = class_single_literal(class) {
                must.insert(literal.to_string());
            }
            must
        }
        Ast::Capture { expr, .. } => extract_must_literal_set(expr),
        Ast::ZeroOrMore { .. } | Ast::ZeroOrOne { .. } => BTreeSet::new(),
        Ast::OneOrMore { expr, .. } => extract_must_literal_set(expr),
        Ast::Repeat { expr, min, .. } => {
            if *min == 0 {
                BTreeSet::new()
            } else {
                extract_must_literal_set(expr)
            }
        }
        Ast::Concat(exprs) => extract_concat_must_literals(exprs),
        Ast::Alternate(left, right) => {
            let left_set = extract_must_literal_set(left);
            let right_set = extract_must_literal_set(right);
            intersect_must_literal_sets(left_set, right_set)
        }
    }
}

fn extract_concat_must_literals(exprs: &[Ast]) -> BTreeSet<String> {
    let mut must = BTreeSet::new();
    let mut literal_run = String::new();

    for expr in exprs {
        if let Some(literal) = ast_single_literal(expr) {
            literal_run.push_str(&literal);
            continue;
        }

        flush_literal_run(&mut must, &mut literal_run);
        union_must_literal_sets(&mut must, extract_must_literal_set(expr));
    }

    flush_literal_run(&mut must, &mut literal_run);
    prune_must_literal_set(&mut must);
    must
}

fn ast_single_literal(ast: &Ast) -> Option<String> {
    let Ast::CharClass(class) = ast else {
        return None;
    };
    class_single_literal(class).map(|c| c.to_string())
}

fn class_single_literal(class: &CharClass) -> Option<char> {
    if class.negated || class.ranges.len() != 1 {
        return None;
    }

    let range = class.ranges.first()?;
    if range.start == range.end {
        Some(range.start)
    } else {
        None
    }
}

fn flush_literal_run(set: &mut BTreeSet<String>, literal_run: &mut String) {
    if literal_run.is_empty() {
        return;
    }

    set.insert(std::mem::take(literal_run));
    prune_must_literal_set(set);
}

fn union_must_literal_sets(dst: &mut BTreeSet<String>, src: BTreeSet<String>) {
    dst.extend(src);
    prune_must_literal_set(dst);
}

fn intersect_must_literal_sets(
    left: BTreeSet<String>,
    right: BTreeSet<String>,
) -> BTreeSet<String> {
    let mut intersection = BTreeSet::new();
    for literal in left {
        if right.contains(&literal) {
            intersection.insert(literal);
        }
    }
    prune_must_literal_set(&mut intersection);
    intersection
}

fn prune_must_literal_set(set: &mut BTreeSet<String>) {
    if set.len() <= MUST_LITERAL_LIMIT {
        return;
    }

    let mut literals: Vec<String> = set.iter().cloned().collect();
    literals.sort_by(|a, b| compare_literals(a, b));
    literals.truncate(MUST_LITERAL_LIMIT);
    *set = literals.into_iter().collect();
}

fn must_literal_set_to_vec(set: BTreeSet<String>) -> Vec<String> {
    let mut literals: Vec<String> = set.into_iter().collect();
    literals.sort_by(|a, b| compare_literals(a, b));
    literals.truncate(MUST_LITERAL_LIMIT);
    literals
}

fn compare_literals(a: &str, b: &str) -> Ordering {
    b.len()
        .cmp(&a.len())
        .then_with(|| a.as_bytes().cmp(b.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{MUST_LITERAL_LIMIT, extract_must_literals};
    use crate::engine::parser::parse;

    #[test]
    fn test_extract_must_literals_dot_star_abc_dot_star() {
        let ast = parse(".*abc.*").unwrap();
        let actual = extract_must_literals(&ast);
        assert_eq!(actual, vec!["abc".to_string()]);
    }

    #[test]
    fn test_extract_must_literals_alternate_no_common_literal() {
        let ast = parse("(abc|def)").unwrap();
        let actual = extract_must_literals(&ast);
        assert_eq!(actual, Vec::<String>::new());
    }

    #[test]
    fn test_extract_must_literals_ab_star_c() {
        let ast = parse("ab*c").unwrap();
        let actual = extract_must_literals(&ast);
        assert_eq!(actual, vec!["a".to_string(), "c".to_string()]);
    }

    #[test]
    fn test_extract_must_literals_a_class_z() {
        let ast = parse("a[a-z]z").unwrap();
        let actual = extract_must_literals(&ast);
        assert_eq!(actual, vec!["a".to_string(), "z".to_string()]);
    }

    #[test]
    fn test_extract_must_literals_limit_prefers_longer_then_lexicographic() {
        let pattern = [
            "ppp", "aaa", "qqq", "bbb", "ccc", "ddd", "eee", "fff", "ggg", "hhh", "iii", "jjj",
            "kkk", "lll", "mmm", "nnn", "ooo", "zzzz",
        ]
        .join(".*");
        let ast = parse(&pattern).unwrap();

        let actual = extract_must_literals(&ast);
        assert_eq!(actual.len(), MUST_LITERAL_LIMIT);
        let expected = [
            "zzzz", "aaa", "bbb", "ccc", "ddd", "eee", "fff", "ggg", "hhh", "iii", "jjj", "kkk",
            "lll", "mmm", "nnn", "ooo",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}
