//! Ast(v2) を命令列(InstructionV2)へコンパイルする。
#![allow(dead_code)]

use thiserror::Error;

use crate::engine::{ast::Ast, instruction_v2::InstructionV2, safe_add};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CompileV2Error {
    #[error("CompileV2Error: PCOverFlow")]
    PCOverFlow,
    #[error("CompileV2Error: InvalidBackreference({0})")]
    InvalidBackreference(usize),
}

#[derive(Default, Debug)]
struct CompilerV2 {
    p_counter: usize,
    instructions: Vec<InstructionV2>,
}

impl CompilerV2 {
    fn increment_p_counter(&mut self) -> Result<(), CompileV2Error> {
        safe_add(&mut self.p_counter, &1, || CompileV2Error::PCOverFlow)
    }

    fn next_address(&self) -> Result<usize, CompileV2Error> {
        self.p_counter
            .checked_add(1)
            .ok_or(CompileV2Error::PCOverFlow)
    }

    fn push_instruction(&mut self, instruction: InstructionV2) -> Result<usize, CompileV2Error> {
        let index = self.p_counter;
        self.increment_p_counter()?;
        self.instructions.push(instruction);
        Ok(index)
    }

    fn patch_split_right(
        &mut self,
        split_index: usize,
        target: usize,
    ) -> Result<(), CompileV2Error> {
        match self.instructions.get_mut(split_index) {
            Some(InstructionV2::Split(_, right)) => {
                *right = target;
                Ok(())
            }
            _ => Err(CompileV2Error::PCOverFlow),
        }
    }

    fn patch_split_left(
        &mut self,
        split_index: usize,
        target: usize,
    ) -> Result<(), CompileV2Error> {
        match self.instructions.get_mut(split_index) {
            Some(InstructionV2::Split(left, _)) => {
                *left = target;
                Ok(())
            }
            _ => Err(CompileV2Error::PCOverFlow),
        }
    }

    fn patch_jump(&mut self, jump_index: usize, target: usize) -> Result<(), CompileV2Error> {
        match self.instructions.get_mut(jump_index) {
            Some(InstructionV2::Jump(addr)) => {
                *addr = target;
                Ok(())
            }
            _ => Err(CompileV2Error::PCOverFlow),
        }
    }

    fn gen_expr(&mut self, ast: &Ast) -> Result<(), CompileV2Error> {
        match ast {
            Ast::Empty => Ok(()),
            Ast::CharClass(class) => {
                self.push_instruction(InstructionV2::CharClass(class.clone()))?;
                Ok(())
            }
            Ast::Assertion(predicate) => {
                self.push_instruction(InstructionV2::Assert(*predicate))?;
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
                self.push_instruction(InstructionV2::Backref(*index))?;
                Ok(())
            }
        }
    }

    fn gen_capture(&mut self, expr: &Ast, index: usize) -> Result<(), CompileV2Error> {
        self.push_instruction(InstructionV2::SaveStart(index))?;
        self.gen_expr(expr)?;
        self.push_instruction(InstructionV2::SaveEnd(index))?;
        Ok(())
    }

    fn gen_zero_or_more(&mut self, expr: &Ast, greedy: bool) -> Result<(), CompileV2Error> {
        let expr_entry = self.next_address()?;
        let split = if greedy {
            InstructionV2::Split(expr_entry, 0)
        } else {
            InstructionV2::Split(0, expr_entry)
        };
        let split_index = self.push_instruction(split)?;
        self.gen_expr(expr)?;
        self.push_instruction(InstructionV2::Jump(split_index))?;

        let out = self.p_counter;
        if greedy {
            self.patch_split_right(split_index, out)
        } else {
            self.patch_split_left(split_index, out)
        }
    }

    fn gen_one_or_more(&mut self, expr: &Ast, greedy: bool) -> Result<(), CompileV2Error> {
        let loop_entry = self.p_counter;
        self.gen_expr(expr)?;

        let out = self.next_address()?;
        if greedy {
            self.push_instruction(InstructionV2::Split(loop_entry, out))?;
        } else {
            self.push_instruction(InstructionV2::Split(out, loop_entry))?;
        }
        Ok(())
    }

