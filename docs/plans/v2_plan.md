# 実装計画: `compiler_v2` / `evaluator_v2` 追加（後方参照対応）

## 概要
`parser_v2` が返す `engine::ast::Ast` を実行可能にするため、既存 v1 系とは独立した `instruction_v2` / `compiler_v2` / `evaluator_v2` を追加する。  
方針は「v1維持」「v2は段階導入」で、現時点では `compiler_v2` / `evaluator_v2` 実装と単体テストまでを対象にし、`lib.rs` や既存 `Regex` API への統合は行わない。  
後方参照は GNU `grep -E` 相当で扱い、不正参照（例: `(a)\2`）は**コンパイルエラー**とする。

## 確定した仕様（意思決定）
- 対応範囲: `ast::Ast` 全ノード対応
- 既存 v1 系: 変更しない（共存）
- IR: `instruction_v2` を新設（既存 `Instruction` は不変更）
- 後方参照:
1. 参照先キャプチャ未確定時はマッチ失敗（false）
2. 不正な参照番号（存在しない capture index）はコンパイルエラー
- 公開範囲: 現時点は `compiler_v2` / `evaluator_v2` 追加まで（`RegexV2` は後続）

## 追加/変更する主要インターフェース
- `crates/regex-core/src/engine/instruction_v2.rs`
1. `InstructionV2` を新規追加
2. 予定バリアント:
   - `CharClass(CharClass)`  
   - `Assert(Predicate)`  
   - `SaveStart(usize)`  
   - `SaveEnd(usize)`  
   - `Backref(usize)`  
   - `Split(usize, usize)`  
   - `Jump(usize)`  
   - `Match`
- `crates/regex-core/src/engine/compiler_v2.rs`
1. `pub fn compile_v2(ast: &crate::engine::ast::Ast) -> Result<Vec<InstructionV2>, CompileV2Error>`
2. `CompileV2Error` をこのモジュール内で新規定義
   - `PCOverFlow`
   - `InvalidBackreference(usize)`
- `crates/regex-core/src/engine/evaluator_v2.rs`
1. `pub fn eval_v2(inst: &[InstructionV2], input: &str) -> Result<bool, EvalV2Error>`
2. `EvalV2Error` をこのモジュール内で新規定義
   - `PCOverFlow`
   - `CharIndexOverFlow`
   - `InvalidPC`
3. 必要なら内部補助:
   - `eval_from_start_v2(...)`
   - `is_word_char(...)`

## 実装手順

### フェーズ1: v2命令セットの追加
1. `instruction_v2.rs` を新規作成し、`InstructionV2` と表示用 `Display` を実装
2. `engine.rs` に `mod instruction_v2; mod compiler_v2; mod evaluator_v2;` を追加
3. v1 の `instruction.rs` は一切変更しない

### フェーズ2: `compiler_v2` 実装
1. `Ast` 走査コンパイラを新規実装
2. ノード変換規則
   - `Empty`: no-op
   - `CharClass`: `InstructionV2::CharClass`
   - `Assertion`: `InstructionV2::Assert`
   - `Capture {expr, index}`: `SaveStart(index)` -> expr -> `SaveEnd(index)`
   - `ZeroOrMore/OneOrMore/ZeroOrOne`: `Split/Jump` で Thompson 変換（`greedy` で分岐順制御）
   - `Repeat {min,max}`:
     - `min` 回は必須展開
     - `max=None` は残りを `ZeroOrMore` 相当
     - `max=Some(n)` は `n-min` 回を `Split` による任意展開
   - `Concat`: 順次展開
   - `Alternate`: `Split + Jump + patch`
   - `Backreference(i)`: `InstructionV2::Backref(i)`
3. 事前検証
   - AST 全体から最大 capture index を収集
   - 各 `Backreference(i)` で `i==0 || i>max_capture` を `InvalidBackreference(i)` にする

### フェーズ3: `evaluator_v2` 実装
1. 入力文字列を `Vec<char>` 化（1回）
2. 探索モデル
   - 先頭固定ではなく全開始位置を試行（grep 互換）
   - `Split` で分岐するため、状態はバックトラックで保持
3. 状態データ
   - `pc`
   - `char_index`
   - `capture_start: Vec<Option<usize>>`
   - `capture_end: Vec<Option<usize>>`
4. 命令評価規則
   - `CharClass`: 現在文字の範囲一致判定（`negated` を反映）
   - `Assert`:
     - `StartOfLine`: `index==0 || prev=='\n'`
     - `EndOfLine`: `index==len || curr=='\n'`
     - `StartOfText`: `index==0`
     - `EndOfText`: `index==len`
     - `WordBoundary` / `NonWordBoundary`: `is_word_char` 境界判定
   - `SaveStart/SaveEnd`: 現在位置を capture テーブルに保存
   - `Backref(i)`:
     - start/end が揃っていなければ失敗
     - capture 内容と入力残りを逐次比較
     - 一致ならその文字数分だけ `char_index` 前進
   - `Split/Jump/Match`: 既存 v1 同様
5. 無限ループ回避
   - `(pc, char_index, capture_hash)` を visited キーに採用  
   - `capture_hash` は capture 配列内容の安定ハッシュ（または clone比較可能な軽量表現）  
   - これにより backref あり分岐でも誤剪定を防ぐ

### フェーズ4: テスト
1. `instruction_v2` 単体
   - `Display` 表示
2. `compiler_v2` 単体
   - 基本: `abc`, `a|b`, `a*`, `a{2,3}`
   - 文字クラス/アサーション: `[a-z]`, `^abc$`
   - capture/backref: `(abc)\1`
   - 異常: `(a)\2` -> `InvalidBackreference(2)`
3. `evaluator_v2` 単体（手書き命令 + compile結果の両方）
   - マッチ: `(abc)\1` with `abcabc`
   - 不一致: `(abc)\1` with `abcabd`
   - 未確定capture参照失敗: `(a)?\1` with `a`/`` など
   - 文字クラス否定: `d[^io]g`
   - アンカー: `^abc$`, `^$`
4. 既存回帰
   - `cargo test -p regex-core --lib` で v1 テストが壊れていないこと確認
   - `cargo test -p regex-core compiler_v2`
   - `cargo test -p regex-core evaluator_v2`

## 想定リスクと対策
- リスク: backtracking + capture状態で visited 設計を誤ると誤判定
  - 対策: visitedキーに capture状態を含める。最初に小ケースで妥当性確認
- リスク: `Repeat` 展開で命令サイズ増大
  - 対策: まず素直実装（min/max 展開）で correctness 優先
- リスク: v1 既存コードへの影響
  - 対策: v2は新モジュールで分離、既存型を変更しない

## 受け入れ基準
- `compiler_v2` が `ast::Ast` 全ノードを受理して `InstructionV2` を生成できる
- `evaluator_v2` が capture/backreference を含むパターンを評価できる
- `(a)\2` がコンパイル時に失敗する
- v1 既存テストが全て通る
- v2 新規テストが全て通る

## 前提・デフォルト
- `compiler_v2/evaluator_v2` のみ追加し、`RegexV2`/`lib.rs` 統合は後続
- `WordBoundary` は ASCII word (`[A-Za-z0-9_]`) として実装
- `parser_v2` の現仕様（GNU ERE寄り）を前提に v2 実行系を合わせる
