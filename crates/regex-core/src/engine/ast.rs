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

/// Aggregate analysis results derived from one AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AstAnalysis {
    /// Conservative must substrings that appear in every successful match.
    pub must_literals: Vec<String>,
    /// Candidate literal substrings used as prefilter hints.
    pub needles: Vec<String>,
    /// Whether this pattern can match the empty string.
    pub nullable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AstAnalysisSet {
    must_literals: BTreeSet<String>,
    needles: BTreeSet<String>,
    nullable: bool,
}

/// Analyzes `ast` and returns deterministic `must_literals`, `needles`, and `nullable`.
pub(crate) fn analyze_ast(ast: &Ast) -> AstAnalysis {
    let result = analyze_ast_set(ast);
    AstAnalysis {
        must_literals: literal_set_to_vec(result.must_literals),
        needles: literal_set_to_vec(result.needles),
        nullable: result.nullable,
    }
}

/// Extracts conservative must substrings from an AST.
///
/// Each returned string is guaranteed to appear in every successful match.
#[allow(dead_code)]
pub(crate) fn extract_must_literals(ast: &Ast) -> Vec<String> {
    analyze_ast(ast).must_literals
}

/// Extracts candidate literal substrings from an AST.
#[allow(dead_code)]
pub(crate) fn extract_needles(ast: &Ast) -> Vec<String> {
    analyze_ast(ast).needles
}

/// Returns whether the pattern represented by `ast` can match an empty string.
#[allow(dead_code)]
pub(crate) fn is_nullable(ast: &Ast) -> bool {
    analyze_ast(ast).nullable
}

fn analyze_ast_set(ast: &Ast) -> AstAnalysisSet {
    match ast {
        Ast::Empty | Ast::Assertion(_) => AstAnalysisSet {
            must_literals: BTreeSet::new(),
            needles: BTreeSet::new(),
            nullable: true,
        },
        Ast::Backreference(_) => AstAnalysisSet {
            must_literals: BTreeSet::new(),
            needles: BTreeSet::new(),
            nullable: false,
        },
        Ast::CharClass(class) => analyze_char_class(class),
        Ast::Capture { expr, .. } => analyze_ast_set(expr),
        Ast::ZeroOrMore { expr, .. } | Ast::ZeroOrOne { expr, .. } => {
            let child = analyze_ast_set(expr);
            AstAnalysisSet {
                must_literals: BTreeSet::new(),
                needles: child.needles,
                nullable: true,
            }
        }
        Ast::OneOrMore { expr, .. } => analyze_ast_set(expr),
        Ast::Repeat { expr, min, .. } => {
            let child = analyze_ast_set(expr);
            AstAnalysisSet {
                must_literals: if *min == 0 {
                    BTreeSet::new()
                } else {
                    child.must_literals
                },
                needles: child.needles,
                nullable: if *min == 0 { true } else { child.nullable },
            }
        }
        Ast::Concat(exprs) => analyze_concat(exprs),
        Ast::Alternate(left, right) => analyze_alternate(left, right),
    }
}

fn analyze_char_class(class: &CharClass) -> AstAnalysisSet {
    let mut must_literals = BTreeSet::new();
    let mut needles = BTreeSet::new();
    if let Some(literal) = class_single_literal(class) {
        let literal = literal.to_string();
        must_literals.insert(literal.clone());
        needles.insert(literal);
    }
    AstAnalysisSet {
        must_literals,
        needles,
        nullable: false,
    }
}

fn analyze_concat(exprs: &[Ast]) -> AstAnalysisSet {
    let mut must_literals = BTreeSet::new();
    let mut needles = BTreeSet::new();
    let mut literal_run = String::new();
    let mut nullable = true;

    for expr in exprs {
        let child = analyze_ast_set(expr);
        nullable &= child.nullable;

        if let Some(literal) = ast_single_literal(expr) {
            literal_run.push_str(&literal);
            continue;
        }

        flush_literal_run(&mut must_literals, &mut needles, &mut literal_run);
        union_literal_sets(&mut must_literals, child.must_literals);
        union_literal_sets(&mut needles, child.needles);
    }

    flush_literal_run(&mut must_literals, &mut needles, &mut literal_run);
    AstAnalysisSet {
        must_literals,
        needles,
        nullable,
    }
}

