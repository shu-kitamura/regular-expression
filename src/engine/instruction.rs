//! コード生成時に使用する型。  
//! codegen モジュールで使用する。

use std::fmt::{self, Display};

/// コード生成時に使用する命令(Instruction)を表す型
#[derive(Debug, PartialEq)]
pub enum Instruction {
    Char(Char),           // 文字列をマッチする命令
    Match,                // マッチング成功を示す命令
    Jump(usize),          // 指定した命令アドレスにジャンプする命令
    Split(usize, usize),  // 指定した2つの命令アドレスに分岐する命令
}

/// 文字を表す型
#[derive(Debug, PartialEq)]
pub enum Char {
    Literal(char), // 指定された文字（'a', 'b'など）に対応する
    Any,           // 任意の文字('.')に対応する
}

impl Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Char(char) => match char {
                Char::Literal(c) => write!(f, "char {}", c),
                Char::Any => write!(f, "char any")
            },
            Instruction::Match => write!(f, "match"),
            Instruction::Jump(addr) => write!(f, "jump {:>04}", addr),
            Instruction::Split(addr1, addr2) => write!(f, "split {:>04}, {:>04}", addr1, addr2),
        }
    }
}