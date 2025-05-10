mod engine;
mod error;

/// パターンと文字列のマッチングを実行するAPI
///
/// # 引数
///
/// * pattern -> 正規表現のパターン
/// * line -> マッチング対象の文字列
/// * is_ignore_case -> 大小文字の区別をするかどうか
/// * is_invert_match -> マッチングの結果を逆にする
///
/// # 返り値
///
/// エラーなく実行でき、マッチングに成功した場合 true を返す。  
/// エラーなく実行でき、マッチングに失敗した場合 false を返す。  
/// ※ is_invert_match に true が指定されている場合は マッチング結果が反対になる。  
pub fn pattern_match(
    pattern: &str,
    line: &str,
    is_ignore_case: bool,
    is_invert_match: bool,
) -> Result<bool, error::RegexError> {
    engine::match_line(
        pattern.to_string(),
        line.to_string(),
        is_ignore_case,
        is_invert_match,
    )
}

// ----- テストコード -----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_match_true() {
        // マッチが成功するケース
        let pattern = "abc";
        let line = "abcdef";
        let is_ignore_case = false;
        let is_invert_match = false;

        let result = pattern_match(pattern, line, is_ignore_case, is_invert_match);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_pattern_match_false() {
        // マッチが失敗するケース
        let pattern = "xyz";
        let line = "abcdef";
        let is_ignore_case = false;
        let is_invert_match = false;
        let result = pattern_match(pattern, line, is_ignore_case, is_invert_match);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_pattern_match_is_ignore_case() {
        // 大小文字を区別しないケース
        let pattern = "abc";
        let line = "ABCDEF";
        let is_ignore_case = true;
        let is_invert_match = false;
        let result = pattern_match(pattern, line, is_ignore_case, is_invert_match);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_pattern_match_is_invert_match() {
        // マッチング結果を逆にするケース
        let pattern = "abc";
        let line = "abcdef";
        let is_ignore_case = false;
        let is_invert_match = true;
        let result = pattern_match(pattern, line, is_ignore_case, is_invert_match);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }
}
