mod engine;
mod error;
mod cli;

use clap::Parser;
use engine::match_line;
use cli::Args;

fn main() {
    let args: Args = Args::parse();
    let pattern = args.pattern;

    // println!("{}", match_line("ab(c|d)", "abc").unwrap());
    // println!("{}", match_line("ab(c|d)", "xabcdefg").unwrap());
    println!("{}", match_line("ab(c|d)", "abx").unwrap());



}
