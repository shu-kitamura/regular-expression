use engine::Instruction;

mod engine;
pub mod error;

/// Public API for pattern matching.
pub struct Regex {
    /// Compiled instruction sequence.
    code: Vec<Instruction>,
    /// Must-have literal substrings used for a fast pre-filter.
    must_literals: Vec<String>,
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
        let (code, must_literals) = if is_ignore_case {
            engine::compile_pattern_with_must_literals(&pattern.to_lowercase())?
        } else {
            engine::compile_pattern_with_must_literals(pattern)?
        };

        Ok(Self {
            code,
            must_literals,
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

    /// Matches a line, optionally using a must-literal pre-filter first.
    fn is_match_line(&self, line: &str) -> Result<bool, error::RegexError> {
        if !self
            .must_literals
            .iter()
            .all(|literal| line.contains(literal))
        {
            return Ok(false);
        }

        engine::match_line(&self.code, line)
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

        let regex = Regex::new("ab*c", false, false).unwrap();
        assert_eq!(regex.must_literals, vec!["a".to_string(), "c".to_string()]);
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
}
