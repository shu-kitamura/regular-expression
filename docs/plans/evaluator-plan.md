# Evaluator 実装計画: byte 指向評価への移行

## 概要
`evaluator` を `&str` ではなく `&[u8]` で評価するように変更します。  
`Instruction::Char(Char::Literal(u8))` を前提に、1バイト単位でマッチングを行います。

## 重要な公開APIの変更
- 公開関数 `eval(inst: &[Instruction], string: &str, is_end_dollar: bool)` を  
  `eval(inst: &[Instruction], input: &[u8], is_end_dollar: bool)` に変更
- `eval_depth` と `eval_char` の引数も `&[u8]` に変更

## 実装手順

1. **eval_char を byte 参照に変更** (File: `crates/regex-core/src/engine/evaluator.rs`)  
Action: `string.chars().nth(index)` を廃止し、`input.get(index)` で比較  
Action: `Char::Any` は常に true（ただし `index` 範囲外時は false）  
Why: O(1) のバイト参照で高速化する  
Dependencies: `Instruction::Char` が `u8` に変更済み  
Risk: 低

2. **eval_depth / eval の引数変更** (File: `crates/regex-core/src/engine/evaluator.rs`)  
Action: `string: &str` → `input: &[u8]` に変更  
Action: `string.len()` → `input.len()` に置換  
Why: byte 指向の評価に統一  
Dependencies: 手順1  
Risk: 低

3. **範囲外アクセスの明確化** (File: `crates/regex-core/src/engine/evaluator.rs`)  
Action: `eval_char` の範囲外時は false を返す  
Why: `.` や `Literal` が入力末尾を超えて一致しないことを保証  
Dependencies: 手順1  
Risk: 低

4. **テスト更新** (File: `crates/regex-core/src/engine/evaluator.rs`)  
Action: テスト入力を `&str` から `b"..."` に更新  
Action: `Char::Literal('a')` → `Char::Literal(b'a')` へ更新  
Why: 型変更に合わせてテストを維持  
Dependencies: 手順1-2  
Risk: 低

## テストケース
- `eval_char` が `b"abc"` に対して `b'a'` が index 0 で true
- `eval_char` が範囲外 index で false
- `eval_depth` / `eval` が既存テストと同じ結果を返す（入力は `b"..."`）

## リスクと対策
- **Risk**: `eval_char` で `Any` の範囲外が true になる  
  - Mitigation: `input.get(index)` を先に確認してから `Any` を true にする
- **Risk**: `is_end_dollar` 判定で `len` 比較がズレる  
  - Mitigation: `input.len()` に統一し、既存テストを維持

## 明示的な前提
- `Instruction::Char` の内部型は `u8`  
- `EvalError` の型・意味は変更しない  
- `eval` の戻り値の意味は維持する（マッチング成否のみ）

## 成功基準
- [ ] evaluator のユニットテストがすべてパスする  
- [ ] 既存のマッチング結果が byte 入力で同一となる  
- [ ] 範囲外アクセスがなく安全に評価できる
