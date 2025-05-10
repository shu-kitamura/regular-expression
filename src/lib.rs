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
    string: &str,
    ignore_case: bool,
    invert_match: bool,
) -> Result<bool, error::RegexError> {
    engine::match_line(
        pattern.to_string(),
        string.to_string(),
        ignore_case,
        invert_match,
    )
}
