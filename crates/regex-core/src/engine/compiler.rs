//! Compile an AST into an instruction sequence (`Instruction`).

use thiserror::Error;

use crate::engine::{ast::Ast, instruction::Instruction, safe_add};

/// Errors returned while compiling AST nodes into instructions.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CompileError {
    /// Program counter overflow while building the instruction stream.
    #[error("CompileError: PCOverFlow")]
    PCOverFlow,
    /// A backreference points to a capture group that does not exist.
    #[error("CompileError: InvalidBackreference({0})")]
    InvalidBackreference(usize),
}

/// Stateful Thompson-style compiler.
///
/// `p_counter` tracks the next instruction address.
/// `instructions` stores emitted bytecode-like instructions.
#[derive(Default, Debug)]
struct Compiler {
    p_counter: usize,
    instructions: Vec<Instruction>,
}

impl Compiler {
    /// Increments the program counter by one with overflow checks.
    fn increment_p_counter(&mut self) -> Result<(), CompileError> {
        safe_add(&mut self.p_counter, &1, || CompileError::PCOverFlow)
    }

    /// Returns the address of the next instruction slot.
    fn next_address(&self) -> Result<usize, CompileError> {
        self.p_counter
            .checked_add(1)
            .ok_or(CompileError::PCOverFlow)
    }

    /// Appends one instruction and returns its address.
    fn push_instruction(&mut self, instruction: Instruction) -> Result<usize, CompileError> {
        let index = self.p_counter;
        self.increment_p_counter()?;
        self.instructions.push(instruction);
        Ok(index)
    }

    /// Patches the right branch target of a previously emitted `Split`.
    fn patch_split_right(&mut self, split_index: usize, target: usize) -> Result<(), CompileError> {
        match self.instructions.get_mut(split_index) {
            Some(Instruction::Split(_, right)) => {
                *right = target;
                Ok(())
            }
            _ => Err(CompileError::PCOverFlow),
        }
    }

    /// Patches the left branch target of a previously emitted `Split`.
    fn patch_split_left(&mut self, split_index: usize, target: usize) -> Result<(), CompileError> {
        match self.instructions.get_mut(split_index) {
            Some(Instruction::Split(left, _)) => {
                *left = target;
                Ok(())
            }
            _ => Err(CompileError::PCOverFlow),
        }
    }

    /// Patches the jump target of a previously emitted `Jump`.
    fn patch_jump(&mut self, jump_index: usize, target: usize) -> Result<(), CompileError> {
        match self.instructions.get_mut(jump_index) {
            Some(Instruction::Jump(addr)) => {
                *addr = target;
                Ok(())
            }
            _ => Err(CompileError::PCOverFlow),
        }
    }

    /// Emits instructions for one AST node.
    fn gen_expr(&mut self, ast: &Ast) -> Result<(), CompileError> {
        match ast {
            Ast::Empty => Ok(()),
            Ast::CharClass(class) => {
                self.push_instruction(Instruction::CharClass(class.clone()))?;
                Ok(())
            }
            Ast::Assertion(predicate) => {
                self.push_instruction(Instruction::Assert(*predicate))?;
                Ok(())
            }
            Ast::Capture { expr, index } => self.gen_capture(expr, *index),
            Ast::ZeroOrMore { expr, greedy } => self.gen_zero_or_more(expr, *greedy),
            Ast::OneOrMore { expr, greedy } => self.gen_one_or_more(expr, *greedy),
            Ast::ZeroOrOne { expr, greedy } => self.gen_zero_or_one(expr, *greedy),
            Ast::Repeat {
                expr,
                greedy,
                min,
                max,
            } => self.gen_repeat(expr, *greedy, *min, *max),
            Ast::Concat(exprs) => self.gen_concat(exprs),
            Ast::Alternate(left, right) => self.gen_alternate(left, right),
            Ast::Backreference(index) => {
                self.push_instruction(Instruction::Backref(*index))?;
                Ok(())
            }
        }
    }