    fn gen_zero_or_one(&mut self, expr: &Ast, greedy: bool) -> Result<(), CompileV2Error> {
        let expr_entry = self.next_address()?;
        let split = if greedy {
            InstructionV2::Split(expr_entry, 0)
        } else {
            InstructionV2::Split(0, expr_entry)
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

    fn gen_repeat(
        &mut self,
        expr: &Ast,
        greedy: bool,
        min: u32,
        max: Option<u32>,
    ) -> Result<(), CompileV2Error> {
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

    fn gen_concat(&mut self, exprs: &[Ast]) -> Result<(), CompileV2Error> {
        for expr in exprs {
            self.gen_expr(expr)?;
        }
        Ok(())
    }

    fn gen_alternate(&mut self, left: &Ast, right: &Ast) -> Result<(), CompileV2Error> {
        let left_entry = self.next_address()?;
        let split_index = self.push_instruction(InstructionV2::Split(left_entry, 0))?;

        self.gen_expr(left)?;
        let jump_index = self.push_instruction(InstructionV2::Jump(0))?;

        let right_entry = self.p_counter;
        self.patch_split_right(split_index, right_entry)?;
        self.gen_expr(right)?;

        let out = self.p_counter;
        self.patch_jump(jump_index, out)
    }

    fn finish(mut self) -> Result<Vec<InstructionV2>, CompileV2Error> {
        self.push_instruction(InstructionV2::Match)?;
        Ok(self.instructions)
    }
}

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

fn validate_backreferences(ast: &Ast, max_capture: usize) -> Result<(), CompileV2Error> {
    match ast {
        Ast::Backreference(index) => {
            if *index == 0 || *index > max_capture {
                Err(CompileV2Error::InvalidBackreference(*index))
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

pub fn compile_v2(ast: &Ast) -> Result<Vec<InstructionV2>, CompileV2Error> {
    let max_capture = max_capture_index(ast);
    validate_backreferences(ast, max_capture)?;

    let mut compiler = CompilerV2::default();
    compiler.gen_expr(ast)?;
    compiler.finish()
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        ast::{CharClass, CharRange, Predicate},
        compiler_v2::{CompileV2Error, compile_v2},
        instruction_v2::InstructionV2,
        parser_v2::parse,
    };

    fn literal(c: char) -> InstructionV2 {
        InstructionV2::CharClass(CharClass::new(vec![CharRange { start: c, end: c }], false))
    }

    #[test]
    fn test_compile_v2_literal() {
        let ast = parse("abc").unwrap();
        let actual = compile_v2(&ast).unwrap();
        let expect = vec![
            literal('a'),
            literal('b'),
            literal('c'),
            InstructionV2::Match,
        ];
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_compile_v2_alternate() {
        let ast = parse("a|b").unwrap();
        let actual = compile_v2(&ast).unwrap();
        let expect = vec![
            InstructionV2::Split(1, 3),
            literal('a'),
            InstructionV2::Jump(4),
            literal('b'),
            InstructionV2::Match,
        ];
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_compile_v2_star() {
        let ast = parse("a*").unwrap();
        let actual = compile_v2(&ast).unwrap();
        let expect = vec![
            InstructionV2::Split(1, 3),
            literal('a'),
            InstructionV2::Jump(0),
            InstructionV2::Match,
        ];
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_compile_v2_repeat() {
        let ast = parse("a{2,3}").unwrap();
        let actual = compile_v2(&ast).unwrap();
        let expect = vec![
            literal('a'),
            literal('a'),
            InstructionV2::Split(3, 4),
            literal('a'),
            InstructionV2::Match,
        ];
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_compile_v2_assert_and_backref() {
        let ast = parse("^(abc)\\1$").unwrap();
        let actual = compile_v2(&ast).unwrap();
        let expect = vec![
            InstructionV2::Assert(Predicate::StartOfLine),
            InstructionV2::SaveStart(1),
            literal('a'),
            literal('b'),
            literal('c'),
            InstructionV2::SaveEnd(1),
            InstructionV2::Backref(1),
            InstructionV2::Assert(Predicate::EndOfLine),
            InstructionV2::Match,
        ];
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_compile_v2_invalid_backreference() {
        let ast = parse("(a)\\2").unwrap();
        let actual = compile_v2(&ast);
        assert_eq!(actual, Err(CompileV2Error::InvalidBackreference(2)));
    }
}
