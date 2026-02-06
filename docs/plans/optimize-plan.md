# 実装計画: `regex-core` バイト検索経路の再設計（正しさ回復 + 高速化）

## 概要
現状の `first_strings` 最適化は一部パターンで false negative を起こしており（例: `a|b|c` が `b` を取りこぼす、`a*`/`a?` が空一致を取りこぼす）、性能以前に探索フィルタが不正確です。  
今回の計画では、探索フィルタを「正しさが保証できる形」に置き換えた上で、`is_match_bytes` の余計なコピーを除去し、検索経路の実行効率を上げます。

## 公開API/インターフェース変更
- 公開APIは維持: `Regex::new`, `Regex::is_match`, `Regex::is_match_bytes` のシグネチャは変更しない。
- 内部インターフェースは変更:
- `Regex` の内部フィールド `first_strings` を廃止し、`search_plan`（新規内部型）へ置換。
- `engine::match_line` に `search_plan` と `is_ignore_case` を渡す形へ変更。
- `evaluator` に「再利用可能 scratch を受ける評価関数」を追加し、開始位置ごとの `HashSet` 再確保を回避。

## 実装ステップ
1. `crates/regex-core/src/lib.rs` と `crates/regex-core/src/engine.rs` に再現テストを先行追加し、既知の取りこぼしを固定化する。対象は `a|b|c`、`(ab|cd|ef)`、`a?`、`a*`、空入力、非UTF-8入力の `is_match_bytes`。
2. `crates/regex-core/src/engine.rs`（必要なら `crates/regex-core/src/engine/search_plan.rs` 新設）に `SearchPlan` を導入する。内容は `can_match_empty`、`first_byte_mask`、`has_any_first_byte`、`leading_literal`（安全に確定できる場合のみ）とする。
3. `SearchPlan` 構築ロジックを実装する。命令列先頭から `Jump`/`Split` の ε遷移を辿る到達解析で「最初に消費される1バイト候補」と「空一致可能性」を算出し、ループは訪問済みPCで停止する。
4. `leading_literal` は「分岐なし・連続 `Char::Literal` のみ」の決定的ケースでのみ構築し、それ以外では無効化する。これにより最適化が正しさを壊さないことを保証する。
5. `match_line` を全面置換する。`is_caret` は単発評価、`can_match_empty && !is_dollar` は即時成功、その他は候補開始位置を `SearchPlan` で絞って評価する。`i == line.len()`（空一致候補）も必要時に評価対象へ含める。
6. `find_index` / `find_subslice` / `from_utf8(...).find(...)` 経路を削除する。探索は純粋にバイト比較で統一する。
7. `crates/regex-core/src/lib.rs` の `is_match_bytes` から入力全体の `to_ascii_lowercase_bytes` コピーを除去する。代わりに `is_ignore_case` を内部評価へ渡し、比較時に `to_ascii_lowercase` を適用する。
8. `crates/regex-core/src/engine/evaluator.rs` で scratch 再利用APIを追加し、`match_line` の複数開始位置評価で `HashSet` を `clear` 再利用する。
9. `crates/regex-core/src/engine.rs` または `crates/regex-core/tests/` に `#[ignore]` の性能計測テストを追加する。`Instant` + `black_box` でケース別の計測値を出し、CIの通常実行には含めない。
10. 検証を実行する。`cargo test -p regex-core`、`cargo clippy -p regex-core -- -D warnings`、必要に応じて `cargo test -p regex-core -- --ignored`、`cargo test -p regex-cli` を実施する。

## テストケースとシナリオ
- 正しさ回帰:
- `a|b|c` が `a/b/c` すべてにマッチする。
- `(ab|cd|ef)` が `ab/cd/ef` すべてにマッチする。
- `a?` と `a*` が非アンカー時に空一致を許容する。
- `^$`、`$`、`^` の既存アンカー挙動を維持する。
- `is_match_bytes` で非UTF-8バイト列入力でも一致判定が破綻しない。
- `-i` は ASCII のみ大小無視を維持する（非ASCIIは現行どおり）。
- 性能計測（`#[ignore]`）:
- 長い入力 + 長いリテラル先頭（miss多め）。
- 多分岐オルタネーション（`a|b|c|...`）。
- バイナリ入力（UTF-8不正バイト含む）。
- 目的は数値可視化と回帰検知であり、厳密時間閾値による fail は設定しない。

## リスクと対策
- リスク: `SearchPlan` 解析の不備で候補を削りすぎる。
- 対策: 解析は「安全側（不明なら絞らない）」を原則にし、取りこぼし再現テストを先に固定する。
- リスク: 空一致判定の扱い変更で期待結果が変わる。
- 対策: 空一致の仕様をテストで明文化し、アンカー有無で期待値を分離する。
- リスク: ignore-case のオンザフライ比較で分岐コストが増える。
- 対策: 候補位置絞り込みを先に実施し、比較回数自体を減らす。計測テストで確認する。

## 明示的な前提とデフォルト
- 依存クレートは追加しない（標準ライブラリのみ）。
- 公開APIは変更しない。
- 互換性は「性能優先で再定義」を採用し、現在の取りこぼしは修正対象とする。
- 性能計測テストは `#[ignore]` で運用し、通常CIの失敗条件にはしない。
