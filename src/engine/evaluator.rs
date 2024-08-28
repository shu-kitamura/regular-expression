//! Instruction と char配列を受け取って評価する

use crate::{
    error::EvalError,
    engine::{
        instruction::Instruction,
        helper::safe_add
    }
};

/// char と Instruction を評価する
fn eval_char(inst_char: &char, chars: &Vec<char>, index: usize)-> bool {
    match chars.get(index) {
        Some(c) => if c == inst_char {
            true
        } else {
            false
        }
        None => false
    }
}

/// プログラムカウンタとchar配列のインデックスをインクリメントする
fn increment_pc_and_index(pc: &mut usize, index: &mut usize) -> Result<(), EvalError> {
    match safe_add(pc, &1, || EvalError::PCOverFlow) {
        Ok(()) => {},
        Err(e) => return Err(e)
    };
    match safe_add(index, &1, || EvalError::CharIndexOverFlow) {
        Ok(()) => Ok(()),
        Err(e) => return Err(e),
    }
}

/// 深さ優先探索で再帰的にマッチングを行う関数
fn eval_depth(
    instructions: &[Instruction],
    chars: &Vec<char>,
    mut p_counter: usize,
    mut char_index: usize,
) -> Result<bool, EvalError> {
    loop {
        // Instruction を取得
        let instruction: &Instruction = match instructions.get(p_counter) {
            Some(inst) => inst,
            None => return Err(EvalError::InvalidPC)
        };

        // Instruction の型に応じて、評価を実行。
        match instruction {
            Instruction::Char(inst_char) => {
                if eval_char(inst_char, chars, char_index) {
                    match increment_pc_and_index(&mut p_counter, &mut char_index) {
                        Ok(()) => {},
                        Err(e) => return Err(e)
                    };
                } else {
                    return Ok(false)
                };
            }
            Instruction::Match => {
                return Ok(true);
            }
            Instruction::Jump(addr) => {
                p_counter = *addr;
            }
            Instruction::Split(addr1, addr2) => {
                if eval_depth(instructions, chars, *addr1, char_index)? || eval_depth(instructions, chars, *addr2, char_index)? {
                    return Ok(true);
                } else {
                    return Ok(false);
                }
            }
            Instruction::Period => {
                match increment_pc_and_index(&mut p_counter, &mut char_index) {
                    Ok(()) => {},
                    Err(e) => return Err(e)
                }
                if chars.len() < char_index {
                    return Ok(false)
                }
            }
        }
    }
}



/// 命令列の評価を行う関数
pub fn eval(inst: &[Instruction], chars:&Vec<char>) -> Result<bool, EvalError> {
    eval_depth(inst, chars, 0, 0)
}

// ----- テストコード -----

#[test]
fn test_eval_char_true() {
    let actual: bool = eval_char(&'a', &vec!['a', 'b', 'c'], 0);
    assert_eq!(actual, true);
}

#[test]
fn test_eval_char_false() {
    let actual1: bool = eval_char(&'a', &vec!['a', 'b', 'c'], 1);
    assert_eq!(actual1, false);

    let actual2: bool = eval_char(&'a', &vec!['a', 'b', 'c'], 10);
    assert_eq!(actual2, false);
}

#[test]
fn test_increment_success() {
    let pc: &mut usize = &mut 10;
    let index: &mut usize = &mut 10;
    let _ = increment_pc_and_index(pc, index);

    assert_eq!(pc,&mut 11);
    assert_eq!(index, &mut 11);
}

#[test]
fn test_increment_pc_overflow() {
    let actual = increment_pc_and_index(&mut 18446744073709551615, &mut 1);
    assert_eq!(actual, Err(EvalError::PCOverFlow));
}

#[test]
fn test_increment_charindex_overflow() {
    let actual = increment_pc_and_index(&mut 1, &mut 18446744073709551615);
    assert_eq!(actual, Err(EvalError::CharIndexOverFlow));
}

#[test]
fn test_eval_depth_true() {
    // "ab(c|d)" が入力された Instraction
    let insts: Vec<Instruction> = vec![
        Instruction::Char('a'),
        Instruction::Char('b'),
        Instruction::Split(3, 5),
        Instruction::Char('c'),
        Instruction::Jump(6),
        Instruction::Char('d'),
        Instruction::Match
    ];

    // "abc" とマッチするケース
    let chars1:Vec<char> = vec!['a', 'b', 'c'];
    
    let actual1 = eval_depth(&insts, &chars1, 0, 0).unwrap();
    assert_eq!(actual1, true);

    // "abd"とマッチするケース
    let chars2:Vec<char> = vec!['a', 'b', 'c'];
    let actual2 = eval_depth(&insts, &chars2, 0, 0).unwrap();
    assert_eq!(actual2, true);
}

#[test]
fn test_eval_depth_false() {
    // "ab(c|d)" が入力された Instraction
    let insts: Vec<Instruction> = vec![
        Instruction::Char('a'),
        Instruction::Char('b'),
        Instruction::Split(3, 5),
        Instruction::Char('c'),
        Instruction::Jump(6),
        Instruction::Char('d'),
        Instruction::Match
    ];

    // "abx" とマッチするケース
    let chars:Vec<char> = vec!['a', 'b', 'X'];

    let actual = eval_depth(&insts, &chars, 0, 0).unwrap();
    assert_eq!(actual, false);
}

#[test]
fn test_eval_depth_invalidpc() {
    let insts: Vec<Instruction> = vec![Instruction::Char('a'), Instruction::Char('b'), Instruction::Match];
    let chars:Vec<char> =vec!['a', 'b', 'c', 'd'];

    let actual = eval_depth(&insts, &chars, 18446744073709551615, 0);
    assert_eq!(actual, Err(EvalError::InvalidPC));
}
