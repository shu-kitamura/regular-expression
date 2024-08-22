mod engine;
mod error;
mod helper;

use engine::do_match;

fn main() {
    println!("{}", do_match("ab(c|d)", "abc").unwrap());
    println!("{}", do_match("ab(c|d)", "abx").unwrap());
}
