# regular-expression

Rust製の正規表現コマンド。  
パターンにマッチする行を表示する。  

## 書式  

```text
regex [OPTIONS] PATTERN [FILE]
regex [OPTIONS] [-e PATTERN] [FILE]
```

## 説明

regex は FILE で名前を指定されたファイルを検索し、PATTERN にマッチする部分を含む行を探す。  
デフォルトの場合、マッチする部分を含む行を表示する。  

### 対応している正規表現

- \+
- \*
- ?
- |
- ()

## オプション

### プログラムについての一般情報

* --help  
コマンドのヘルプを表示する。
* -V, --version  
バージョンを表示する。

### マッチング・出力の制御  

* -e, --regexp PATTERN  
パターンを指定する。このオプションを複数回指定することで、複数のパターンを指定することができる。
* -v, --invert-match  
マッチの意味を逆にして、マッチしない行を抜き出して表示する。
* -i, --ignore-case  
PATTERN と入力ファイルの双方で、アルファベットの大文字と小文字を区別しない。
* -c, --count  
マッチした行を出力せず、マッチした行数を出力する。(-v と併用した場合、マッチしなかった行数を出力する)
* -h, --no-filename  
出力する行の前にファイル名を付けない。検索ファイルが1つの場合、こちらがデフォルト。
* -H, --with-filename  
出力する行の前にファイル名を付ける。検索ファイルが2つ以上の場合、こちらがデフォルト。

## インストール

以下の手順でインストールできる。  
任意のディレクトリは、パス通っている必要がある。
```shell
git clone https://github.com/shu-kitamura/regular-expression.git
cd regular-expression
cargo build
mv target/debug/regex [任意のディレクトリ]
```

## 使用例

```shell
$ cat test.txt # 使用するファイル
hoge hoge
HOGE HOGE
fuga fuga
FUGA FUGA

$ regex ho* test.txt # オプション無し
hoge hoge

$ regex -i ho* test.txt # 大文字と小文字を区別しない(-iオプション)
hoge hoge
HOGE HOGE

$ regex -ci ho* test.txt # マッチした行数を表示(-cオプション)
2

$ regex -iv ho* test.txt # マッチしなかった行を表示(-vオプション)
fuga fuga
FUGA FUGA

$ regex -e ho* -e fu* test.txt # 複数のパターンを指定(-eオプション)
hoge hoge
fuga fuga
```

# ライセンス

このプロジェクトは MIT ライセンスの下でライセンスされている。  
詳細については LICENCE.txt を参照してください。