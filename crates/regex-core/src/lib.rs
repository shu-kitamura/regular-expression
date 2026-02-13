use std::collections::BTreeSet;

use engine::Instruction;

mod engine;
pub mod error;

/// Public API for pattern matching.
pub struct Regex {
    /// Compiled instruction sequence.
    code: Vec<Instruction>,
    /// Must-have literal substrings used for a fast pre-filter.
    must_literals: Vec<String>,
    /// Candidate literal substrings used to find likely start positions.
    needles: Vec<String>,
    /// Whether this pattern can match the empty string.
    nullable: bool,
    /// Whether the instruction stream contains zero-width assertions.
    has_assertion: bool,
    /// Enables case-insensitive matching by lowercasing pattern/input.
    is_ignore_case: bool,
    /// Inverts the final match result.
    is_invert_match: bool,
}

impl Regex {
    /// Create a new `Regex`.
    pub fn new(
        pattern: &str,
        is_ignore_case: bool,
        is_invert_match: bool,
    ) -> Result<Self, error::RegexError> {
        let (code, analysis) = if is_ignore_case {
            engine::compile_pattern_with_analysis(&pattern.to_lowercase())?
        } else {
            engine::compile_pattern_with_analysis(pattern)?
        };
        let has_assertion = code
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Assert(_)));

        Ok(Self {
            code,
            must_literals: analysis.must_literals,
            needles: analysis.needles,
            nullable: analysis.nullable,
            has_assertion,
            is_ignore_case,
            is_invert_match,
        })
    }

    /// Match a line against the compiled pattern.
    pub fn is_match(&self, line: &str) -> Result<bool, error::RegexError> {
        let is_match = if self.is_ignore_case {
            self.is_match_line(&line.to_lowercase())?
        } else {
            self.is_match_line(line)?
        };

        Ok(is_match ^ self.is_invert_match)
    }

    /// Matches a line with nullable/must/needle prefilters and a full-eval fallback.
    fn is_match_line(&self, line: &str) -> Result<bool, error::RegexError> {
        if self.nullable && !self.has_assertion {
            return Ok(true);
        }

        if !self
            .must_literals
            .iter()
            .all(|literal| line.contains(literal))
        {
            return Ok(false);
        }

        if self.must_literals.is_empty() && !self.needles.is_empty() {
            let starts = Self::collect_start_positions_from_needles(line, &self.needles);
            if !starts.is_empty() && engine::match_line_from_starts(&self.code, line, &starts)? {
                return Ok(true);
            }
        }

        engine::match_line(&self.code, line)
    }

    fn collect_start_positions_from_needles(line: &str, needles: &[String]) -> Vec<usize> {
        let mut byte_starts = BTreeSet::new();
        for needle in needles {
            if needle.is_empty() {
                continue;
            }
            for (byte_start, _) in line.match_indices(needle) {
                byte_starts.insert(byte_start);
            }
        }

        if byte_starts.is_empty() {
            return Vec::new();
        }

        let mut starts = Vec::with_capacity(byte_starts.len());
        let mut targets = byte_starts.into_iter().peekable();
        let mut char_index = 0;
        for (byte_index, _) in line.char_indices() {
            while let Some(target) = targets.peek() {
                if *target == byte_index {
                    starts.push(char_index);
                    targets.next();
                } else {
                    break;
                }
            }
            if targets.peek().is_none() {
                break;
            }
            char_index += 1;
        }

        starts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_match() {
        let regex = Regex::new("ab(c|d)", false, false).unwrap();
        assert!(regex.is_match("abc").unwrap());
        assert!(!regex.is_match("abe").unwrap());
    }

    #[test]
    fn test_is_match_ignore_case() {
        let regex = Regex::new("ab(c|d)", true, false).unwrap();
        assert!(regex.is_match("ABC").unwrap());

        let regex = Regex::new("ab(c|d)", false, false).unwrap();
        assert!(!regex.is_match("ABC").unwrap());
    }

    #[test]
    fn test_is_match_invert() {
        let regex = Regex::new("ab(c|d)", false, true).unwrap();
        assert!(!regex.is_match("abc").unwrap());
        assert!(regex.is_match("abe").unwrap());
    }

    #[test]
    fn test_backreference() {
        let regex = Regex::new("(abc)\\1", false, false).unwrap();
        assert!(regex.is_match("abcabc").unwrap());
        assert!(!regex.is_match("abcabd").unwrap());
    }

    #[test]
    fn test_anchor_patterns() {
        let regex_start = Regex::new("^hello", false, false).unwrap();
        assert!(regex_start.is_match("hello world").unwrap());
        assert!(!regex_start.is_match("say hello").unwrap());

        let regex_end = Regex::new("world$", false, false).unwrap();
        assert!(regex_end.is_match("hello world").unwrap());
        assert!(!regex_end.is_match("world peace").unwrap());

        let regex_both = Regex::new("^hello$", false, false).unwrap();
        assert!(regex_both.is_match("hello").unwrap());
        assert!(!regex_both.is_match("hello world").unwrap());
        assert!(!regex_both.is_match("say hello").unwrap());
    }

    #[test]
    fn test_invalid_pattern() {
        assert!(Regex::new("(", false, false).is_err());
        assert!(Regex::new(")", false, false).is_err());
        assert!(Regex::new("*", false, false).is_err());
        assert!(Regex::new("+", false, false).is_err());
        assert!(Regex::new("?", false, false).is_err());
    }

    #[test]
    fn test_extracts_must_literals_for_filtering() {
        let regex = Regex::new(".*abc.*", false, false).unwrap();
        assert_eq!(regex.must_literals, vec!["abc".to_string()]);
        assert_eq!(regex.needles, vec!["abc".to_string()]);
        assert!(!regex.nullable);

        let regex = Regex::new("ab*c", false, false).unwrap();
        assert_eq!(regex.must_literals, vec!["a".to_string(), "c".to_string()]);
        assert_eq!(
            regex.needles,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_must_literal_filter_skips_non_matching_lines() {
        let regex = Regex::new(".*abc.*", false, false).unwrap();
        assert!(!regex.is_match("zzz").unwrap());
    }

    #[test]
    fn test_must_literal_filter_allows_matching_lines() {
        let regex = Regex::new("a.*c", false, false).unwrap();
        assert!(regex.is_match("a---c").unwrap());
        assert!(!regex.is_match("a---").unwrap());
    }

    #[test]
    fn test_must_literal_filter_respects_invert_match() {
        let regex = Regex::new(".*abc.*", false, true).unwrap();
        assert!(regex.is_match("zzz").unwrap());
    }

    #[test]
    fn test_empty_must_literals_still_runs_matcher() {
        let regex = Regex::new("(abc|def)", false, false).unwrap();
        assert!(regex.must_literals.is_empty());
        assert!(regex.is_match("def").unwrap());
        assert!(!regex.is_match("xyz").unwrap());
    }

    #[test]
    fn test_nullable_fast_path_without_assertion() {
        let regex = Regex::new("a*", false, false).unwrap();
        assert!(regex.nullable);
        assert!(!regex.has_assertion);
        assert!(regex.is_match("zzz").unwrap());
    }

    #[test]
    fn test_nullable_fast_path_is_guarded_by_assertion() {
        let regex = Regex::new("^$", false, false).unwrap();
        assert!(regex.nullable);
        assert!(regex.has_assertion);
        assert!(regex.is_match("").unwrap());
        assert!(!regex.is_match("x").unwrap());
    }

    #[test]
    fn test_needles_preferred_search_still_matches() {
        let regex = Regex::new("(abc|def)", false, false).unwrap();
        assert!(regex.must_literals.is_empty());
        assert_eq!(regex.needles, vec!["abc".to_string(), "def".to_string()]);
        assert!(regex.is_match("xyzdef").unwrap());
        assert!(!regex.is_match("xyz").unwrap());
    }

    #[test]
    fn test_needles_fallback_to_full_scan_preserves_correctness() {
        let regex = Regex::new("(a|[0-9])", false, false).unwrap();
        assert!(regex.must_literals.is_empty());
        assert_eq!(regex.needles, vec!["a".to_string()]);
        assert!(regex.is_match("5").unwrap());
    }
}