fn analyze_alternate(left: &Ast, right: &Ast) -> AstAnalysisSet {
    let left = analyze_ast_set(left);
    let right = analyze_ast_set(right);

    let must_literals = intersect_literal_sets(left.must_literals, right.must_literals);
    let mut needles = left.needles;
    union_literal_sets(&mut needles, right.needles);

    AstAnalysisSet {
        must_literals,
        needles,
        nullable: left.nullable || right.nullable,
    }
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

fn flush_literal_run(
    must_literals: &mut BTreeSet<String>,
    needles: &mut BTreeSet<String>,
    literal_run: &mut String,
) {
    if literal_run.is_empty() {
        return;
    }

    let literal = std::mem::take(literal_run);
    must_literals.insert(literal.clone());
    prune_literal_set(must_literals);
    needles.insert(literal);
    prune_literal_set(needles);
}

fn union_literal_sets(dst: &mut BTreeSet<String>, src: BTreeSet<String>) {
    dst.extend(src);
    prune_literal_set(dst);
}

fn intersect_literal_sets(left: BTreeSet<String>, right: BTreeSet<String>) -> BTreeSet<String> {
    let mut intersection = BTreeSet::new();
    for literal in left {
        if right.contains(&literal) {
            intersection.insert(literal);
        }
    }
    prune_literal_set(&mut intersection);
    intersection
}

fn prune_literal_set(set: &mut BTreeSet<String>) {
    if set.len() <= MUST_LITERAL_LIMIT {
        return;
    }

    let mut literals: Vec<String> = set.iter().cloned().collect();
    literals.sort_by(|a, b| compare_literals(a, b));
    literals.truncate(MUST_LITERAL_LIMIT);
    *set = literals.into_iter().collect();
}

fn literal_set_to_vec(set: BTreeSet<String>) -> Vec<String> {
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
    use super::{
        MUST_LITERAL_LIMIT, analyze_ast, extract_must_literals, extract_needles, is_nullable,
    };
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

    #[test]
    fn test_analyze_ast_alternate_needles_and_nullable() {
        let ast = parse("(abc|def)").unwrap();
        let actual = analyze_ast(&ast);
        assert_eq!(actual.needles, vec!["abc".to_string(), "def".to_string()]);
        assert!(!actual.nullable);
    }

    #[test]
    fn test_analyze_ast_zero_or_more_nullable_and_needles() {
        let ast = parse("(abc)*").unwrap();
        let actual = analyze_ast(&ast);
        assert!(actual.nullable);
        assert_eq!(actual.needles, vec!["abc".to_string()]);
    }

    #[test]
    fn test_is_nullable_repeat_zero_to_n() {
        let ast = parse("(abc|def){0,3}").unwrap();
        assert!(is_nullable(&ast));
    }

    #[test]
    fn test_is_nullable_ab_plus_c() {
        let ast = parse("ab+c").unwrap();
        assert!(!is_nullable(&ast));
    }

    #[test]
    fn test_extract_needles_limit_prefers_longer_then_lexicographic() {
        let pattern = [
            "ppp", "aaa", "qqq", "bbb", "ccc", "ddd", "eee", "fff", "ggg", "hhh", "iii", "jjj",
            "kkk", "lll", "mmm", "nnn", "ooo", "zzzz",
        ]
        .join("|");
        let ast = parse(&pattern).unwrap();

        let actual = extract_needles(&ast);
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

    #[test]
    fn test_backreference_analysis_is_conservative() {
        let ast = parse("\\1").unwrap();
        let actual = analyze_ast(&ast);
        assert!(!actual.nullable);
        assert!(actual.needles.is_empty());
    }
}
