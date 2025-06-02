//! Instruction と char配列を受け取って評価する

use std::collections::HashSet;

use crate::{
    engine::{
        instruction::{Char, Instruction},
        safe_add,
    },
    error::EvalError,
};

/// char と Instruction を評価する
fn eval_char(inst: &Char, string: &str, index: usize) -> bool {
    let inst_char = match inst {
        Char::Literal(c) => *c,
        Char::Any => return true,
    };

    string.chars().nth(index) == Some(inst_char)
}

/// プログラムカウンタとchar配列のインデックスをインクリメントする
fn increment_pc_and_index(pc: &mut usize, index: &mut usize) -> Result<(), EvalError> {
    safe_add(pc, &1, || EvalError::PCOverFlow)?;
    safe_add(index, &1, || EvalError::CharIndexOverFlow)
}

/// 深さ優先探索で再帰的にマッチングを行う関数
fn eval_depth(
    instructions: &[Instruction],
    string: &str,
    mut p_counter: usize,
    mut char_index: usize,
    is_end_dollar: bool,
    visited: &mut HashSet<(usize, usize)>,
) -> Result<bool, EvalError> {
    loop {
        // Instruction を取得
        let instruction: &Instruction = match instructions.get(p_counter) {
            Some(inst) => inst,
            None => return Err(EvalError::InvalidPC),
        };

        // Instruction の型に応じて、評価を実行。
        match instruction {
            Instruction::Char(inst_char) => {
                if eval_char(inst_char, string, char_index) {
                    increment_pc_and_index(&mut p_counter, &mut char_index)?;
                } else {
                    return Ok(false);
                };
            }
            Instruction::Match => {
                if is_end_dollar {
                    return Ok(string.len() == char_index);
                } else {
                    return Ok(true);
                }
            }
            Instruction::Jump(addr) => p_counter = *addr,
            Instruction::Split(addr1, addr2) => {
                // すでに訪れた状態の場合、無限ループを避けるために false を返す
                if is_visited(visited, *addr1, char_index) {
                    return Ok(false);
                }

                // 1つ目の Split を評価する
                if eval_depth(
                    instructions,
                    string,
                    *addr1,
                    char_index,
                    is_end_dollar,
                    visited,
                )? {
                    return Ok(true);
                }

                // 1つ目の Split が失敗した場合、2つ目の Split を評価する
                return eval_depth(
                    instructions,
                    string,
                    *addr2,
                    char_index,
                    is_end_dollar,
                    visited,
                );
            }
        }
    }
}

/// 命令列の評価を行う関数
pub fn eval(inst: &[Instruction], string: &str, is_end_dollar: bool) -> Result<bool, EvalError> {
    let mut visited = HashSet::new();
    eval_depth(inst, string, 0, 0, is_end_dollar, &mut visited)
}

fn is_visited(visited: &mut HashSet<(usize, usize)>, addr: usize, char_index: usize) -> bool {
    if addr <= char_index {
        return !visited.insert((addr, char_index));
    }
    false
}

