use engine::search_plan::SearchPlan;

mod engine;
pub mod error;

/// パターンとバイト列のマッチングを実行するAPI
///
/// # 引数
///
/// * code -> コンパイル済みのコード
/// * is_ignore_case -> 大小文字の区別をするかどうか（ASCII のみ）
/// * is_invert_match -> マッチングの結果を逆にする
/// * is_caret -> 行頭からのマッチングをするかどうか
/// * is_dollar -> 行末からのマッチングをするかどうか
pub struct Regex {
    code: Vec<engine::instruction::Instruction>,
    search_plan: SearchPlan,
    is_ignore_case: bool,
    is_invert_match: bool,
    is_caret: bool,
    is_dollar: bool,
}

impl Regex {
    /// 新しい Regex 構造体を生成する
    ///
    /// # 引数
    ///
    /// * pattern -> 正規表現のパターン
    /// * is_ignore_case -> 大小文字の区別をするかどうか（ASCII のみ対象）
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
            // 大小文字を区別しない場合、パターンを ASCII 小文字でコンパイルする
            engine::compile_pattern(&Self::to_ascii_lowercase(pattern))?
        } else {
            engine::compile_pattern(pattern)?
        };

        let search_plan = engine::build_search_plan(&code);

        Ok(Regex {
            code,
            search_plan,
            is_ignore_case,
            is_invert_match,
            is_caret,
            is_dollar,
        })
    }

    /// 行とパターンのマッチングを実行する（文字列版）
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
    ///   
    /// # 注意
    ///
    /// is_ignore_case が true の場合、ASCII のみを対象に大小文字を無視します。
    pub fn is_match(&self, line: &str) -> Result<bool, error::RegexError> {
        self.is_match_bytes(line.as_bytes())
    }

    /// 行とパターンのマッチングを実行する（バイト列版）
    ///
    /// # 引数
    ///
    /// * line -> マッチング対象のバイト列
    ///
    /// # 返り値
    ///
    /// * エラーが発生した場合は RegexError を返す。
    /// * エラーが発生しなかった場合は、マッチング結果を返す。
    ///   ※ is_invert_match に true が指定されている場合は マッチング結果が反対になる。  
    ///   
    /// # 注意
    ///
    /// is_ignore_case が true の場合、ASCII のみを対象に大小文字を無視します。
    pub fn is_match_bytes(&self, line: &[u8]) -> Result<bool, error::RegexError> {
        let is_match = engine::match_line(
            &self.code,
            &self.search_plan,
            line,
            self.is_ignore_case,
            self.is_caret,
            self.is_dollar,
        )?;
        Ok(is_match ^ self.is_invert_match)
    }

    /// ASCII 文字列を小文字に変換するヘルパー関数
    fn to_ascii_lowercase(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii() {
                    c.to_ascii_lowercase()
                } else {
                    c
                }
            })
            .collect()
    }
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
    fn test_is_match_bytes() {
        // パターン "ab(c|d)" から Regex 構造体を生成
        let pattern = "ab(c|d)";
        let regex = Regex::new(pattern, false, false).unwrap();

        // b"abc" というバイト列に対して、マッチングを実行
        let line = b"abc";
        let result = regex.is_match_bytes(line).unwrap();
        assert!(result);

        // b"abe" というバイト列に対して、マッチングを実行
        let line = b"abe";
        let result = regex.is_match_bytes(line).unwrap();
        assert!(!result);

        // b"zab" というバイト列に対して、マッチングを実行（部分マッチ）
        let line = b"zabc";
        let result = regex.is_match_bytes(line).unwrap();
        assert!(result);
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
    fn test_is_match_bytes_ignore_case() {
        // パターン "ab(c|d)" から Regex 構造体を生成
        // is_ignore_case を true に設定（ASCII のみ対応）
        let pattern = "ab(c|d)";
        let regex = Regex::new(pattern, true, false).unwrap();

        // b"ABC" というバイト列に対して、マッチングを実行
        let result = regex.is_match_bytes(b"ABC").unwrap();
        assert!(result);

        // 大小文字混在
        let result = regex.is_match_bytes(b"AbC").unwrap();
        assert!(result);

        // ASCII 大小文字無視が機能していることを確認
        let result = regex.is_match_bytes(b"ABD").unwrap();
        assert!(result);
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
    fn test_regression_or_branches() {
        let regex = Regex::new("a|b|c", false, false).unwrap();
        assert!(regex.is_match("a").unwrap());
        assert!(regex.is_match("b").unwrap());
        assert!(regex.is_match("c").unwrap());
    }

    #[test]
    fn test_regression_empty_match_non_anchored() {
        let star = Regex::new("a*", false, false).unwrap();
        assert!(star.is_match("").unwrap());
        assert!(star.is_match("bbb").unwrap());

        let question = Regex::new("a?", false, false).unwrap();
        assert!(question.is_match("").unwrap());
        assert!(question.is_match("bbb").unwrap());
    }

    #[test]
    fn test_regression_non_utf8_input() {
        let regex = Regex::new("ab", false, false).unwrap();
        let input = [0xFF, b'a', b'b'];
        assert!(regex.is_match_bytes(&input).unwrap());
    }
}
