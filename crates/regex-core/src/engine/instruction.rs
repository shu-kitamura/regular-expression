//! Instruction set used by the compiler and evaluator.
#![allow(dead_code)]

use std::fmt::{self, Display};

use crate::engine::ast::{CharClass, Predicate};

/// Executable instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    /// Match a single character against a character class.
    CharClass(CharClass),
    /// Evaluate a zero-width assertion.
    Assert(Predicate),
    /// Store the start index of a capture group.
    SaveStart(usize),
    /// Store the end index of a capture group.
    SaveEnd(usize),
    /// Match the same text as a previously captured group.
    Backref(usize),
    /// Branch execution into two instruction addresses.
    Split(usize, usize),
    /// Unconditional jump.
    Jump(usize),
    /// Successful match terminator.
    Match,
}

impl Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::CharClass(class) => {
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
            Instruction::Assert(predicate) => write!(f, "assert {predicate:?}"),
            Instruction::SaveStart(index) => write!(f, "save_start {index}"),
            Instruction::SaveEnd(index) => write!(f, "save_end {index}"),
            Instruction::Backref(index) => write!(f, "backref {index}"),
            Instruction::Split(addr1, addr2) => write!(f, "split {addr1:>04}, {addr2:>04}"),
            Instruction::Jump(addr) => write!(f, "jump {addr:>04}"),
            Instruction::Match => write!(f, "match"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        ast::{CharClass, CharRange, Predicate},
        instruction::Instruction,
    };

    #[test]
    fn test_instruction_fmt() {
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
            format!("{}", Instruction::CharClass(class)),
            "charclass [a-z,0-9]"
        );
        assert_eq!(
            format!("{}", Instruction::Assert(Predicate::StartOfLine)),
            "assert StartOfLine"
        );
        assert_eq!(format!("{}", Instruction::SaveStart(1)), "save_start 1");
        assert_eq!(format!("{}", Instruction::SaveEnd(1)), "save_end 1");
        assert_eq!(format!("{}", Instruction::Backref(1)), "backref 1");
        assert_eq!(format!("{}", Instruction::Split(2, 10)), "split 0002, 0010");
        assert_eq!(format!("{}", Instruction::Jump(10)), "jump 0010");
        assert_eq!(format!("{}", Instruction::Match), "match");
    }

    #[test]
    fn test_instruction_fmt_negated_class() {
        let class = CharClass::new(
            vec![CharRange {
                start: 'a',
                end: 'a',
            }],
            true,
        );
        assert_eq!(
            format!("{}", Instruction::CharClass(class)),
            "charclass ^[a-a]"
        );
    }
}
