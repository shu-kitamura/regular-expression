# regular-expression

Rust で実装した正規表現エンジンと grep 風 CLI です。

このワークスペースは次の 2 クレートで構成されています。
- `regex-core`: parser/compiler/evaluator を提供するライブラリ
- `regex-cli`: `regex-core` を使った CLI ツール（`regex`）

## インストール

```sh
cargo install --git https://github.com/shu-kitamura/regular-expression.git -p regex-cli
```

## CLI の使い方

```text
regex [OPTIONS] PATTERN [FILE...]
regex [OPTIONS] -e PATTERN [-e PATTERN ...] [FILE...]
command | regex [OPTIONS] PATTERN
command | regex [OPTIONS] -e PATTERN [-e PATTERN ...]
```

入力ファイルを指定しない場合は、標準入力（stdin）から読み込みます。

## オプション

- `-e, --regexp <PATTERN>`: パターンを追加する（複数指定可）
- `-c, --count`: マッチした行数のみ表示する
- `-i, --ignore-case`: 大文字小文字を区別しない
- `-v, --invert-match`: 非マッチ行を選択する
- `-h, --no-filename`: 出力にファイル名を表示しない
- `-H, --with-filename`: 出力に常にファイル名を表示する
- `-n, --line-number`: 出力行に行番号を付ける
- `--help`: ヘルプを表示する
- `-V, --version`: バージョンを表示する

補足:
- `-h` と `-H` は同時に指定できません。
- 複数ファイルを入力した場合、デフォルトでファイル名を表示します。
- 1 ファイル入力（または stdin）の場合、デフォルトでファイル名を表示しません。

## 対応している正規表現構文

- リテラル（例: `abc`）
- エスケープされたリテラル（例: `\*`, `\+`, `\\`）
- ワイルドカード: `.`
- 文字クラス: `[abc]`、範囲 `[a-z]`、否定クラス `[^0-9]`
- 量指定子: `*`, `+`, `?`, `{m}`, `{m,}`, `{m,n}`
- グルーピングと選択: `(ab|cd)`
- キャプチャと後方参照: `(abc)\1`
- アンカー: `^`, `$`

現在の制限:
- 非貪欲量指定子（`*?`, `+?`, `??`, `{m,n}?`）は未対応です。
- 非キャプチャグループ（`(?:...)`）は未対応です。
- `\d` や `\w` は特殊クラスではなく、リテラル文字（`d`, `w`）として扱われます。

## 使用例

```sh
# ファイルを検索
regex "ho*" test.txt

# 大文字小文字を無視
regex -i "ho*" test.txt

# マッチ件数を表示
regex -c "ho*" test.txt

# 非マッチ行を表示
regex -v "ho*" test.txt

# 複数パターン（OR）
regex -e "ho*" -e "fu*" test.txt

# 標準入力を検索
cat test.txt | regex -e "ho*" -e "fu*"

# キャプチャ + 後方参照
regex "(abc)\\1" test.txt
```

## ライブラリ利用（`regex-core`）

```rust
use regex_core::Regex;

fn main() -> Result<(), regex_core::error::RegexError> {
    let re = Regex::new("(abc)\\1", false, false)?;
    assert!(re.is_match("abcabc")?);
    assert!(!re.is_match("abcabd")?);
    Ok(())
}
```

## 開発コマンド

```sh
# ビルド
cargo build

# CLI 実行
cargo run -p regex-cli --bin regex -- "a*b" sample.txt

# テスト
cargo test

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all

# ベンチマーク（criterion）
cargo bench -p regex-core
```

## ライセンス

このプロジェクトは MIT License で提供されています。  
詳細は `LICENCE.txt` を参照してください。
