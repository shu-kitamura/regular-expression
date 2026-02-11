//! Evaluate an instruction sequence.

use std::collections::HashSet;

use thiserror::Error;

use crate::engine::{
    ast::{CharClass, Predicate},
    instruction::Instruction,
    safe_add,
};

/// Errors returned while evaluating instructions.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EvalError {
    /// Program counter overflow.
    #[error("EvalError: PCOverFlow")]
    PCOverFlow,
    /// Character index overflow.
    #[error("EvalError: CharIndexOverFlow")]
    CharIndexOverFlow,
    /// Instruction pointer points outside the instruction array.
    #[error("EvalError: InvalidPC")]
    InvalidPC,
}

/// Runtime state for one NFA execution branch.
#[derive(Debug, Clone)]
struct State {
    pc: usize,
    char_index: usize,
    capture_start: Vec<Option<usize>>,
    capture_end: Vec<Option<usize>>,
}

impl State {
    /// Creates a new state at `start` with preallocated capture slots.
    fn new(start: usize, capture_slots: usize) -> Self {
        Self {
            pc: 0,
            char_index: start,
            capture_start: vec![None; capture_slots],
            capture_end: vec![None; capture_slots],
        }
    }
}

/// Hashable state identity used to detect revisits and prevent infinite loops.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct StateKey {
    pc: usize,
    char_index: usize,
    capture_start: Vec<Option<usize>>,
    capture_end: Vec<Option<usize>>,
}

impl StateKey {
    /// Builds a deduplication key from the current state.
    fn from_state(state: &State) -> Self {
        Self {
            pc: state.pc,
            char_index: state.char_index,
            capture_start: state.capture_start.clone(),
            capture_end: state.capture_end.clone(),
        }
    }
}

/// Increments the program counter with overflow checks.
fn increment_pc(pc: &mut usize) -> Result<(), EvalError> {
    safe_add(pc, &1, || EvalError::PCOverFlow)
}

/// Advances the current character index by `size` with overflow checks.
fn increment_char_index(char_index: &mut usize, size: usize) -> Result<(), EvalError> {
    safe_add(char_index, &size, || EvalError::CharIndexOverFlow)
}

/// Evaluates one character-class instruction against the current character.
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

/// Evaluates one zero-width assertion at the current position.
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

/// Returns whether the current boundary is between word and non-word characters.
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

/// Defines word characters for `WordBoundary`.
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Evaluates a backreference by comparing against the captured slice.
fn eval_backref(index: usize, state: &mut State, chars: &[char]) -> Result<bool, EvalError> {
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

/// Returns the largest capture index referenced by instructions.
fn max_capture_index(inst: &[Instruction]) -> usize {
    let mut max_index = 0;
    for instruction in inst {
        match instruction {
            Instruction::SaveStart(index)
            | Instruction::SaveEnd(index)
            | Instruction::Backref(index) => {
                max_index = max_index.max(*index);
            }
            _ => {}
        }
    }
    max_index
}

/// Runs the NFA from a fixed starting character index.
fn eval_from_start_inner(
    inst: &[Instruction],
    chars: &[char],
    start: usize,
    capture_slots: usize,
) -> Result<bool, EvalError> {
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
                None => return Err(EvalError::InvalidPC),
            };

            match instruction {
                Instruction::CharClass(class) => {
                    if !eval_char_class(class, chars.get(state.char_index).copied()) {
                        break;
                    }
                    increment_pc(&mut state.pc)?;
                    increment_char_index(&mut state.char_index, 1)?;
                }
                Instruction::Assert(predicate) => {
                    if !eval_assert(*predicate, chars, state.char_index) {
                        break;
                    }
                    increment_pc(&mut state.pc)?;
                }
                Instruction::SaveStart(index) => {
                    if let Some(slot) = state.capture_start.get_mut(*index) {
                        *slot = Some(state.char_index);
                    } else {
                        break;
                    }
                    increment_pc(&mut state.pc)?;
                }
                Instruction::SaveEnd(index) => {
                    if let Some(slot) = state.capture_end.get_mut(*index) {
                        *slot = Some(state.char_index);
                    } else {
                        break;
                    }
                    increment_pc(&mut state.pc)?;
                }
                Instruction::Backref(index) => {
                    if !eval_backref(*index, &mut state, chars)? {
                        break;
                    }
                }
                Instruction::Split(left, right) => {
                    let mut right_state = state.clone();
                    right_state.pc = *right;
                    stack.push(right_state);
                    state.pc = *left;
                }
                Instruction::Jump(addr) => state.pc = *addr,
                Instruction::Match => return Ok(true),
            }
        }
    }

    Ok(false)
}

