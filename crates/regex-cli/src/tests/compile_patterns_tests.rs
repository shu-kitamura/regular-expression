use crate::compile_patterns;
use regex_core::error::RegexError;

#[test]
fn test_compile_valid_patterns() {
    // 有効なパターンのリスト
    let patterns = vec!["abc".to_string(), "a(b|c)d".to_string(), "x.*y".to_string()];

    // デフォルトオプションでコンパイル
    let result = compile_patterns(&patterns, false, false);

    // 結果が成功であることを確認
    assert!(result.is_ok());

    // 結果のベクターの長さがパターンの数と一致することを確認
    let regexes = result.unwrap();
    assert_eq!(regexes.len(), patterns.len());
}

#[test]
fn test_compile_with_ignore_case() {
    // ignore_case オプションのテスト
    let patterns = vec!["abc".to_string()];

    // ignore_case = true でコンパイル
    let result = compile_patterns(&patterns, true, false);
    assert!(result.is_ok());
    let regexes = result.unwrap();

    // 大文字小文字を区別してマッチすることを確認
    assert!(regexes[0].is_match("ABC").unwrap());
    assert!(regexes[0].is_match("abc").unwrap());

    // ignore_case = false でコンパイル
    let result = compile_patterns(&patterns, false, false);
    assert!(result.is_ok());
    let regexes = result.unwrap();

    // 大文字小文字を区別してマッチすることを確認
    assert!(!regexes[0].is_match("ABC").unwrap());
    assert!(regexes[0].is_match("abc").unwrap());
}

#[test]
fn test_compile_with_invert_match() {
    // invert_match オプションのテスト
    let patterns = vec!["abc".to_string()];

    // invert_match = true でコンパイル
    let result = compile_patterns(&patterns, false, true);
    assert!(result.is_ok());
    let regexes = result.unwrap();

    // マッチ結果が反転することを確認
    assert!(!regexes[0].is_match("abc").unwrap());
    assert!(regexes[0].is_match("def").unwrap());
}

#[test]
fn test_compile_invalid_pattern() {
    // 無効なパターンを含むリスト
    let patterns = vec![
        "abc".to_string(),
        "(".to_string(), // 閉じ括弧がないので無効
        "xyz".to_string(),
    ];

    // コンパイル結果がエラーであることを確認
    let result = compile_patterns(&patterns, false, false);
    assert!(result.is_err());

    // エラーの種類を確認（ParseError::NoRightParen）
    if let Err(err) = result {
        match err {
            RegexError::Parse(e) => {
                // エラーメッセージに "no right parenthesis" が含まれていることを確認
                assert!(format!("{}", e).contains("no right parenthesis"));
            }
            _ => panic!("Expected ParseError"),
        }
    }
}

#[test]
fn test_compile_empty_patterns() {
    // 空のパターンリスト
    let patterns: Vec<String> = vec![];

    // 空のリストをコンパイル
    let result = compile_patterns(&patterns, false, false);

    // 結果が成功であることを確認
    assert!(result.is_ok());

    // 結果が空のベクターであることを確認
    let regexes = result.unwrap();
    assert!(regexes.is_empty());
}

#[test]
fn test_compile_multiple_invalid_patterns() {
    // 複数の無効なパターンを含むリスト
    let patterns = vec![
        "*".to_string(), // 先行する式がないので無効
        "+".to_string(), // 先行する式がないので無効
    ];

    // コンパイル結果がエラーであることを確認
    let result = compile_patterns(&patterns, false, false);
    assert!(result.is_err());

    // 最初のエラー（*に関するエラー）が返されることを確認
    if let Err(err) = result {
        match err {
            RegexError::Parse(e) => {
                // エラーメッセージに "no previous expression" が含まれていることを確認
                assert!(format!("{}", e).contains("no previous expression"));
            }
            _ => panic!("Expected ParseError"),
        }
    }
}
