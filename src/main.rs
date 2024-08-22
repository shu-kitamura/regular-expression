mod engine;
mod error;

use engine::match_line;

fn main() {
    println!("{}", match_line("ab(c|d)", "abc").unwrap());
    println!("{}", match_line("ab(c|d)", "xabcdefg").unwrap());
    println!("{}", match_line("ab(c|d)", "abx").unwrap());
}
