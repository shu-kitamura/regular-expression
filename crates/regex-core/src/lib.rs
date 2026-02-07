use std::collections::BTreeSet;

use engine::{
    InstructionV2,
    instruction::{Char, Instruction},
};

mod engine;
pub mod error;
pub use engine::RegexV2Error;

/// パターンと文字列のマッチングを実行するAPI
///
/// # 引数
///
/// * code -> コンパイル済みのコード
/// * is_ignore_case -> 大小文字の区別をするかどうか
/// * is_invert_match -> マッチングの結果を逆にする
/// * is_caret -> 行頭からのマッチングをするかどうか
/// * is_dollar -> 行末からのマッチングをするかどうか
pub struct Regex {
    code: Vec<Instruction>,
    first_strings: BTreeSet<String>,
    is_ignore_case: bool,
    is_invert_match: bool,
    is_caret: bool,
    is_dollar: bool,
}

/// parser_v2 / compiler_v2 / evaluator_v2 を利用した API
pub struct RegexV2 {
    code: Vec<InstructionV2>,
    first_strings: BTreeSet<String>,
    is_ignore_case: bool,
    is_invert_match: bool,
}

impl Regex {
    /// 新しい Regex 構造体を生成する
    ///
    /// # 引数
    ///
    /// * pattern -> 正規表現のパターン
    /// * is_ignore_case -> 大小文字の区別をするかどうか
    /// * is_invert_match -> マッチングの結果を逆にするかどうか
    ///
    /// # 返り値
    ///
    /// * 正規表現のコンパイルに成功した場合は Regex 構造体を返す。
    /// * 正規表現のコンパイルに失敗した場合は RegexError を返す。
    pub fn new(
        pattern: &str,
        is_ignore_case: bool,
        is_invert_match: bool,
    ) -> Result<Self, error::RegexError> {
        let (code, is_caret, is_dollar) = if is_ignore_case {
            // 大小文字を区別しない場合、パターンを小文字でコンパイルする
            engine::compile_pattern(&pattern.to_lowercase())?
        } else {
            engine::compile_pattern(pattern)?
        };

        let first_strings = Self::get_first_strings(&code);

        Ok(Regex {
            code,
            first_strings,
            is_ignore_case,
            is_invert_match,
            is_caret,
            is_dollar,
        })
    }

    /// 行とパターンのマッチングを実行する
    ///
    /// # 引数
    ///
    /// * line -> マッチング対象の行
    ///
    /// # 返り値
    ///
    /// * エラーが発生した場合は RegexError を返す。
    /// * エラーが発生しなかった場合は、マッチング結果を返す。
    ///   ※ is_invert_match に true が指定されている場合は マッチング結果が反対になる。  
    pub fn is_match(&self, line: &str) -> Result<bool, error::RegexError> {
        let is_match = if self.is_ignore_case {
            // 大小文字を区別しない場合、行を小文字にしてマッチングする
            engine::match_line(
                &self.code,
                &self.first_strings,
                &line.to_lowercase(),
                self.is_caret,
                self.is_dollar,
            )?
        } else {
            engine::match_line(
                &self.code,
                &self.first_strings,
                line,
                self.is_caret,
                self.is_dollar,
            )?
        };
        Ok(is_match ^ self.is_invert_match)
    }

    fn get_first_strings(insts: &[Instruction]) -> BTreeSet<String> {
        let mut first_strings: BTreeSet<String> = BTreeSet::new();
        match insts.first() {
            Some(Instruction::Char(Char::Literal(_))) => {
                if let Some(string) = Self::get_string(insts, 0) {
                    first_strings.insert(string);
                };
            }
            Some(Instruction::Split(left, right)) => {
                if let Some(string) = Self::get_string(insts, *left) {
                    first_strings.insert(string);
                };
                if let Some(string) = Self::get_string(insts, *right) {
                    first_strings.insert(string);
                };
            }
            _ => {} // Jump や Match になることはないため、何もしない
        };
        first_strings
    }

    fn get_string(insts: &[Instruction], mut start: usize) -> Option<String> {
        let mut pre: String = String::new();

        while start < insts.len() {
            match insts.get(start) {
                Some(Instruction::Char(Char::Literal(c))) => {
                    pre.push(*c);
                    start += 1;
                }
                _ => break,
            }
        }

        if pre.is_empty() { None } else { Some(pre) }
    }
}

