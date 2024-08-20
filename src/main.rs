mod engine;
mod error;
mod helper;

use engine::parser::parse;
fn main() {
    println!("{:?}", parse("ab*c+d(qq|pp)").unwrap());
    println!("{:?}", parse(r"ab*c\+d(qq|pp)").unwrap());
}