// ----- テストコード -----

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{
        engine::{
            evaluator::{eval_char, eval_depth, increment_pc_and_index},
            instruction::{Char, Instruction},
        },
        error::EvalError,
    };

    #[test]
    fn test_eval_char_true() {
        let actual: bool = eval_char(&Char::Literal('a'), "abc", 0);
        assert_eq!(actual, true);
    }

    #[test]
    fn test_eval_char_false() {
        let actual1: bool = eval_char(&Char::Literal('a'), "abc", 1);
        assert_eq!(actual1, false);

        let actual2: bool = eval_char(&Char::Literal('a'), "abc", 10);
        assert_eq!(actual2, false);
    }

    #[test]
    fn test_eval_char_any() {
        let actual: bool = eval_char(&Char::Any, "abc", 0);
        assert_eq!(actual, true);
    }

    #[test]
    fn test_increment_success() {
        let pc: &mut usize = &mut 10;
        let index: &mut usize = &mut 10;
        let _ = increment_pc_and_index(pc, index);

        assert_eq!(pc, &mut 11);
        assert_eq!(index, &mut 11);
    }

    #[test]
    fn test_increment_pc_overflow() {
        let mut u = usize::MAX;
        let actual = increment_pc_and_index(&mut u, &mut 1);
        assert_eq!(actual, Err(EvalError::PCOverFlow));
    }

    #[test]
    fn test_increment_charindex_overflow() {
        let mut u = usize::MAX;
        let actual = increment_pc_and_index(&mut 1, &mut u);
        assert_eq!(actual, Err(EvalError::CharIndexOverFlow));
    }

    #[test]
    fn test_eval_depth_true() {
        // "ab(c|d)" が入力された Instruction
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Split(3, 5),
            Instruction::Char(Char::Literal('c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal('d')),
            Instruction::Match,
        ];

        // "abc" とマッチするケース
        let mut visited1: HashSet<(usize, usize)> = HashSet::new();
        let actual1 = eval_depth(&insts, "abc", 0, 0, false, &mut visited1).unwrap();
        assert_eq!(actual1, true);

        // "abd"とマッチするケース
        let mut visited2: HashSet<(usize, usize)> = HashSet::new();
        let actual2 = eval_depth(&insts, "abc", 0, 0, false, &mut visited2).unwrap();
        assert_eq!(actual2, true);
    }

    #[test]
    fn test_eval_depth_false() {
        // "ab(c|d)" が入力された Instruction
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Split(3, 5),
            Instruction::Char(Char::Literal('c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal('d')),
            Instruction::Match,
        ];

        // "abx" とマッチするケース
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        let actual = eval_depth(&insts, "abX", 0, 0, false, &mut visited).unwrap();
        assert_eq!(actual, false);
    }

    #[test]
    fn test_eval_depth_is_end_dollar() {
        // "ab(c|d)" が入力された Instruction
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Split(3, 5),
            Instruction::Char(Char::Literal('c')),
            Instruction::Jump(6),
            Instruction::Char(Char::Literal('d')),
            Instruction::Match,
        ];

        // "xxxabc" とマッチするケース (true になる)
        let mut visited1: HashSet<(usize, usize)> = HashSet::new();
        let actual1: bool = eval_depth(&insts, "abc", 0, 0, true, &mut visited1).unwrap();
        assert_eq!(actual1, true);

        // "abcxxx"とマッチするケース (false になる)
        let mut visited2: HashSet<(usize, usize)> = HashSet::new();
        let actual2: bool = eval_depth(&insts, "abcxxx", 0, 0, true, &mut visited2).unwrap();
        assert_eq!(actual2, false);
    }

    #[test]
    fn test_eval_depth_infinite_loop() {
        // "abc(d*)*" が入力された Instruction
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Char(Char::Literal('c')),
            Instruction::Split(4, 8),
            Instruction::Split(5, 7),
            Instruction::Char(Char::Literal('d')),
            Instruction::Jump(4),
            Instruction::Jump(3),
            Instruction::Match,
        ];

        // "abcde" とマッチするケース（true）
        let mut visited1: HashSet<(usize, usize)> = HashSet::new();
        let actual1 = eval_depth(&insts, "abcde", 0, 0, false, &mut visited1).unwrap();
        assert_eq!(actual1, true);

        // "bcdef" とマッチするケース（false）
        let mut visited2: HashSet<(usize, usize)> = HashSet::new();
        let actual2 = eval_depth(&insts, "bcdef", 0, 0, false, &mut visited2).unwrap();
        assert_eq!(actual2, false);
    }

    #[test]
    fn test_eval_depth_invalidpc() {
        let insts: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        let actual = eval_depth(&insts, "abcd", usize::MAX, 0, false, &mut visited);
        assert_eq!(actual, Err(EvalError::InvalidPC));
    }
}