/// Evaluates whether `input` matches from the first character.
pub fn eval_from_start(inst: &[Instruction], input: &str) -> Result<bool, EvalError> {
    let chars: Vec<char> = input.chars().collect();
    let capture_slots = max_capture_index(inst)
        .checked_add(1)
        .ok_or(EvalError::PCOverFlow)?;
    eval_from_start_inner(inst, &chars, 0, capture_slots)
}

/// Evaluates whether `input` matches at any starting position.
pub fn eval(inst: &[Instruction], input: &str) -> Result<bool, EvalError> {
    let chars: Vec<char> = input.chars().collect();
    let capture_slots = max_capture_index(inst)
        .checked_add(1)
        .ok_or(EvalError::PCOverFlow)?;

    for start in 0..=chars.len() {
        if eval_from_start_inner(inst, &chars, start, capture_slots)? {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        ast::{CharClass, CharRange, Predicate},
        compiler::compile,
        evaluator::{EvalError, eval, eval_from_start},
        instruction::Instruction,
        parser::parse,
    };

    fn literal(c: char) -> Instruction {
        Instruction::CharClass(CharClass::new(vec![CharRange { start: c, end: c }], false))
    }

    #[test]
    fn test_eval_backreference_match_and_mismatch() {
        let ast = parse("(abc)\\1").unwrap();
        let inst = compile(&ast).unwrap();

        assert!(eval(&inst, "abcabc").unwrap());
        assert!(!eval(&inst, "abcabd").unwrap());
    }

    #[test]
    fn test_eval_unresolved_backreference() {
        let ast = parse("(a)?\\1").unwrap();
        let inst = compile(&ast).unwrap();

        assert!(!eval(&inst, "a").unwrap());
        assert!(!eval(&inst, "").unwrap());
        assert!(eval(&inst, "aa").unwrap());
    }

    #[test]
    fn test_eval_negated_class() {
        let ast = parse("d[^io]g").unwrap();
        let inst = compile(&ast).unwrap();

        assert!(eval(&inst, "dag").unwrap());
        assert!(!eval(&inst, "dig").unwrap());
        assert!(!eval(&inst, "dog").unwrap());
    }

    #[test]
    fn test_eval_anchors() {
        let ast = parse("^abc$").unwrap();
        let inst = compile(&ast).unwrap();
        assert!(eval(&inst, "abc").unwrap());
        assert!(!eval(&inst, "xabc").unwrap());
        assert!(!eval(&inst, "abcx").unwrap());

        let ast_empty = parse("^$").unwrap();
        let inst_empty = compile(&ast_empty).unwrap();
        assert!(eval(&inst_empty, "").unwrap());
        assert!(!eval(&inst_empty, "a").unwrap());
    }

    #[test]
    fn test_eval_word_boundary_predicate() {
        let inst = vec![
            Instruction::Assert(Predicate::WordBoundary),
            literal('a'),
            Instruction::Match,
        ];
        assert!(eval(&inst, "a").unwrap());
        assert!(!eval(&inst, "_a").unwrap());
    }

    #[test]
    fn test_eval_invalid_pc() {
        let inst = vec![Instruction::Jump(10)];
        let actual = eval(&inst, "abc");
        assert_eq!(actual, Err(EvalError::InvalidPC));
    }

    #[test]
    fn test_eval_from_start() {
        let ast = parse("abc").unwrap();
        let inst = compile(&ast).unwrap();
        assert!(eval_from_start(&inst, "abcxxx").unwrap());
        assert!(!eval_from_start(&inst, "xabc").unwrap());
    }
}
