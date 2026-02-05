# 実装計画: regex-core をバイト指向に移行し `is_match_bytes` を追加

## 概要
regex-core のマッチングをバイト列前提に変更し、入力を `&[u8]` で受け取れる API を追加します。  
既存の `Regex::is_match(&str)` は互換維持しつつ内部でバイトマッチングを呼び出し、ASCII だけの大小文字無視に統一します。

## 要件
- バイト指向のマッチングに統一する（`.` は 1 バイト）
- 既存 `Regex::is_match(&str)` は残しつつ `Regex::is_match_bytes(&[u8])` を追加
- `-i`（ignore-case）は ASCII のみを対象にする
- 既存テストが通るように必要な修正を行う
- 変更点は README と doc コメントに反映する

## 重要な公開APIの変更
- 追加: `Regex::is_match_bytes(&self, line: &[u8]) -> Result<bool, RegexError>`
- 既存: `Regex::is_match(&self, line: &str)` はバイト指向ロジックに委譲される  
  非 ASCII 文字に対する `-i` の挙動は Unicode ではなく ASCII のみになる点を明記

## アーキテクチャ変更
- `crates/regex-core/src/engine/instruction.rs`  
  `Char::Literal(char)` を `Char::Literal(u8)` に変更し、コメントを「バイト」に更新
- `crates/regex-core/src/engine/parser.rs`  
  文字列走査を `chars()` から `as_bytes()` に変更し、`Ast::Char` を `u8` に変更
- `crates/regex-core/src/engine/compiler.rs`  
  `Ast::Char(u8)` に対応する命令生成に変更
- `crates/regex-core/src/engine/evaluator.rs`  
  評価対象を `&[u8]` に変更し、`eval_char` を `slice.get(index)` ベースに変更
- `crates/regex-core/src/engine.rs`  
  `match_line`/`match_string`/`find_index` をバイト列版に更新  
  `first_strings` は `BTreeSet<Vec<u8>>`
- `crates/regex-core/src/lib.rs`  
  `Regex::is_match_bytes` を追加、`is_match(&str)` は `as_bytes()` で委譲  
  `ignore_case` 用の ASCII 小文字化ヘルパー追加  
  `get_first_strings`/`get_string` を `Vec<u8>` で実装

## 実装手順

### フェーズ1: バイト指向データモデル化
1. **Instruction/Char の更新** (File: crates/regex-core/src/engine/instruction.rs)  
Action: `Char::Literal(u8)` 化、コメントと Display を ASCII 前提に更新  
Why: バイトマッチングの前提を型で固定  
Dependencies: なし  
Risk: 低

2. **Parser のバイト化** (File: crates/regex-core/src/engine/parser.rs)  
Action: `pattern.as_bytes().iter().enumerate()` で走査、`Ast::Char(u8)` に変更、`ESCAPE_CHARS` を `u8` 配列に変更  
Why: パターンを byte-wise に解析するため  
Dependencies: ステップ1  
Risk: 中（ParseError の位置や表示の意味が byte index になる）

3. **Compiler のバイト対応** (File: crates/regex-core/src/engine/compiler.rs)  
Action: `Ast::Char(u8)` から `Instruction::Char(Char::Literal(u8))` を生成  
Why: 命令列をバイト前提に統一  
Dependencies: ステップ1-2  
Risk: 低

### フェーズ2: 評価・マッチングのバイト化
4. **Evaluator を &[u8] に変更** (File: crates/regex-core/src/engine/evaluator.rs)  
Action: `eval_char` を `string.get(index)` で比較、`eval`/`eval_depth` を `&[u8]` に変更  
Why: O(1) のバイト参照で高速化  
Dependencies: フェーズ1  
Risk: 低

5. **match_line 系のバイト化** (File: crates/regex-core/src/engine.rs)  
Action: `match_line`/`match_string`/`find_index` を `&[u8]` と `BTreeSet<Vec<u8>>` に変更  
Why: バイト列で直接マッチングするため  
Dependencies: ステップ4  
Risk: 中（最小インデックス探索の実装ミスに注意）

6. **find_index の実装方針** (File: crates/regex-core/src/engine.rs)  
Action: `find_subslice` ヘルパーで `windows` を使う単純実装、長さ1は単純ループで最適化  
Why: 依存追加を避けつつ速度を確保  
Dependencies: ステップ5  
Risk: 低

### フェーズ3: 公開APIと周辺更新
7. **Regex API 追加と委譲** (File: crates/regex-core/src/lib.rs)  
Action: `is_match_bytes(&[u8])` を追加し、`is_match(&str)` は `as_bytes()` 委譲  
Why: 互換性を保ちつつバイトAPIを提供  
Dependencies: フェーズ2  
Risk: 低

8. **ASCII ignore-case の統一** (File: crates/regex-core/src/lib.rs)  
Action: パターン・入力の両方を ASCII 小文字化するヘルパーを追加し使用  
Why: バイト指向で一貫した大小文字無視にする  
Dependencies: ステップ7  
Risk: 中（既存の Unicode ケースフォールドとの互換性が変わる）

9. **first_strings のバイト化** (File: crates/regex-core/src/lib.rs)  
Action: `BTreeSet<Vec<u8>>` に変更、`get_string` は `Vec<u8>` を構築  
Why: 先頭リテラル最適化の維持  
Dependencies: フェーズ2  
Risk: 低

10. **ドキュメント更新** (File: README.md, crates/regex-core/src/lib.rs)  
Action: バイト指向の意味と ASCII のみの `-i` を明記  
Why: 仕様変更の周知  
Dependencies: ステップ7-8  
Risk: 低

11. **テスト修正と追加** (Files: crates/regex-core/src/lib.rs, crates/regex-core/src/engine/*.rs)  
Action: `Char::Literal` の型変更に合わせてテストを更新  
Action: `is_match_bytes` の新規テスト追加  
Why: 変更点の回帰を防止  
Dependencies: フェーズ1-3  
Risk: 低

## テスト戦略
- ユニットテスト: `crates/regex-core/src/lib.rs` の既存テストを更新して通す  
- ユニットテスト: `crates/regex-core/src/engine/*.rs` の既存テストを更新して通す  
- 追加テスト: `is_match_bytes` の正常系（例: `b"zab"` が `ab` にマッチ）  
- 追加テスト: `ignore_case` の ASCII 動作（例: `b"ABC"` が `ab` にマッチ）

## リスクと対策
- **Risk**: 非 ASCII に対する `-i` の挙動が Unicode から ASCII に変わる  
  - Mitigation: README と doc コメントに明記し、テストで ASCII のみを保証
- **Risk**: パターン位置エラーが byte index 基準になる  
  - Mitigation: 変更点として明記、既存テストで位置依存がないことを確認

## 成功基準
- [ ] 既存テストがすべてパスする  
- [ ] `is_match_bytes` の追加テストがパスする  
- [ ] README と doc コメントにバイト指向・ASCII `-i` を明記している  
- [ ] API 互換性が保たれている（`Regex::is_match(&str)` が残っている）
