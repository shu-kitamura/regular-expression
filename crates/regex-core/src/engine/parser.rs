//! Recursive-descent parser for regex patterns.
//!
//! The parser converts a pattern string into an `Ast` used by the compiler.
#![allow(dead_code)]

use crate::engine::ast::{Ast, CharClass, CharRange, Predicate};
use thiserror::Error;

const SPECIAL_CHARS: [char; 14] = [
    '*', '+', '?', '|', '(', ')', '[', ']', '{', '}', '\\', '.', '^', '$',
];

/// Errors that can occur while parsing a pattern string.
#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    /// Input ended while the parser still expected more tokens.
    #[error("unexpected end of input")]
    UnexpectedEnd,
    /// Encountered an unexpected character in the current context.
    #[error("unexpected character: {0}")]
    UnexpectedChar(char),
    /// Invalid repetition operator syntax.
    #[error("invalid repeat operator")]
    InvalidRepeatOp,
    /// Invalid numeric range in repetition syntax.
    #[error("invalid repeat size")]
    InvalidRepeatSize,
    /// Missing closing `]` for a character class.
    #[error("missing closing bracket ']'")]
    MissingBracket,
    /// Missing closing `)` for a group.
    #[error("missing closing parenthesis ')'")]
    MissingParenthesis,
    /// Trailing `\` at the end of the pattern.
    #[error("trailing backslash")]
    TrailingBackslash,
    /// Invalid character class (for example, reversed range).
    #[error("invalid character class")]
    InvalidCharClass,
    /// Missing numeric argument in repetition syntax.
    #[error("missing repeat argument")]
    MissingRepeatArgument,
}

/// Internal parser state.
#[derive(Debug, Clone)]
struct Parser {
    /// Pattern characters as a random-access array.
    input: Vec<char>,
    /// Current cursor position in `input`.
    pos: usize,
    /// Next capture-group index (1-based).
    captures: usize,
}

/// Parses `regex` and returns its AST representation.
pub fn parse(regex: &str) -> Result<Ast, ParseError> {
    let mut parser = Parser::new(regex);
    let ast = parser.parse_expression()?;
    if parser.peek().is_some() {
        return Err(ParseError::UnexpectedChar(parser.peek().unwrap()));
    }
    Ok(ast)
}

impl Parser {
    /// Creates a parser from a pattern string.
    fn new(regex: &str) -> Self {
        Self {
            input: regex.chars().collect(),
            pos: 0,
            captures: 1,
        }
    }

