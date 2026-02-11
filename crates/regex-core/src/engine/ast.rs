//! AST definitions for the regex engine.

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
