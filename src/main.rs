mod engine;
mod error;
mod cli;

use clap::Parser;
use engine::match_line;
use cli::Args;

fn main() {
    let mut args: Args = Args::parse();
    let patterns = match args.get_patterns(){
        Ok(pattern_list) => pattern_list,
        Err(e) => {
            eprintln!("{e}");
            return
        },
    };
    println!("{:?}", patterns);

    let files = match args.get_files() {
        Ok(file_list) => file_list,
        Err(e) => {
            eprintln!("{e}");
            return
        }
    };
    println!("{:?}", files);

    let is_count = args.count;
    let is_ignore_case = args.ignore_case;
    let is_invert_match = args.invert_match;

    // println!("{}", match_line("ab(c|d)", "abc").unwrap());
    // println!("{}", match_line("ab(c|d)", "xabcdefg").unwrap());
    println!("{}", match_line("ab(c|d)", "abx").unwrap());



}
