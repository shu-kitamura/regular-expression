//! v2 命令列を評価する。
#![allow(dead_code)]

use std::collections::HashSet;

use thiserror::Error;

use crate::engine::{
    ast::{CharClass, Predicate},
    instruction_v2::InstructionV2,
    safe_add,
};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EvalV2Error {
    #[error("EvalV2Error: PCOverFlow")]
    PCOverFlow,
    #[error("EvalV2Error: CharIndexOverFlow")]
    CharIndexOverFlow,
    #[error("EvalV2Error: InvalidPC")]
    InvalidPC,
}

#[derive(Debug, Clone)]
struct State {
    pc: usize,
    char_index: usize,
    capture_start: Vec<Option<usize>>,
    capture_end: Vec<Option<usize>>,
}

impl State {
    fn new(start: usize, capture_slots: usize) -> Self {
        Self {
            pc: 0,
            char_index: start,
            capture_start: vec![None; capture_slots],
            capture_end: vec![None; capture_slots],
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct StateKey {
    pc: usize,
    char_index: usize,
    capture_start: Vec<Option<usize>>,
    capture_end: Vec<Option<usize>>,
}

impl StateKey {
    fn from_state(state: &State) -> Self {
        Self {
            pc: state.pc,
            char_index: state.char_index,
            capture_start: state.capture_start.clone(),
            capture_end: state.capture_end.clone(),
        }
    }
}

fn increment_pc(pc: &mut usize) -> Result<(), EvalV2Error> {
    safe_add(pc, &1, || EvalV2Error::PCOverFlow)
}

fn increment_char_index(char_index: &mut usize, size: usize) -> Result<(), EvalV2Error> {
    safe_add(char_index, &size, || EvalV2Error::CharIndexOverFlow)
}

fn eval_char_class(class: &CharClass, current: Option<char>) -> bool {
    let Some(current_char) = current else {
        return false;
    };

    let is_in_range = class
        .ranges
        .iter()
        .any(|range| range.start <= current_char && current_char <= range.end);

    if class.negated {
        !is_in_range
    } else {
        is_in_range
    }
}

fn eval_assert(predicate: Predicate, chars: &[char], char_index: usize) -> bool {
    if char_index > chars.len() {
        return false;
    }

    match predicate {
        Predicate::StartOfLine => {
            char_index == 0 || chars.get(char_index.saturating_sub(1)) == Some(&'\n')
        }
        Predicate::EndOfLine => char_index == chars.len() || chars.get(char_index) == Some(&'\n'),
        Predicate::StartOfText => char_index == 0,
        Predicate::EndOfText => char_index == chars.len(),
        Predicate::WordBoundary => is_word_boundary(chars, char_index),
        Predicate::NonWordBoundary => !is_word_boundary(chars, char_index),
    }
}

fn is_word_boundary(chars: &[char], char_index: usize) -> bool {
    let prev = if char_index == 0 {
        None
    } else {
        chars.get(char_index - 1).copied()
    };
    let curr = chars.get(char_index).copied();

    let is_prev_word = prev.map(is_word_char).unwrap_or(false);
    let is_curr_word = curr.map(is_word_char).unwrap_or(false);

    is_prev_word != is_curr_word
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn eval_backref(index: usize, state: &mut State, chars: &[char]) -> Result<bool, EvalV2Error> {
    let start = match state.capture_start.get(index).and_then(|value| *value) {
        Some(start) => start,
        None => return Ok(false),
    };
    let end = match state.capture_end.get(index).and_then(|value| *value) {
        Some(end) => end,
        None => return Ok(false),
    };

    if end < start || end > chars.len() || state.char_index > chars.len() {
        return Ok(false);
    }

    let capture_len = end - start;
    if chars.len() - state.char_index < capture_len {
        return Ok(false);
    }

    for i in 0..capture_len {
        if chars[start + i] != chars[state.char_index + i] {
            return Ok(false);
        }
    }

    increment_pc(&mut state.pc)?;
    increment_char_index(&mut state.char_index, capture_len)?;
    Ok(true)
}

fn max_capture_index(inst: &[InstructionV2]) -> usize {
    let mut max_index = 0;
    for instruction in inst {
        match instruction {
            InstructionV2::SaveStart(index)
            | InstructionV2::SaveEnd(index)
            | InstructionV2::Backref(index) => {
                max_index = max_index.max(*index);
            }
            _ => {}
        }
    }
    max_index
}

fn eval_from_start_v2(
    inst: &[InstructionV2],
    chars: &[char],
    start: usize,
    capture_slots: usize,
) -> Result<bool, EvalV2Error> {
    let mut stack = vec![State::new(start, capture_slots)];
    let mut visited = HashSet::new();

    while let Some(mut state) = stack.pop() {
        loop {
            let key = StateKey::from_state(&state);
            if !visited.insert(key) {
                break;
            }

            let instruction = match inst.get(state.pc) {
                Some(instruction) => instruction,
                None => return Err(EvalV2Error::InvalidPC),
            };

            match instruction {
                InstructionV2::CharClass(class) => {
                    if !eval_char_class(class, chars.get(state.char_index).copied()) {
                        break;
                    }
                    increment_pc(&mut state.pc)?;
                    increment_char_index(&mut state.char_index, 1)?;
                }
                InstructionV2::Assert(predicate) => {
                    if !eval_assert(*predicate, chars, state.char_index) {
                        break;
                    }
                    increment_pc(&mut state.pc)?;
                }
                InstructionV2::SaveStart(index) => {
                    if let Some(slot) = state.capture_start.get_mut(*index) {
                        *slot = Some(state.char_index);
                    } else {
                        break;
                    }
                    increment_pc(&mut state.pc)?;
                }
                InstructionV2::SaveEnd(index) => {
                    if let Some(slot) = state.capture_end.get_mut(*index) {
                        *slot = Some(state.char_index);
                    } else {
                        break;
                    }
                    increment_pc(&mut state.pc)?;
                }
                InstructionV2::Backref(index) => {
                    if !eval_backref(*index, &mut state, chars)? {
                        break;
                    }
                }
                InstructionV2::Split(left, right) => {
                    let mut right_state = state.clone();
                    right_state.pc = *right;
                    stack.push(right_state);
                    state.pc = *left;
                }
                InstructionV2::Jump(addr) => state.pc = *addr,
                InstructionV2::Match => return Ok(true),
            }
        }
    }

    Ok(false)
}

pub fn eval_v2(inst: &[InstructionV2], input: &str) -> Result<bool, EvalV2Error> {
    let chars: Vec<char> = input.chars().collect();
    let capture_slots = max_capture_index(inst)
        .checked_add(1)
        .ok_or(EvalV2Error::PCOverFlow)?;

    for start in 0..=chars.len() {
        if eval_from_start_v2(inst, &chars, start, capture_slots)? {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        ast::{CharClass, CharRange, Predicate},
        compiler_v2::compile_v2,
        evaluator_v2::{EvalV2Error, eval_v2},
        instruction_v2::InstructionV2,
        parser_v2::parse,
    };

    fn literal(c: char) -> InstructionV2 {
        InstructionV2::CharClass(CharClass::new(vec![CharRange { start: c, end: c }], false))
    }

    #[test]
    fn test_eval_v2_backreference_match_and_mismatch() {
        let ast = parse("(abc)\\1").unwrap();
        let inst = compile_v2(&ast).unwrap();

        assert!(eval_v2(&inst, "abcabc").unwrap());
        assert!(!eval_v2(&inst, "abcabd").unwrap());
    }

    #[test]
    fn test_eval_v2_unresolved_backreference() {
        let ast = parse("(a)?\\1").unwrap();
        let inst = compile_v2(&ast).unwrap();

        assert!(!eval_v2(&inst, "a").unwrap());
        assert!(!eval_v2(&inst, "").unwrap());
        assert!(eval_v2(&inst, "aa").unwrap());
    }

    #[test]
    fn test_eval_v2_negated_class() {
        let ast = parse("d[^io]g").unwrap();
        let inst = compile_v2(&ast).unwrap();

        assert!(eval_v2(&inst, "dag").unwrap());
        assert!(!eval_v2(&inst, "dig").unwrap());
        assert!(!eval_v2(&inst, "dog").unwrap());
    }

    #[test]
    fn test_eval_v2_anchors() {
        let ast = parse("^abc$").unwrap();
        let inst = compile_v2(&ast).unwrap();
        assert!(eval_v2(&inst, "abc").unwrap());
        assert!(!eval_v2(&inst, "xabc").unwrap());
        assert!(!eval_v2(&inst, "abcx").unwrap());

        let ast_empty = parse("^$").unwrap();
        let inst_empty = compile_v2(&ast_empty).unwrap();
        assert!(eval_v2(&inst_empty, "").unwrap());
        assert!(!eval_v2(&inst_empty, "a").unwrap());
    }

    #[test]
    fn test_eval_v2_word_boundary_predicate() {
        let inst = vec![
            InstructionV2::Assert(Predicate::WordBoundary),
            literal('a'),
            InstructionV2::Match,
        ];
        assert!(eval_v2(&inst, "a").unwrap());
        assert!(!eval_v2(&inst, "_a").unwrap());
    }

    #[test]
    fn test_eval_v2_invalid_pc() {
        let inst = vec![InstructionV2::Jump(10)];
        let actual = eval_v2(&inst, "abc");
        assert_eq!(actual, Err(EvalV2Error::InvalidPC));
    }
}
