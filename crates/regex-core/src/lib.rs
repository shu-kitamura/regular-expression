use std::collections::BTreeSet;

use engine::Instruction;

mod engine;
pub mod error;

/// パターンと文字列のマッチングを実行する API
pub struct Regex {
    code: Vec<Instruction>,
    first_strings: BTreeSet<String>,
    is_ignore_case: bool,
    is_invert_match: bool,
}

impl Regex {
    /// 新しい Regex 構造体を生成する
    pub fn new(
        pattern: &str,
        is_ignore_case: bool,
        is_invert_match: bool,
    ) -> Result<Self, error::RegexError> {
        let code = if is_ignore_case {
            engine::compile_pattern(&pattern.to_lowercase())?
        } else {
            engine::compile_pattern(pattern)?
        };

        let first_strings = Self::get_first_strings(&code);

        Ok(Self {
            code,
            first_strings,
            is_ignore_case,
            is_invert_match,
        })
    }

    /// 行とパターンのマッチングを実行する
    pub fn is_match(&self, line: &str) -> Result<bool, error::RegexError> {
        let is_match = if self.is_ignore_case {
            self.is_match_line(&line.to_lowercase())?
        } else {
            self.is_match_line(line)?
        };

        Ok(is_match ^ self.is_invert_match)
    }

    fn is_match_line(&self, line: &str) -> Result<bool, error::RegexError> {
        if self.first_strings.is_empty() {
            return engine::match_line(&self.code, line);
        }

        let mut pos = 0;
        while let Some(i) = find_index(&line[pos..], &self.first_strings) {
            let start = pos + i;
            if engine::match_line_from_start(&self.code, &line[start..])? {
                return Ok(true);
            }
            pos = start + 1;
        }

        Ok(false)
    }

    fn get_first_strings(insts: &[Instruction]) -> BTreeSet<String> {
        let mut first_strings: BTreeSet<String> = BTreeSet::new();
        match insts.first() {
            Some(inst) if Self::literal_from_instruction(inst).is_some() => {
                if let Some(string) = Self::get_string(insts, 0) {
                    first_strings.insert(string);
                }
            }
            Some(Instruction::Split(left, right)) => {
                if let Some(string) = Self::get_string(insts, *left) {
                    first_strings.insert(string);
                }
                if let Some(string) = Self::get_string(insts, *right) {
                    first_strings.insert(string);
                }
            }
            _ => {}
        }
        first_strings
    }

    fn get_string(insts: &[Instruction], mut start: usize) -> Option<String> {
        let mut pre: String = String::new();

        while start < insts.len() {
            let Some(inst) = insts.get(start) else {
                break;
            };

            match Self::literal_from_instruction(inst) {
                Some(c) => {
                    pre.push(c);
                    start += 1;
                }
                None => break,
            }
        }

        if pre.is_empty() { None } else { Some(pre) }
    }

    fn literal_from_instruction(inst: &Instruction) -> Option<char> {
        let Instruction::CharClass(class) = inst else {
            return None;
        };

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
}

fn find_index(string: &str, string_set: &BTreeSet<String>) -> Option<usize> {
    string_set
        .iter()
        .map(|s| string.find(s))
        .filter(|opt| opt.is_some())
        .min()?
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
    fn test_get_first_strings() {
        let regex = Regex::new("abc", false, false).unwrap();
        assert_eq!(regex.first_strings.len(), 1);
        assert!(regex.first_strings.contains("abc"));

        let regex = Regex::new("a*bc", false, false).unwrap();
        assert_eq!(regex.first_strings.len(), 2);
        assert!(regex.first_strings.contains("a"));
        assert!(regex.first_strings.contains("bc"));
    }
}