    /// Parses alternation expressions: `seq ('|' seq)*`.
    fn parse_expression(&mut self) -> Result<Ast, ParseError> {
        let mut left = self.parse_sequence()?;
        while self.consume_if('|') {
            let right = self.parse_sequence()?;
            left = Ast::Alternate(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// Parses concatenated terms until `|`, `)`, or end-of-input.
    fn parse_sequence(&mut self) -> Result<Ast, ParseError> {
        let mut sequence = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == '|' || ch == ')' {
                break;
            }
            let term = self.parse_term()?;
            sequence.push(term);
        }
        Ok(match sequence.len() {
            0 => Ast::Empty,
            1 => sequence.pop().unwrap(),
            _ => Ast::Concat(sequence),
        })
    }

    /// Parses one factor followed by an optional quantifier.
    fn parse_term(&mut self) -> Result<Ast, ParseError> {
        let mut base = self.parse_factor()?;
        match self.peek() {
            Some('*') => {
                self.next();
                if self.peek() == Some('?') {
                    return Err(ParseError::InvalidRepeatOp);
                }
                let greedy = true;
                base = Ast::ZeroOrMore {
                    expr: Box::new(base),
                    greedy,
                };
            }
            Some('+') => {
                self.next();
                if self.peek() == Some('?') {
                    return Err(ParseError::InvalidRepeatOp);
                }
                let greedy = true;
                base = Ast::OneOrMore {
                    expr: Box::new(base),
                    greedy,
                };
            }
            Some('?') => {
                self.next();
                if self.peek() == Some('?') {
                    return Err(ParseError::InvalidRepeatOp);
                }
                let greedy = true;
                base = Ast::ZeroOrOne {
                    expr: Box::new(base),
                    greedy,
                };
            }
            Some('{') => {
                self.next();
                let (min, max) = self.parse_repeat()?;
                if self.peek() == Some('?') {
                    return Err(ParseError::InvalidRepeatOp);
                }
                let greedy = true;
                base = Ast::Repeat {
                    expr: Box::new(base),
                    greedy,
                    min,
                    max,
                };
            }
            _ => {}
        }
        Ok(base)
    }

    /// Parses a primary expression:
    /// group, class, dot, assertion, escape, or literal.
    fn parse_factor(&mut self) -> Result<Ast, ParseError> {
        match self.peek() {
            Some('(') => {
                self.next();
                if self.consume_if('?') {
                    return Err(ParseError::UnexpectedChar('?'));
                }
                let capture_index = self.captures;
                self.captures += 1;
                let expr = self.parse_expression()?;
                if !self.consume_if(')') {
                    return Err(ParseError::MissingParenthesis);
                }
                Ok(Ast::Capture {
                    expr: Box::new(expr),
                    index: capture_index,
                })
            }
            Some('[') => {
                self.next();
                self.parse_char_class()
            }
            Some('.') => {
                self.next();
                Ok(Ast::CharClass(CharClass::new(
                    vec![CharRange {
                        start: '\u{0000}',
                        end: '\u{10FFFF}',
                    }],
                    false,
                )))
            }
            Some('^') => {
                self.next();
                Ok(Ast::Assertion(Predicate::StartOfLine))
            }
            Some('$') => {
                self.next();
                Ok(Ast::Assertion(Predicate::EndOfLine))
            }
            Some('\\') => {
                self.next();
                self.parse_escape()
            }
            Some(ch) if Self::is_special_char(ch) => Err(ParseError::UnexpectedChar(ch)),
            Some(_) => {
                let ch = self.next().ok_or(ParseError::UnexpectedEnd)?;
                Ok(Ast::CharClass(CharClass::new(
                    vec![CharRange { start: ch, end: ch }],
                    false,
                )))
            }
            None => Err(ParseError::UnexpectedEnd),
        }
    }

    /// Parses a character class body after `[` has been consumed.
    fn parse_char_class(&mut self) -> Result<Ast, ParseError> {
        let negated = self.consume_if('^');
        let mut ranges: Vec<CharRange> = Vec::new();
        if self.peek() == Some(']') {
            self.next();
            ranges.push(CharRange {
                start: ']',
                end: ']',
            });
        }
        while let Some(ch) = self.peek() {
            if ch == ']' {
                break;
            }
            let start = self.parse_class_atom()?;
            if self.consume_if('-') {
                if let Some(end) = self.peek() {
                    if end == ']' {
                        ranges.push(CharRange { start, end: start });
                        ranges.push(CharRange {
                            start: '-',
                            end: '-',
                        });
                    } else {
                        let end = self.parse_class_atom()?;
                        if end < start {
                            return Err(ParseError::InvalidCharClass);
                        }
                        ranges.push(CharRange { start, end });
                    }
                } else {
                    return Err(ParseError::MissingBracket);
                }
            } else {
                ranges.push(CharRange { start, end: start });
            }
        }
        if !self.consume_if(']') {
            return Err(ParseError::MissingBracket);
        }
        Ok(Ast::CharClass(CharClass::new(ranges, negated)))
    }

    /// Parses one atom inside a character class, including escaped chars.
    fn parse_class_atom(&mut self) -> Result<char, ParseError> {
        let ch = self.next().ok_or(ParseError::MissingBracket)?;
        if ch != '\\' {
            return Ok(ch);
        }
        let esc = self.next().ok_or(ParseError::TrailingBackslash)?;
        Ok(esc)
    }

    /// Parses an escape sequence.
    ///
    /// `\1`, `\2`, ... are parsed as backreferences.
    /// Other escapes are treated as escaped literals.
    fn parse_escape(&mut self) -> Result<Ast, ParseError> {
        let ch = self.next().ok_or(ParseError::TrailingBackslash)?;
        let ast = match ch {
            '1'..='9' => {
                let mut num: u32 = (ch as u32) - ('0' as u32);
                while let Some(d) = self.peek() {
                    if d.is_ascii_digit() {
                        self.next();
                        num = num * 10 + (d as u32 - ('0' as u32));
                    } else {
                        break;
                    }
                }
                Ast::Backreference(num as usize)
            }
            _ => single_char_class(ch),
        };
        Ok(ast)
    }

    /// Parses repetition arguments in `{m}`, `{m,}`, `{m,n}`.
    fn parse_repeat(&mut self) -> Result<(u32, Option<u32>), ParseError> {
        let min = self.parse_number()?;
        match self.peek() {
            Some('}') => {
                self.next();
                Ok((min, Some(min)))
            }
            Some(',') => {
                self.next();
                match self.peek() {
                    Some('}') => {
                        self.next();
                        Ok((min, None))
                    }
                    _ => {
                        let max = self.parse_number()?;
                        if !self.consume_if('}') {
                            return Err(ParseError::InvalidRepeatOp);
                        }
                        if max < min {
                            return Err(ParseError::InvalidRepeatSize);
                        }
                        Ok((min, Some(max)))
                    }
                }
            }
            _ => Err(ParseError::InvalidRepeatOp),
        }
    }

    /// Parses a decimal number used in repetition arguments.
    fn parse_number(&mut self) -> Result<u32, ParseError> {
        let mut value: u32 = 0;
        let mut has_digits = false;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                has_digits = true;
                self.next();
                value = value * 10 + (ch as u32 - ('0' as u32));
            } else {
                break;
            }
        }
        if has_digits {
            Ok(value)
        } else {
            Err(ParseError::MissingRepeatArgument)
        }
    }

