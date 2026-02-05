# Parser 実装計画: byte 指向への移行（ParseError は文字位置維持）

## 概要
`parser` を byte 指向で解析するように変更し、`Ast::Char` を `u8` 化します。  
ParseError の `position` は UTF-8 文字位置を維持するため、byte 走査中に文字位置を別途算出します。

## 重要な変更点（API / 型）
- `Ast::Char(char)` → `Ast::Char(u8)`（内部型変更）
- `parse(pattern: &str)` のシグネチャは維持
- `ParseError` の `position` は「文字位置」を維持（byte 位置ではない）

## 実装方針（文字位置の算出）
- `pattern.as_bytes()` を走査しつつ、文字位置 `char_pos` を別途計算する
- UTF-8 連続バイトかどうかで `char_pos` を更新  
  `is_utf8_continuation(b) = (b & 0b1100_0000) == 0b1000_0000`  
  `current_pos` は次で決定する
  - 連続バイト: `current_pos = char_pos - 1`
  - 先頭バイト: `current_pos = char_pos` の後に `char_pos += 1`
- エラー位置は `current_pos` を使う（従来の文字位置を維持）

## 実装手順

1. **Ast と escape 定義の byte 化** (File: `crates/regex-core/src/engine/parser.rs`)  
Action: `Ast::Char(u8)` に変更、`ESCAPE_CHARS` を `[u8; 8]` に変更  
Why: byte 指向の AST に合わせる  
Dependencies: なし  
Risk: 低

2. **parse_escape / parse_qualifier の byte 化** (File: `crates/regex-core/src/engine/parser.rs`)  
Action: `parse_escape(pos, b: u8)` / `parse_qualifier(qualifier: u8, prev: Ast)` に変更  
Action: `ParseError::InvalidEscape` 用に `char::from(b)` で char を生成  
Why: byte 指向解析に統一しつつ、既存エラー型を維持  
Dependencies: 手順1  
Risk: 低（非 ASCII の無効エスケープは U+00xx 表示になる）

3. **parse ループの byte 走査 + 文字位置維持** (File: `crates/regex-core/src/engine/parser.rs`)  
Action: `for (i, &b) in pattern.as_bytes().iter().enumerate()` に変更  
Action: `current_pos` を上記方針で算出し、`ParseError` に使用  
Why: byte 指向解析と「文字位置」エラーの両立  
Dependencies: 手順1-2  
Risk: 中（`current_pos` 計算ミスでエラー位置がずれる）

4. **メタ文字判定を byte 比較へ** (File: `crates/regex-core/src/engine/parser.rs`)  
Action: `b'+'`, `b'*'`, `b'?'`, `b'('`, `b')'`, `b'|'`, `b'\\'`, `b'.'` を使用  
Why: byte 走査の一貫性  
Dependencies: 手順3  
Risk: 低

5. **パーサーテスト更新** (File: `crates/regex-core/src/engine/parser.rs`)  
Action: `Ast::Char('a')` → `Ast::Char(b'a')` に更新  
Action: `parse_escape` など byte 引数に合わせて修正  
Why: 型変更に追随  
Dependencies: 手順1-4  
Risk: 低

## テストケース
- `parse("abc")` が `Ast::Seq([Char(b'a'), Char(b'b'), Char(b'c')])`
- `parse("a\\*b")` が `Char(b'*')` を含む
- `parse("*abc")` が `ParseError::NoPrev(0)`（位置は文字位置）
- `parse("abc(def|ghi))")` が `ParseError::InvalidRightParen(12)`（既存と一致）
- `parse("a\\bc")` が `ParseError::InvalidEscape(2, 'b')`（既存と一致）

## リスクと対策
- **Risk**: 文字位置の算出が誤り、エラー位置がずれる  
  - Mitigation: 既存の parser テストを維持し、位置依存のテストを必ず通す

## 明示的な前提
- `ParseError` の `position` は「文字位置」のまま維持する  
- `ParseError::InvalidEscape` の `char` は `char::from(byte)` を使用する  
- 末尾 `\` の挙動は現状維持（新たなエラーは出さない）

## 成功基準
- [ ] `parser` のユニットテストがすべてパスする  
- [ ] `ParseError` の位置が従来の文字位置と一致する  
- [ ] `Ast::Char` が `u8` に統一され、byte 指向解析が成立している