    /// Emits capture boundary instructions around the nested expression.
    fn gen_capture(&mut self, expr: &Ast, index: usize) -> Result<(), CompileError> {
        self.push_instruction(Instruction::SaveStart(index))?;
        self.gen_expr(expr)?;
        self.push_instruction(Instruction::SaveEnd(index))?;
        Ok(())
    }

    /// Emits a `*` quantifier as a loop with `Split` and `Jump`.
    fn gen_zero_or_more(&mut self, expr: &Ast, greedy: bool) -> Result<(), CompileError> {
        let expr_entry = self.next_address()?;
        let split = if greedy {
            Instruction::Split(expr_entry, 0)
        } else {
            Instruction::Split(0, expr_entry)
        };
        let split_index = self.push_instruction(split)?;
        self.gen_expr(expr)?;
        self.push_instruction(Instruction::Jump(split_index))?;

        let out = self.p_counter;
        if greedy {
            self.patch_split_right(split_index, out)
        } else {
            self.patch_split_left(split_index, out)
        }
    }

    /// Emits a `+` quantifier as one mandatory match plus a loop.
    fn gen_one_or_more(&mut self, expr: &Ast, greedy: bool) -> Result<(), CompileError> {
        let loop_entry = self.p_counter;
        self.gen_expr(expr)?;

        let out = self.next_address()?;
        if greedy {
            self.push_instruction(Instruction::Split(loop_entry, out))?;
        } else {
            self.push_instruction(Instruction::Split(out, loop_entry))?;
        }
        Ok(())
    }

    /// Emits a `?` quantifier as one conditional branch.
    fn gen_zero_or_one(&mut self, expr: &Ast, greedy: bool) -> Result<(), CompileError> {
        let expr_entry = self.next_address()?;
        let split = if greedy {
            Instruction::Split(expr_entry, 0)
        } else {
            Instruction::Split(0, expr_entry)
        };
        let split_index = self.push_instruction(split)?;
        self.gen_expr(expr)?;

        let out = self.p_counter;
        if greedy {
            self.patch_split_right(split_index, out)
        } else {
            self.patch_split_left(split_index, out)
        }
    }

    /// Emits bounded or unbounded repetition (`{m}`, `{m,n}`, `{m,}`).
    fn gen_repeat(
        &mut self,
        expr: &Ast,
        greedy: bool,
        min: u32,
        max: Option<u32>,
    ) -> Result<(), CompileError> {
        for _ in 0..min {
            self.gen_expr(expr)?;
        }

        match max {
            Some(max_count) => {
                if max_count <= min {
                    return Ok(());
                }
                for _ in min..max_count {
                    self.gen_zero_or_one(expr, greedy)?;
                }
                Ok(())
            }
            None => self.gen_zero_or_more(expr, greedy),
        }
    }

    /// Emits concatenated expressions in order.
    fn gen_concat(&mut self, exprs: &[Ast]) -> Result<(), CompileError> {
        for expr in exprs {
            self.gen_expr(expr)?;
        }
        Ok(())
    }

    /// Emits alternation using one `Split` and one trailing `Jump`.
    fn gen_alternate(&mut self, left: &Ast, right: &Ast) -> Result<(), CompileError> {
        let left_entry = self.next_address()?;
        let split_index = self.push_instruction(Instruction::Split(left_entry, 0))?;

        self.gen_expr(left)?;
        let jump_index = self.push_instruction(Instruction::Jump(0))?;

        let right_entry = self.p_counter;
        self.patch_split_right(split_index, right_entry)?;
        self.gen_expr(right)?;

        let out = self.p_counter;
        self.patch_jump(jump_index, out)
    }

    /// Finalizes the compiled program by appending `Match`.
    fn finish(mut self) -> Result<Vec<Instruction>, CompileError> {
        self.push_instruction(Instruction::Match)?;
        Ok(self.instructions)
    }
}

