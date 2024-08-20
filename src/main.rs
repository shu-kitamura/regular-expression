mod parser;
mod error;

use parser::parse;
fn main() {
    println!("{:?}", parse("ab*c+d(qq|pp)").unwrap());
    println!("{:?}", parse(r"ab*c\+d(qq|pp)").unwrap());
}

