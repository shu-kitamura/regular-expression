mod engine;
mod error;
mod cli;

use std::borrow::Borrow;

use clap::Parser;
use engine::match_line;
use cli::Args;

fn main() {
    let mut args: Args = Args::parse();
    let mut patterns = match args.get_patterns(){
        Ok(pattern_list) => pattern_list.clone(),
        Err(e) => {
            eprintln!("{e}");
            return
        },
    };

    println!("{:?}", patterns);

    for i in 0..patterns.len() {
        if args.ignore_case {
            patterns[i] = patterns[i].to_lowercase();
        }
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

    let ms = "ABCD".to_string();
    ignore_case(ms);

    let l = "abx";
    match match_line("ab(c|d)", l) {
        Ok(res) => if (res && !args.invert_match) || (!res && args.invert_match) {
            println!("{l}")
        },
        Err(e) => eprintln!("{e}"),
    };

    // println!("{}", match_line("ab(c|d)", "abc").unwrap());
    // println!("{}", match_line("ab(c|d)", "xabcdefg").unwrap());
}

fn ignore_case(s:String) -> String {
    println!("{} : {}", s, s.to_lowercase());

    s.to_lowercase()
}
