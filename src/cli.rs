use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {

    #[arg(value_name = "PATTERN")]
    /// パターンを指定する。
    pub pattern: Option<String>,

    #[arg(value_name = "FILE")]
    /// ファイルを指定する。
    pub files: Vec<String>,

    #[arg(short = 'e', long = "regexp", value_name = "PATTERN")]
    /// パターンを指定する。このオプションを使用すれば複数のパターンを指定することができる
    pub patterns : Vec<String>,

    #[arg(short = 'c', long = "count")]
    /// マッチした行数のみ表示する
    pub count: bool,

    #[arg(short = 'i', long = "ignore-case")]
    /// マッチしなかった行を表示する
    pub ignore_case: bool,
    
    #[arg(short = 'v', long = "invert-match")]
    /// マッチしなかった行を表示する
    pub invert_match: bool,

}

impl Args {
    fn get_patterns(self) -> Result<Vec<String>, E> {
        self.patterns.len()
    }
}