impl RegexV2 {
    /// 新しい RegexV2 構造体を生成する
    pub fn new(
        pattern: &str,
        is_ignore_case: bool,
        is_invert_match: bool,
    ) -> Result<Self, RegexV2Error> {
        let code = if is_ignore_case {
            engine::compile_pattern_v2(&pattern.to_lowercase())?
        } else {
            engine::compile_pattern_v2(pattern)?
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
    pub fn is_match(&self, line: &str) -> Result<bool, RegexV2Error> {
        let is_match = if self.is_ignore_case {
            self.is_match_line(&line.to_lowercase())?
        } else {
            self.is_match_line(line)?
        };

        Ok(is_match ^ self.is_invert_match)
    }

    fn is_match_line(&self, line: &str) -> Result<bool, RegexV2Error> {
        if self.first_strings.is_empty() {
            return engine::match_line_v2(&self.code, line);
        }

        let mut pos = 0;
        while let Some(i) = find_index(&line[pos..], &self.first_strings) {
            let start = pos + i;
            if engine::match_line_v2_from_start(&self.code, &line[start..])? {
                return Ok(true);
            }
            pos = start + 1;
        }

        Ok(false)
    }

    fn get_first_strings(insts: &[InstructionV2]) -> BTreeSet<String> {
        let mut first_strings: BTreeSet<String> = BTreeSet::new();
        match insts.first() {
            Some(inst) if Self::literal_from_instruction(inst).is_some() => {
                if let Some(string) = Self::get_string(insts, 0) {
                    first_strings.insert(string);
                };
            }
            Some(InstructionV2::Split(left, right)) => {
                if let Some(string) = Self::get_string(insts, *left) {
                    first_strings.insert(string);
                };
                if let Some(string) = Self::get_string(insts, *right) {
                    first_strings.insert(string);
                };
            }
            _ => {}
        };
        first_strings
    }

    fn get_string(insts: &[InstructionV2], mut start: usize) -> Option<String> {
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

    fn literal_from_instruction(inst: &InstructionV2) -> Option<char> {
        let InstructionV2::CharClass(class) = inst else {
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

// ----- テストコード -----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_match() {
        // パターン "ab(c|d)" から Regex 構造体を生成
        let pattern = "ab(c|d)";
        let regex = Regex::new(pattern, false, false).unwrap();

        // "abc" という文字列に対して、マッチングを実行
        let line = "abc";
        let result = regex.is_match(line).unwrap();
        assert!(result);

        // "abe" という文字列に対して、マッチングを実行
        let line = "abe";
        let result = regex.is_match(line).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_is_match_ignore_case() {
        // パターン "ab(c|d)" から Regex 構造体を生成
        // is_ignore_case を true に設定
        let pattern = "ab(c|d)";
        let regex1 = Regex::new(pattern, true, false).unwrap();

        // "ABC" という文字列に対して、マッチングを実行
        let line = "ABC";
        let result = regex1.is_match(line).unwrap();
        assert!(result);

        // パターン "ab(c|d)" から Regex 構造体を生成
        // is_ignore_case を false に設定
        let pattern = "ab(c|d)";
        let regex2 = Regex::new(pattern, false, false).unwrap();

        // "ABC" という文字列に対して、マッチングを実行
        let line = "ABC";
        let result = regex2.is_match(line).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_is_match_invert() {
        // パターン "ab(c|d)" から Regex 構造体を生成
        let pattern = "ab(c|d)";
        let regex = Regex::new(pattern, false, true).unwrap();

        // "abc" という文字列に対して、マッチングを実行
        let line = "abc";
        let result = regex.is_match(line).unwrap();
        assert!(!result);

        // "abe" という文字列に対して、マッチングを実行
        let line = "abe";
        let result = regex.is_match(line).unwrap();
        assert!(result);
    }

    #[test]
    fn test_get_first_strings() {
        // "abc" のテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Char(Char::Literal('c')),
            Instruction::Match,
        ];
        let first_strings = Regex::get_first_strings(&insts);
        assert_eq!(first_strings.len(), 1);
        assert!(first_strings.contains("abc"));

        // "a*bc" のテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Split(1, 3),
            Instruction::Char(Char::Literal('a')),
            Instruction::Jump(0),
            Instruction::Char(Char::Literal('b')),
            Instruction::Char(Char::Literal('c')),
            Instruction::Match,
        ];
        let first_strings = Regex::get_first_strings(&insts);
        assert_eq!(first_strings.len(), 2);
        assert!(first_strings.contains("a"));
        assert!(first_strings.contains("bc"));

        // 以下のテストは実際にはありえないが、テストのために用意

        // 命令列の先頭が Jump のテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Jump(1),
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];
        let first_strings = Regex::get_first_strings(&insts);
        assert_eq!(first_strings.len(), 0);

        // 命令列の先頭が Match のテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Match,
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
        ];
        let first_strings = Regex::get_first_strings(&insts);
        assert_eq!(first_strings.len(), 0);
    }

    #[test]
    fn test_get_string() {
        // "ED*vQYpl" のテスト
        let regex = Regex::new("ED*vQYpl", false, false).unwrap();
        let insts = regex.code;
        let first_strings = Regex::get_first_strings(&insts);
        assert_eq!(first_strings.len(), 1);
        assert!(first_strings.contains("E"))
    }

    #[test]
    fn test_regex_new_error_cases() {
        // 不正なパターンのテスト
        let result = Regex::new("(", false, false);
        assert!(result.is_err());

        let result = Regex::new(")", false, false);
        assert!(result.is_err());

        let result = Regex::new("*", false, false);
        assert!(result.is_err());

        let result = Regex::new("+", false, false);
        assert!(result.is_err());

        let result = Regex::new("?", false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_patterns() {
        // より複雑なパターンのテスト
        let regex = Regex::new("a(b|c)*d", false, false).unwrap();
        assert!(regex.is_match("ad").unwrap());
        assert!(regex.is_match("abd").unwrap());
        assert!(regex.is_match("acd").unwrap());
        assert!(regex.is_match("abcd").unwrap());
        assert!(regex.is_match("abcbcbd").unwrap());
        assert!(!regex.is_match("ae").unwrap());
    }

    #[test]
    fn test_anchor_patterns() {
        // アンカーパターンのテスト
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

        // 空行にマッチする ^$ パターンのテスト
        // この機能は以前 ParseError::Empty を返していた問題を修正したもの
        let regex_empty_line = Regex::new("^$", false, false).unwrap();
        assert!(regex_empty_line.is_match("").unwrap());
        assert!(!regex_empty_line.is_match("test").unwrap());
        assert!(!regex_empty_line.is_match(" ").unwrap()); // スペースを含む行はマッチしない
    }

    #[test]
    fn test_empty_and_special_strings() {
        // 実際の動作に基づいたテスト

        // a+パターンのテスト（1個以上のa）
        let regex_plus = Regex::new("a+", false, false).unwrap();
        assert!(!regex_plus.is_match("").unwrap());
        assert!(regex_plus.is_match("a").unwrap());
        assert!(regex_plus.is_match("aaa").unwrap());
        assert!(regex_plus.is_match("baaac").unwrap()); // 文字列内にaが含まれている

        // 空文字列パターンのテスト - エラーになることを確認
        let result_empty = Regex::new("", false, false);
        assert!(result_empty.is_err()); // 空パターンはエラーになる

        // より具体的なパターンのテスト
        let regex_literal = Regex::new("abc", false, false).unwrap();
        assert!(regex_literal.is_match("abc").unwrap());
        assert!(regex_literal.is_match("xabcy").unwrap()); // 部分マッチ
        assert!(!regex_literal.is_match("ab").unwrap());
        assert!(!regex_literal.is_match("def").unwrap());

        // ドット（任意文字）のテスト
        let regex_dot = Regex::new("a.c", false, false).unwrap();
        assert!(regex_dot.is_match("abc").unwrap());
        assert!(regex_dot.is_match("axc").unwrap());
        assert!(regex_dot.is_match("a1c").unwrap());
        assert!(!regex_dot.is_match("ac").unwrap());
    }

    #[test]
    fn test_case_sensitivity_edge_cases() {
        // 大文字小文字の境界ケース
        let regex_sensitive = Regex::new("Hello", false, false).unwrap();
        assert!(regex_sensitive.is_match("Hello").unwrap());
        assert!(!regex_sensitive.is_match("hello").unwrap());
        assert!(!regex_sensitive.is_match("HELLO").unwrap());

        let regex_insensitive = Regex::new("Hello", true, false).unwrap();
        assert!(regex_insensitive.is_match("Hello").unwrap());
        assert!(regex_insensitive.is_match("hello").unwrap());
        assert!(regex_insensitive.is_match("HELLO").unwrap());
        assert!(regex_insensitive.is_match("hELLo").unwrap());
    }

    #[test]
    fn test_invert_match_combinations() {
        // 反転マッチの組み合わせテスト
        let regex_normal = Regex::new("test", false, false).unwrap();
        let regex_invert = Regex::new("test", false, true).unwrap();

        assert!(regex_normal.is_match("test").unwrap());
        assert!(!regex_invert.is_match("test").unwrap());

        assert!(!regex_normal.is_match("other").unwrap());
        assert!(regex_invert.is_match("other").unwrap());

        // 大文字小文字無視 + 反転
        let regex_ignore_invert = Regex::new("TEST", true, true).unwrap();
        assert!(!regex_ignore_invert.is_match("test").unwrap());
        assert!(!regex_ignore_invert.is_match("TEST").unwrap());
        assert!(regex_ignore_invert.is_match("other").unwrap());
    }

    #[test]
    fn test_get_first_strings_edge_cases() {
        // get_first_strings の境界ケース

        // AnyChar で始まるパターン
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Any),
            Instruction::Char(Char::Literal('a')),
            Instruction::Match,
        ];
        let first_strings = Regex::get_first_strings(&insts);
        assert_eq!(first_strings.len(), 0);

        // 空の命令列
        let insts: Vec<Instruction> = vec![];
        let first_strings = Regex::get_first_strings(&insts);
        assert_eq!(first_strings.len(), 0);

        // Split で始まり、両方の分岐が Literal
        let insts: Vec<Instruction> = vec![
            Instruction::Split(1, 3),
            Instruction::Char(Char::Literal('a')),
            Instruction::Jump(5),
            Instruction::Char(Char::Literal('b')),
            Instruction::Char(Char::Literal('c')),
            Instruction::Match,
        ];
        let first_strings = Regex::get_first_strings(&insts);
        assert_eq!(first_strings.len(), 2);
        assert!(first_strings.contains("a"));
        assert!(first_strings.contains("bc"));
    }

    #[test]
    fn test_get_string_edge_cases() {
        // get_string の境界ケース

        // 範囲外のインデックス
        let insts: Vec<Instruction> =
            vec![Instruction::Char(Char::Literal('a')), Instruction::Match];
        let result = Regex::get_string(&insts, 10);
        assert_eq!(result, None);

        // Literal以外の命令で始まる
        let insts: Vec<Instruction> =
            vec![Instruction::Match, Instruction::Char(Char::Literal('a'))];
        let result = Regex::get_string(&insts, 0);
        assert_eq!(result, None);

        // 単一のLiteral文字
        let insts: Vec<Instruction> =
            vec![Instruction::Char(Char::Literal('x')), Instruction::Match];
        let result = Regex::get_string(&insts, 0);
        assert_eq!(result, Some("x".to_string()));
    }

    #[test]
    fn test_regex_v2_is_match() {
        let regex = RegexV2::new("ab(c|d)", false, false).unwrap();
        assert!(regex.is_match("abc").unwrap());
        assert!(!regex.is_match("abe").unwrap());
    }

    #[test]
    fn test_regex_v2_backreference() {
        let regex = RegexV2::new("(abc)\\1", false, false).unwrap();
        assert!(regex.is_match("abcabc").unwrap());
        assert!(!regex.is_match("abcabd").unwrap());
    }

    #[test]
    fn test_regex_v2_invalid_backreference() {
        let result = RegexV2::new("(a)\\2", false, false);
        assert!(matches!(result, Err(RegexV2Error::Compile(_))));
    }

    #[test]
    fn test_regex_v2_get_first_strings() {
        let regex = RegexV2::new("abc", false, false).unwrap();
        assert_eq!(regex.first_strings.len(), 1);
        assert!(regex.first_strings.contains("abc"));

        let regex = RegexV2::new("a*bc", false, false).unwrap();
        assert_eq!(regex.first_strings.len(), 2);
        assert!(regex.first_strings.contains("a"));
        assert!(regex.first_strings.contains("bc"));
    }
}
