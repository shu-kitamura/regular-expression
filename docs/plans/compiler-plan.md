# Compiler 実装計画: `Ast::Char(u8)` への追随

## 概要
parser で `Ast::Char` を `u8` 化する前提で、compiler を byte 指向に調整します。  
公開関数のシグネチャは維持しつつ、命令生成で `Char::Literal(u8)` を扱えるように変更します。

## 重要な公開APIの変更
- 公開APIのシグネチャ変更はなし  
  `compile(&Ast) -> Result<Vec<Instruction>, CompileError>` は維持
- `Instruction::Char(Char::Literal(..))` の内部型が `u8` になるため、compiler の生成ロジックがそれに合わせて変わる

## 実装手順
1. **Ast 型変更への追随** (File: `crates/regex-core/src/engine/compiler.rs`)  
Action: `gen_char(&mut self, c: u8)` に変更し、`Instruction::Char(Char::Literal(c))` を生成  
Why: `Ast::Char(u8)` をそのまま命令に落とすため  
Dependencies: `parser` で `Ast::Char(u8)` 化が完了していること  
Risk: 低

2. **Ast マッチ分岐の更新** (File: `crates/regex-core/src/engine/compiler.rs`)  
Action: `Ast::Char(c)` の `c` が `u8` 前提になるため、関連箇所を整合  
Why: 型エラーを避ける  
Dependencies: 手順1  
Risk: 低

3. **AnyChar/Qualifier/Or/Seq の挙動確認** (File: `crates/regex-core/src/engine/compiler.rs`)  
Action: `AnyChar`, `Plus`, `Star`, `Question`, `Or`, `Seq` はロジック変更不要だが型整合を確認  
Why: 影響範囲を明確化し、不要な変更を避ける  
Dependencies: 手順1-2  
Risk: 低

4. **テスト更新** (File: `crates/regex-core/src/engine/compiler.rs`)  
Action: 既存テスト内の `Ast::Char('a')` を `Ast::Char(b'a')` に更新  
Action: 期待命令列の `Char::Literal('a')` を `Char::Literal(b'a')` に更新  
Why: 型変更に追随してテストを維持  
Dependencies: 手順1-2  
Risk: 低

5. **ドキュメントコメントの整合** (File: `crates/regex-core/src/engine/compiler.rs`)  
Action: コメント中の「char」表現を「byte」へ簡潔に更新（必要最低限）  
Why: 実装と説明の不整合回避  
Dependencies: 手順1  
Risk: 低

## テストケース
- 既存の compiler テストが全て通ること  
  例: `gen_char` が `Instruction::Char(Char::Literal(b'a'))` を生成  
  例: `"a*b"` の期待命令列が byte literal で一致する

## リスクと対策
- **Risk**: テスト更新漏れで型不一致が残る  
  - Mitigation: `rg "Ast::Char\\('" crates/regex-core/src/engine/compiler.rs` で漏れ確認
- **Risk**: parser 側変更前に compiler を触ると一時的にビルド不能  
  - Mitigation: parser 変更と同一ブランチで段階的にコミットする

## 明示的な前提
- `Instruction::Char` の内部型は `u8` で確定している  
- `CompileError` の型・意味は変更しない  
- `compile` の公開シグネチャは維持する

## 成功基準
- [ ] `crates/regex-core/src/engine/compiler.rs` のテストがすべてパスする  
- [ ] `gen_char` が `u8` を正しく命令に落とせる  
- [ ] コメントと実装の整合が取れている
