//! compiler_v2 / evaluator_v2 で使用する命令セット。
#![allow(dead_code)]

use std::fmt::{self, Display};

use crate::engine::ast::{CharClass, Predicate};

/// v2 系で使用する命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionV2 {
    CharClass(CharClass),
    Assert(Predicate),
    SaveStart(usize),
    SaveEnd(usize),
    Backref(usize),
    Split(usize, usize),
    Jump(usize),
    Match,
}

impl Display for InstructionV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstructionV2::CharClass(class) => {
                let neg = if class.negated { "^" } else { "" };
                write!(f, "charclass {neg}[")?;
                for (i, range) in class.ranges.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}-{}", range.start, range.end)?;
                }
                write!(f, "]")
            }
            InstructionV2::Assert(predicate) => write!(f, "assert {predicate:?}"),
            InstructionV2::SaveStart(index) => write!(f, "save_start {index}"),
            InstructionV2::SaveEnd(index) => write!(f, "save_end {index}"),
            InstructionV2::Backref(index) => write!(f, "backref {index}"),
            InstructionV2::Split(addr1, addr2) => write!(f, "split {addr1:>04}, {addr2:>04}"),
            InstructionV2::Jump(addr) => write!(f, "jump {addr:>04}"),
            InstructionV2::Match => write!(f, "match"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        ast::{CharClass, CharRange, Predicate},
        instruction_v2::InstructionV2,
    };

    #[test]
    fn test_instruction_v2_fmt() {
        let class = CharClass::new(
            vec![
                CharRange {
                    start: 'a',
                    end: 'z',
                },
                CharRange {
                    start: '0',
                    end: '9',
                },
            ],
            false,
        );

        assert_eq!(
            format!("{}", InstructionV2::CharClass(class)),
            "charclass [a-z,0-9]"
        );
        assert_eq!(
            format!("{}", InstructionV2::Assert(Predicate::StartOfLine)),
            "assert StartOfLine"
        );
        assert_eq!(format!("{}", InstructionV2::SaveStart(1)), "save_start 1");
        assert_eq!(format!("{}", InstructionV2::SaveEnd(1)), "save_end 1");
        assert_eq!(format!("{}", InstructionV2::Backref(1)), "backref 1");
        assert_eq!(
            format!("{}", InstructionV2::Split(2, 10)),
            "split 0002, 0010"
        );
        assert_eq!(format!("{}", InstructionV2::Jump(10)), "jump 0010");
        assert_eq!(format!("{}", InstructionV2::Match), "match");
    }

    #[test]
    fn test_instruction_v2_fmt_negated_class() {
        let class = CharClass::new(
            vec![CharRange {
                start: 'a',
                end: 'a',
            }],
            true,
        );
        assert_eq!(
            format!("{}", InstructionV2::CharClass(class)),
            "charclass ^[a-a]"
        );
    }
}
