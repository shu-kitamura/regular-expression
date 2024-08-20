use super::{
    parser::AST,
    instruction::Instruction
};


/// コード生成器の型
#[derive(Default, Debug)]
struct Generator {
    p_counter: usize,
    instructions: Vec<Instruction>,
}