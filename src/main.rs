mod engine;
mod error;
mod helper;

use engine::{
    parser::parse,
    codegen::gen_code,
};
fn main() {
    let mut ast = parse("ab*(c|d)").unwrap();
    println!("{:?}", ast);
    println!("{:?}", gen_code(&ast).unwrap());

    ast = parse(r"a|b").unwrap();
    println!("{:?}", gen_code(&ast));
}