/// Returns the maximum capture index used in the AST.
fn max_capture_index(ast: &Ast) -> usize {
    match ast {
        Ast::Capture { expr, index } => (*index).max(max_capture_index(expr)),
        Ast::ZeroOrMore { expr, .. }
        | Ast::OneOrMore { expr, .. }
        | Ast::ZeroOrOne { expr, .. }
        | Ast::Repeat { expr, .. } => max_capture_index(expr),
        Ast::Concat(exprs) => exprs.iter().map(max_capture_index).max().unwrap_or(0),
        Ast::Alternate(left, right) => max_capture_index(left).max(max_capture_index(right)),
        _ => 0,
    }
}

/// Validates that every backreference points to an existing capture.
fn validate_backreferences(ast: &Ast, max_capture: usize) -> Result<(), CompileError> {
    match ast {
        Ast::Backreference(index) => {
            if *index == 0 || *index > max_capture {
                Err(CompileError::InvalidBackreference(*index))
            } else {
                Ok(())
            }
        }
        Ast::Capture { expr, .. }
        | Ast::ZeroOrMore { expr, .. }
        | Ast::OneOrMore { expr, .. }
        | Ast::ZeroOrOne { expr, .. }
        | Ast::Repeat { expr, .. } => validate_backreferences(expr, max_capture),
        Ast::Concat(exprs) => {
            for expr in exprs {
                validate_backreferences(expr, max_capture)?;
            }
            Ok(())
        }
        Ast::Alternate(left, right) => {
            validate_backreferences(left, max_capture)?;
            validate_backreferences(right, max_capture)
        }
        _ => Ok(()),
    }
}

/// Compiles an AST into an executable instruction sequence.
pub fn compile(ast: &Ast) -> Result<Vec<Instruction>, CompileError> {
    let max_capture = max_capture_index(ast);
    validate_backreferences(ast, max_capture)?;

    let mut compiler = Compiler::default();
    compiler.gen_expr(ast)?;
    compiler.finish()
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        ast::{CharClass, CharRange, Predicate},
        compiler::{CompileError, compile},
        instruction::Instruction,
        parser::parse,
    };

    fn literal(c: char) -> Instruction {
        Instruction::CharClass(CharClass::new(vec![CharRange { start: c, end: c }], false))
    }

    #[test]
    fn test_compile_literal() {
        let ast = parse("abc").unwrap();
        let actual = compile(&ast).unwrap();
        let expect = vec![literal('a'), literal('b'), literal('c'), Instruction::Match];
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_compile_alternate() {
        let ast = parse("a|b").unwrap();
        let actual = compile(&ast).unwrap();
        let expect = vec![
            Instruction::Split(1, 3),
            literal('a'),
            Instruction::Jump(4),
            literal('b'),
            Instruction::Match,
        ];
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_compile_star() {
        let ast = parse("a*").unwrap();
        let actual = compile(&ast).unwrap();
        let expect = vec![
            Instruction::Split(1, 3),
            literal('a'),
            Instruction::Jump(0),
            Instruction::Match,
        ];
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_compile_repeat() {
        let ast = parse("a{2,3}").unwrap();
        let actual = compile(&ast).unwrap();
        let expect = vec![
            literal('a'),
            literal('a'),
            Instruction::Split(3, 4),
            literal('a'),
            Instruction::Match,
        ];
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_compile_assert_and_backref() {
        let ast = parse("^(abc)\\1$").unwrap();
        let actual = compile(&ast).unwrap();
        let expect = vec![
            Instruction::Assert(Predicate::StartOfLine),
            Instruction::SaveStart(1),
            literal('a'),
            literal('b'),
            literal('c'),
            Instruction::SaveEnd(1),
            Instruction::Backref(1),
            Instruction::Assert(Predicate::EndOfLine),
            Instruction::Match,
        ];
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_compile_invalid_backreference() {
        let ast = parse("(a)\\2").unwrap();
        let actual = compile(&ast);
        assert_eq!(actual, Err(CompileError::InvalidBackreference(2)));
    }
}