    /// Returns whether `c` has special meaning in this grammar.
    fn is_special_char(c: char) -> bool {
        SPECIAL_CHARS.contains(&c)
    }

    /// Returns the current character without consuming it.
    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    /// Consumes and returns the current character.
    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        Some(ch)
    }

    /// Consumes `expected` if it is the current character.
    fn consume_if(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
}

/// Builds an `Ast::CharClass` representing exactly one literal character.
fn single_char_class(ch: char) -> Ast {
    Ast::CharClass(CharClass::new(
        vec![CharRange { start: ch, end: ch }],
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::{ParseError, Parser, parse, single_char_class};
    use crate::engine::ast::{Ast, CharClass, CharRange, Predicate};

    #[test]
    fn test_parse_abc() {
        let actual = parse("abc").unwrap();
        let expect = Ast::Concat(vec![
            single_char_class('a'),
            single_char_class('b'),
            single_char_class('c'),
        ]);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_alternate_chain() {
        let actual = parse("a|b|c").unwrap();
        let expect = Ast::Alternate(
            Box::new(Ast::Alternate(
                Box::new(single_char_class('a')),
                Box::new(single_char_class('b')),
            )),
            Box::new(single_char_class('c')),
        );
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_alternation_precedence() {
        let actual = parse("ab|cd").unwrap();
        let expect = Ast::Alternate(
            Box::new(Ast::Concat(vec![
                single_char_class('a'),
                single_char_class('b'),
            ])),
            Box::new(Ast::Concat(vec![
                single_char_class('c'),
                single_char_class('d'),
            ])),
        );
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_alternation_empty_side() {
        let actual = parse("a|").unwrap();
        let expect = Ast::Alternate(Box::new(single_char_class('a')), Box::new(Ast::Empty));
        assert_eq!(actual, expect);

        let actual = parse("|a").unwrap();
        let expect = Ast::Alternate(Box::new(Ast::Empty), Box::new(single_char_class('a')));
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_qualifier() {
        let actual_star = parse("a*b").unwrap();
        let expect_star = Ast::Concat(vec![
            Ast::ZeroOrMore {
                expr: Box::new(single_char_class('a')),
                greedy: true,
            },
            single_char_class('b'),
        ]);
        assert_eq!(actual_star, expect_star);

        let actual_plus = parse("a+b").unwrap();
        let expect_plus = Ast::Concat(vec![
            Ast::OneOrMore {
                expr: Box::new(single_char_class('a')),
                greedy: true,
            },
            single_char_class('b'),
        ]);
        assert_eq!(actual_plus, expect_plus);

        let actual_question = parse("a?b").unwrap();
        let expect_question = Ast::Concat(vec![
            Ast::ZeroOrOne {
                expr: Box::new(single_char_class('a')),
                greedy: true,
            },
            single_char_class('b'),
        ]);
        assert_eq!(actual_question, expect_question);
    }

    #[test]
    fn test_parse_group_quantifier() {
        let actual = parse("(ab)*").unwrap();
        let expect = Ast::ZeroOrMore {
            expr: Box::new(Ast::Capture {
                expr: Box::new(Ast::Concat(vec![
                    single_char_class('a'),
                    single_char_class('b'),
                ])),
                index: 1,
            }),
            greedy: true,
        };
        assert_eq!(actual, expect);

        let actual = parse("(ab){2,3}").unwrap();
        let expect = Ast::Repeat {
            expr: Box::new(Ast::Capture {
                expr: Box::new(Ast::Concat(vec![
                    single_char_class('a'),
                    single_char_class('b'),
                ])),
                index: 1,
            }),
            greedy: true,
            min: 2,
            max: Some(3),
        };
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_repeat_forms() {
        let actual = parse("a{3}").unwrap();
        let expect = Ast::Repeat {
            expr: Box::new(single_char_class('a')),
            greedy: true,
            min: 3,
            max: Some(3),
        };
        assert_eq!(actual, expect);

        let actual = parse("a{2,}").unwrap();
        let expect = Ast::Repeat {
            expr: Box::new(single_char_class('a')),
            greedy: true,
            min: 2,
            max: None,
        };
        assert_eq!(actual, expect);

        let actual = parse("a{2,5}").unwrap();
        let expect = Ast::Repeat {
            expr: Box::new(single_char_class('a')),
            greedy: true,
            min: 2,
            max: Some(5),
        };
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_char_class_range() {
        let actual = parse("[a-z]").unwrap();
        let expect = Ast::CharClass(CharClass::new(
            vec![CharRange {
                start: 'a',
                end: 'z',
            }],
            false,
        ));
        assert_eq!(actual, expect);

        let actual = parse("[A-Z]").unwrap();
        let expect = Ast::CharClass(CharClass::new(
            vec![CharRange {
                start: 'A',
                end: 'Z',
            }],
            false,
        ));
        assert_eq!(actual, expect);

        let actual = parse("[0-9]").unwrap();
        let expect = Ast::CharClass(CharClass::new(
            vec![CharRange {
                start: '0',
                end: '9',
            }],
            false,
        ));
        assert_eq!(actual, expect);

        let actual = parse("[a-zA-Z0-9]").unwrap();
        let expect = Ast::CharClass(CharClass::new(
            vec![
                CharRange {
                    start: 'a',
                    end: 'z',
                },
                CharRange {
                    start: 'A',
                    end: 'Z',
                },
                CharRange {
                    start: '0',
                    end: '9',
                },
            ],
            false,
        ));
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_char_class_literals() {
        let actual = parse("[-a]").unwrap();
        let expect = Ast::CharClass(CharClass::new(
            vec![
                CharRange {
                    start: '-',
                    end: '-',
                },
                CharRange {
                    start: 'a',
                    end: 'a',
                },
            ],
            false,
        ));
        assert_eq!(actual, expect);

        let actual = parse("[a-]").unwrap();
        let expect = Ast::CharClass(CharClass::new(
            vec![
                CharRange {
                    start: 'a',
                    end: 'a',
                },
                CharRange {
                    start: '-',
                    end: '-',
                },
            ],
            false,
        ));
        assert_eq!(actual, expect);

        let actual = parse("[]a]").unwrap();
        let expect = Ast::CharClass(CharClass::new(
            vec![
                CharRange {
                    start: ']',
                    end: ']',
                },
                CharRange {
                    start: 'a',
                    end: 'a',
                },
            ],
            false,
        ));
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_char_class_concat() {
        let actual = parse("a[bc]d").unwrap();
        let expect = Ast::Concat(vec![
            single_char_class('a'),
            Ast::CharClass(CharClass::new(
                vec![
                    CharRange {
                        start: 'b',
                        end: 'b',
                    },
                    CharRange {
                        start: 'c',
                        end: 'c',
                    },
                ],
                false,
            )),
            single_char_class('d'),
        ]);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_char_class_negated() {
        let actual = parse("[^abc]").unwrap();
        let expect = Ast::CharClass(CharClass::new(
            vec![
                CharRange {
                    start: 'a',
                    end: 'a',
                },
                CharRange {
                    start: 'b',
                    end: 'b',
                },
                CharRange {
                    start: 'c',
                    end: 'c',
                },
            ],
            true,
        ));
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_capture_sequence() {
        let actual = parse("(abc)(def)").unwrap();
        let expect = Ast::Concat(vec![
            Ast::Capture {
                expr: Box::new(Ast::Concat(vec![
                    single_char_class('a'),
                    single_char_class('b'),
                    single_char_class('c'),
                ])),
                index: 1,
            },
            Ast::Capture {
                expr: Box::new(Ast::Concat(vec![
                    single_char_class('d'),
                    single_char_class('e'),
                    single_char_class('f'),
                ])),
                index: 2,
            },
        ]);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_backreference() {
        let actual = parse("(abc)\\1").unwrap();
        let expect = Ast::Concat(vec![
            Ast::Capture {
                expr: Box::new(Ast::Concat(vec![
                    single_char_class('a'),
                    single_char_class('b'),
                    single_char_class('c'),
                ])),
                index: 1,
            },
            Ast::Backreference(1),
        ]);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_anchors() {
        let actual = parse("^abc$").unwrap();
        let expect = Ast::Concat(vec![
            Ast::Assertion(Predicate::StartOfLine),
            single_char_class('a'),
            single_char_class('b'),
            single_char_class('c'),
            Ast::Assertion(Predicate::EndOfLine),
        ]);
        assert_eq!(actual, expect);

        let actual = parse("^$").unwrap();
        let expect = Ast::Concat(vec![
            Ast::Assertion(Predicate::StartOfLine),
            Ast::Assertion(Predicate::EndOfLine),
        ]);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_dot() {
        let actual = parse("a.c").unwrap();
        let expect = Ast::Concat(vec![
            single_char_class('a'),
            Ast::CharClass(CharClass::new(
                vec![CharRange {
                    start: '\u{0000}',
                    end: '\u{10FFFF}',
                }],
                false,
            )),
            single_char_class('c'),
        ]);
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_empty() {
        let actual = parse("").unwrap();
        let expect = Ast::Empty;
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_parse_escaped_literals() {
        let actual = parse("\\*").unwrap();
        let expect = single_char_class('*');
        assert_eq!(actual, expect);

        let actual = parse("\\\\").unwrap();
        let expect = single_char_class('\\');
        assert_eq!(actual, expect);

        let actual = parse("\\+").unwrap();
        let expect = single_char_class('+');
        assert_eq!(actual, expect);

        let actual = parse("\\?").unwrap();
        let expect = single_char_class('?');
        assert_eq!(actual, expect);

        let actual = parse("\\a").unwrap();
        let expect = single_char_class('a');
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_error_unexpected_end() {
        let mut parser = Parser::new("");
        let actual = parser.parse_factor();
        assert_eq!(actual, Err(ParseError::UnexpectedEnd));
    }

    #[test]
    fn test_error_unexpected_char() {
        let actual = parse("*");
        assert_eq!(actual, Err(ParseError::UnexpectedChar('*')));

        let actual = parse(")");
        assert_eq!(actual, Err(ParseError::UnexpectedChar(')')));

        let actual = parse("}");
        assert_eq!(actual, Err(ParseError::UnexpectedChar('}')));
    }

    #[test]
    fn test_error_invalid_repeat_op() {
        let actual = parse("a*?");
        assert_eq!(actual, Err(ParseError::InvalidRepeatOp));
    }

    #[test]
    fn test_error_invalid_repeat_size() {
        let actual = parse("a{2,1}");
        assert_eq!(actual, Err(ParseError::InvalidRepeatSize));
    }

    #[test]
    fn test_error_missing_bracket() {
        let actual = parse("[abc");
        assert_eq!(actual, Err(ParseError::MissingBracket));
    }

    #[test]
    fn test_error_missing_parenthesis() {
        let actual = parse("(abc");
        assert_eq!(actual, Err(ParseError::MissingParenthesis));
    }

    #[test]
    fn test_error_trailing_backslash() {
        let actual = parse("\\");
        assert_eq!(actual, Err(ParseError::TrailingBackslash));
    }

    #[test]
    fn test_error_invalid_char_class() {
        let actual = parse("[z-a]");
        assert_eq!(actual, Err(ParseError::InvalidCharClass));
    }

    #[test]
    fn test_error_missing_repeat_argument() {
        let actual = parse("a{}");
        assert_eq!(actual, Err(ParseError::MissingRepeatArgument));

        let actual = parse("a{,}");
        assert_eq!(actual, Err(ParseError::MissingRepeatArgument));

        let actual = parse("a{2,");
        assert_eq!(actual, Err(ParseError::MissingRepeatArgument));
    }
}
