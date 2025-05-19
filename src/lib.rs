use std::collections::BTreeSet;

use engine::instruction::{Char, Instruction};

mod engine;
mod error;

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
    first_chars: BTreeSet<char>,
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

        let first_chars = Self::get_first_chars(&code);

        Ok(Regex {
            code,
            first_chars,
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
                &self.first_chars,
                &line.to_lowercase(),
                self.is_caret,
                self.is_dollar,
                self.is_invert_match,
            )?
        } else {
            engine::match_line(
                &self.code,
                &self.first_chars,
                line,
                self.is_caret,
                self.is_dollar,
                self.is_invert_match,
            )?
        };
        Ok(is_match)
    }

    /// 命令列の先頭の命令に対応した文字を取得する
    /// * Char -> 最初の文字を取得する
    /// * Split -> 分岐先の文字を取得する
    /// * Jump, Match -> 何も取得しない
    fn get_first_chars(insts: &[Instruction]) -> BTreeSet<char> {
        let mut first_chars: BTreeSet<char> = BTreeSet::new();
        match insts.first() {
            Some(Instruction::Char(Char::Literal(ch))) => {
                first_chars.insert(*ch);
            }
            Some(Instruction::Split(left, right)) => {
                if let Some(Instruction::Char(Char::Literal(ch))) = insts.get(*left) {
                    first_chars.insert(*ch);
                };
                if let Some(Instruction::Char(Char::Literal(ch))) = insts.get(*right) {
                    first_chars.insert(*ch);
                };
            }
            _ => {} // Jump や Match になることはないため、何もしない
        };
        first_chars
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
        assert_eq!(result, true);

        // "abe" という文字列に対して、マッチングを実行
        let line = "abe";
        let result = regex.is_match(line).unwrap();
        assert_eq!(result, false);
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
        assert_eq!(result, true);

        // パターン "ab(c|d)" から Regex 構造体を生成
        // is_ignore_case を false に設定
        let pattern = "ab(c|d)";
        let regex2 = Regex::new(pattern, false, false).unwrap();

        // "ABC" という文字列に対して、マッチングを実行
        let line = "ABC";
        let result = regex2.is_match(line).unwrap();
        assert_eq!(result, false);
    }

    #[test]
    fn test_is_match_invert() {
        // パターン "ab(c|d)" から Regex 構造体を生成
        let pattern = "ab(c|d)";
        let regex = Regex::new(pattern, false, true).unwrap();

        // "abc" という文字列に対して、マッチングを実行
        let line = "abc";
        let result = regex.is_match(line).unwrap();
        assert_eq!(result, false);

        // "abe" という文字列に対して、マッチングを実行
        let line = "abe";
        let result = regex.is_match(line).unwrap();
        assert_eq!(result, true);
    }

    #[test]
    fn test_get_first_chars() {
        // "abc" のテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Char(Char::Literal('c')),
            Instruction::Match,
        ];
        let first_chars = Regex::get_first_chars(&insts);
        assert_eq!(first_chars.len(), 1);
        assert!(first_chars.contains(&'a'));

        // "a*bc" のテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Split(1, 3),
            Instruction::Char(Char::Literal('a')),
            Instruction::Jump(0),
            Instruction::Char(Char::Literal('b')),
            Instruction::Char(Char::Literal('c')),
            Instruction::Match,
        ];
        let first_chars = Regex::get_first_chars(&insts);
        assert_eq!(first_chars.len(), 2);
        assert!(first_chars.contains(&'a'));
        assert!(first_chars.contains(&'b'));

        // 以下のテストは実際にはありえないが、テストのために用意

        // 命令列の先頭が Jump のテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Jump(1),
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];
        let first_chars = Regex::get_first_chars(&insts);
        assert_eq!(first_chars.len(), 0);

        // 命令列の先頭が Match のテスト
        let insts: Vec<Instruction> = vec![
            Instruction::Match,
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
        ];
        let first_chars = Regex::get_first_chars(&insts);
        assert_eq!(first_chars.len(), 0);
    }
}
