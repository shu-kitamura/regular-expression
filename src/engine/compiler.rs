//! AST を命令列(Instruction)にコンパイルするための型・関数  
//! "ab(c|b)" が入力された場合、以下にコンパイルする
//! (左の数字はプログラムカウンタ)
//! 
//! ```text
//! 0 : Char(a)
//! 1 : Char(b)
//! 2 : Split 3, 5
//! 3 : Char(c)
//! 4 : Jump 6
//! 5 : Char(d)
//! 6 : Match
//! ```

use crate::{
    error::CompileError,
    engine::{
        safe_add,
        instruction::{Instruction, Char},
        parser::AST,
    }
};

/// コンパイラの型
#[derive(Default, Debug)]
struct Compiler {
    p_counter: usize,
    instructions: Vec<Instruction>
}

impl Compiler {
    /// p_counter をインクリメントさせる
    fn increment_p_counter(&mut self) -> Result<(), CompileError> {
        safe_add(&mut self.p_counter, &1, || CompileError::PCOverFlow)
    }

    /// 入力された AST の型に応じた関数を実行する
    fn gen_expr(&mut self, ast: &AST) -> Result<(), CompileError> {
        match ast {
            AST::AnyChar => self.gen_anychar(),
            AST::Char(c) => self.gen_char(*c),
            AST::Or(e1, e2) => self.gen_or(e1, e2),
            AST::Plus(ast) => match &**ast {
                AST::Star(child_ast) => self.gen_expr(&child_ast),
                AST::Seq(child_vec) if child_vec.len() == 1 =>
                    if let Some(child_ast @ AST::Star(_)) = child_vec.get(0) {
                        self.gen_expr(&child_ast)
                    } else {
                        self.gen_expr(ast)
                    }
                AST::Or(child_ast1, child_ast2 ) => {
                    match self.gen_expr(&child_ast1){
                        Ok(()) => {},
                        Err(e) => return Err(e),
                    };
                    self.gen_expr(&child_ast2)
                }
                e => self.gen_plus(e)
            },
            AST::Star(ast) => match &**ast {
                AST::Star(child_ast) => self.gen_expr(&child_ast),
                AST::Seq(child_vec) if child_vec.len() == 1 =>
                    if let Some(child_ast @ AST::Star(_)) = child_vec.get(0) {
                        self.gen_expr(&child_ast)
                    } else {
                        self.gen_expr(ast)
                    }
                AST::Or(child_ast1, child_ast2 ) => {
                    match self.gen_expr(&child_ast1){
                        Ok(()) => {},
                        Err(e) => return Err(e),
                    };
                    self.gen_expr(&child_ast2)
                }    
                e => self.gen_star(e)
            },
            AST::Question(ast) => self.gen_question(ast),
            AST::Seq(v) => self.gen_seq(v),
        }
    }

    /// AST::Char 型に対応する Instruction を生成し、instructions に push する
    fn gen_char(&mut self, c: char) -> Result<(), CompileError> {
        let inst: Instruction = Instruction::Char(Char::Literal(c));
        self.increment_p_counter()?;
        self.instructions.push(inst);
        Ok(())
    }

    /// AST::AnyChar 型に対応する Instruction を生成し、instractions に push する
    fn gen_anychar(&mut self) -> Result<(), CompileError> {
        let inst: Instruction = Instruction::Char(Char::Any);
        self.increment_p_counter()?;
        self.instructions.push(inst);
        Ok(())
    }

    /// AST::Star 型に対応する Instruction を生成し、instructions に push する  
    /// a* 入力された場合、以下のような Instruction を生成する  
    /// 
    /// ```text
    /// 0 : split 1, 3
    /// 1 : Char(a)
    /// 2 : jump 0 
    /// 3 : ... 続き
    /// ```
    fn gen_star(&mut self, ast: &AST) -> Result<(), CompileError> {
        // 後で使うので、現在のポインタを変数に格納する
        let split_count: usize = self.p_counter;

        self.increment_p_counter()?;
        // split を挿入する。第二引数には expr2 のコードの開始のカウンタを指定する必要があるが、
        // この時点ではまだ値がわからないので、ここでは仮の数値(0)を入れて、数値は後で更新する。
        self.instructions.push(Instruction::Split(self.p_counter, 0));

        // AST を再帰的に処理する
        self.gen_expr(ast)?;

        // カウンタをインクリメントし、Jump を挿入する
        self.increment_p_counter()?;
        self.instructions.push(Instruction::Jump(split_count));
        
        // Split の第二引数を更新する
        if let Some(Instruction::Split(_, right)) = self.instructions.get_mut(split_count) {
            *right = self.p_counter;
            Ok(())
        } else {
            Err(CompileError::FailStar)
        }
    }

    /// AST::Plus 型に対応する Instruction を生成し、instructions に push する  
    /// a+ 入力された場合、以下のような Instruction を生成する  
    /// 
    /// ```text
    /// 0 : Char(a)
    /// 1 : split 0, 2
    /// 2 : ... 続き
    /// ```
    fn gen_plus(&mut self, ast: &AST) -> Result<(), CompileError> {
        let left: usize = self.p_counter;
        // AST を再帰的に処理する
        self.gen_expr(ast)?;

        // カウンタをインクリメントし Split を挿入する
        self.increment_p_counter()?;
        self.instructions.push(Instruction::Split(left, self.p_counter));
        Ok(())
    }

    /// AST::Question 型に対応する Instruction を生成し、instructions に push する  
    /// a? 入力された場合、以下のような Instruction を生成する  
    /// 
    /// ```text
    /// 0 : split 1, 2
    /// 1 : Char(a)
    /// 2 : ... 続き
    /// ```
    fn gen_question(&mut self, ast: &AST) -> Result<(), CompileError> {
        // 後で使うので、現在のポインタを変数に格納する。
        let split_count: usize = self.p_counter;

        self.increment_p_counter()?;

        // split を挿入する。第二引数には expr2 のコードの開始のカウンタを指定する必要があるが、
        // この時点ではまだ値がわからないので、ここでは仮の数値(0)を入れて、数値は後で更新する。
        self.instructions.push(Instruction::Split(self.p_counter, 0));
        
        // AST を再帰的に処理する。
        self.gen_expr(ast)?;

        // Split の第二引数を更新する。
        if let Some(Instruction::Split(_, right)) = self.instructions.get_mut(split_count) {
            *right = self.p_counter;
            Ok(())
        } else {
            Err(CompileError::FailQuestion)
        }
    }

    /// AST::Or 型に対応する Instruction を生成し、instructions に push する  
    /// a|b が入力された場合、以下のような Instruction を生成する。  
    /// 
    /// ```text
    /// 0 : split 1, 3
    /// 1 : Char(a)
    /// 2 : jump 4 
    /// 3 : Char(b)
    /// 4 : ... 続き
    /// ```
    fn gen_or(&mut self, expr1: &AST, expr2: &AST) -> Result<(), CompileError> {
        // 後で使うので、現在のポインタを変数に格納する。
        let split_counter: usize = self.p_counter;

        self.increment_p_counter()?;

        // split を挿入する。第二引数には expr2 のコードの開始のカウンタを指定する必要があるが、
        // この時点ではまだ値がわからないので、ここでは仮の数値(0)を入れて、数値は後で更新する。
        self.instructions.push(Instruction::Split(self.p_counter, 0));

        // 1つ目の AST を再帰的に処理する。
        self.gen_expr(expr1)?;

        let jump_counter: usize = self.p_counter;

        self.increment_p_counter()?;

        // Jump を挿入する。引数には expr2 のコードの次のカウンタを指定する必要があるが、
        // この時点ではまだ値がわからないので、ここでは仮の数値(0)を入れて、数値は後で更新する。
        self.instructions.push(Instruction::Jump(0));

        // Splitの第二引数を更新する。
        if let Some(Instruction::Split(_, right)) = self.instructions.get_mut(split_counter) {
            *right = self.p_counter;
        } else {
            return Err(CompileError::FailOr);
        }

        // 2つ目の AST を再帰的に処理する。
        self.gen_expr(expr2)?;

        // Jump の引数を更新する。
        if let Some(Instruction::Jump(arg)) = self.instructions.get_mut(jump_counter) {
            *arg = self.p_counter;
            Ok(())
        } else {
            return Err(CompileError::FailOr)
        }
    }

    /// AST::Seq 型に対応する Instruction を生成し、instructions に push する
    fn gen_seq(&mut self, vec:&Vec<AST>) -> Result<(), CompileError> {
        for ast in vec {
            self.gen_expr(ast)?;
        }
        Ok(())
    }

    /// AST から Instruction を生成し、instructions に push する  
    /// 最後に Match を instructions に push する
    fn gen_code(&mut self, ast: &AST) -> Result<(), CompileError> {
        // AST から Instruction を生成し、instructions に挿入する
        self.gen_expr(ast)?;

        // Match を instructions に挿入する
        self.increment_p_counter()?;
        self.instructions.push(Instruction::Match);
        Ok(())
    }
}

/// コード生成を行う関数
pub fn compile(ast: &AST) -> Result<Vec<Instruction>, CompileError> {
    let mut compiler: Compiler = Compiler::default();
    compiler.gen_code(ast)?;
    Ok(compiler.instructions)
}

// ----- テストコード -----

#[test]
fn test_increment_p_counter_success() {
    let count: usize = 10;
    let mut compiler: Compiler = Compiler {
        p_counter: count,
        instructions : Vec::new()
    };

    compiler.increment_p_counter().unwrap();
    let actual: usize = compiler.p_counter;
    assert_eq!(actual, count+1);
}

#[test]
fn test_increment_p_counter_failure() {
    let count: usize = usize::MAX;
    let mut compiler: Compiler = Compiler {
        p_counter: count,
        instructions : Vec::new()
    };

    let actual = compiler.increment_p_counter();
    assert_eq!(actual, Err(CompileError::PCOverFlow));
}

#[test]
fn test_gen_char_success() {
    let expect: Vec<Instruction> = vec![Instruction::Char(Char::Literal('a'))];
    let mut compiler: Compiler = Compiler {
        p_counter: 0,
        instructions : Vec::new()
    };

    let _ = compiler.gen_char('a');
    let actual: Vec<Instruction> = compiler.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_char_failure() {
    let expect = Err(CompileError::PCOverFlow);
    let mut compiler: Compiler = Compiler {
        p_counter: usize::MAX,
        instructions : Vec::new()
    };

    let actual = compiler.gen_char('a');    assert_eq!(actual, expect);
}

#[test]
fn test_gen_anychar() {
    let expect: Vec<Instruction> = vec![Instruction::Char(Char::Any)];

    let mut compiler: Compiler = Compiler {
        p_counter: 0,
        instructions : Vec::new()
    };
    let _ = compiler.gen_anychar();
    assert_eq!(compiler.instructions, expect)
}

#[test]
fn test_gen_star_success() {
    // a* が入力されたケース
    let expect: Vec<Instruction> = vec![
        Instruction::Split(1, 3),
        Instruction::Char(Char::Literal('a')),
        Instruction::Jump(0),
    ];

    let mut compiler: Compiler = Compiler {
        p_counter: 0,
        instructions : Vec::new()
    };

    let ast: Box<AST> = Box::new(AST::Char('a'));

    let _ = compiler.gen_star(&ast);
    let actual: Vec<Instruction> = compiler.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_star_failure() {
    // a* が入力されたケース
    let expect:Result<(), CompileError>  = Err(CompileError::FailStar);

    let mut compiler: Compiler = Compiler {
        p_counter: 100,
        instructions : Vec::new()
    };

    let ast: Box<AST> = Box::new(AST::Char('a'));

    let actual: Result<(), CompileError> = compiler.gen_star(&ast);
    assert_eq!(actual, expect);
}


#[test]
fn test_gen_plus_success() {
    // a+ が入力されたケース
    let expect: Vec<Instruction> = vec![
        Instruction::Char(Char::Literal('a')),
        Instruction::Split(0, 2),
    ];

    let mut compiler: Compiler = Compiler {
        p_counter: 0,
        instructions : Vec::new()
    };

    let ast: Box<AST> = Box::new(AST::Char('a'));

    let _ = compiler.gen_plus(&ast);
    let actual: Vec<Instruction> = compiler.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_question_success() {
    // a+ が入力されたケース
    let expect: Vec<Instruction> = vec![
        Instruction::Split(1, 2),
        Instruction::Char(Char::Literal('a')),
    ];

    let mut compiler: Compiler = Compiler {
        p_counter: 0,
        instructions : Vec::new()
    };

    let ast: Box<AST> = Box::new(AST::Char('a'));

    let _ = compiler.gen_question(&ast);
    let actual: Vec<Instruction> = compiler.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_question_failure() {
    let expect:Result<(), CompileError>  = Err(CompileError::FailQuestion);

    let mut compiler: Compiler = Compiler {
        p_counter: 100,
        instructions : Vec::new()
    };
    let ast: Box<AST> = Box::new(AST::Char('a'));

    let actual: Result<(), CompileError> = compiler.gen_question(&ast);
    assert_eq!(actual, expect);
}


#[test]
fn test_gen_or_success() {
    // a|b が入力されたケース
    let expect: Vec<Instruction> = vec![
        Instruction::Split(1, 3),
        Instruction::Char(Char::Literal('a')),
        Instruction::Jump(4),
        Instruction::Char(Char::Literal('b')),
    ];

    let mut compiler: Compiler = Compiler {
        p_counter: 0,
        instructions : Vec::new()
    };
 
    let e1: Box<AST> = Box::new(AST::Seq(vec![AST::Char('a')]));
    let e2: Box<AST> = Box::new(AST::Seq(vec![AST::Char('b')]));

    let _ = compiler.gen_or(&e1, &e2);
    let actual: Vec<Instruction> = compiler.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_or_failure() {
    let expect:Result<(), CompileError>  = Err(CompileError::FailOr);

    let mut compiler: Compiler = Compiler {
        p_counter: 100,
        instructions : Vec::new()
    };

    let e1: Box<AST> = Box::new(AST::Seq(vec![AST::Char('a')]));
    let e2: Box<AST> = Box::new(AST::Seq(vec![AST::Char('b')]));

    let actual: Result<(), CompileError> = compiler.gen_or(&e1, &e2);
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_seq_success() {
    let expect: Vec<Instruction> = vec![
        Instruction::Char(Char::Literal('a')),
        Instruction::Char(Char::Literal('b')),
        Instruction::Char(Char::Literal('c')),
    ];

    let v: Vec<AST> = vec![AST::Char('a'), AST::Char('b'), AST::Char('c')];

    let mut compiler: Compiler = Compiler {
        p_counter: 0,
        instructions : Vec::new()
    };

    let _ = compiler.gen_seq(&v);
    let actual: Vec<Instruction> = compiler.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_code_success() {
    // a|b が入力されたケース
    let expect: Vec<Instruction> = vec![
        Instruction::Split(1, 3),
        Instruction::Char(Char::Literal('a')),
        Instruction::Jump(4),
        Instruction::Char(Char::Literal('b')),
        Instruction::Match
    ];

    let mut compiler: Compiler = Compiler {
        p_counter: 0,
        instructions : Vec::new()
    };
 
    let e1: Box<AST> = Box::new(AST::Seq(vec![AST::Char('a')]));
    let e2: Box<AST> = Box::new(AST::Seq(vec![AST::Char('b')]));
    let or = AST::Or(e1, e2);

    let _ = compiler.gen_code(&or);
    let actual: Vec<Instruction> = compiler.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_compile_success() {
    let expect: Vec<Instruction> = vec![
        Instruction::Split(1, 3),
        Instruction::Char(Char::Literal('a')),
        Instruction::Jump(4),
        Instruction::Char(Char::Literal('b')),
        Instruction::Match
    ];

    let e1: Box<AST> = Box::new(AST::Seq(vec![AST::Char('a')]));
    let e2: Box<AST> = Box::new(AST::Seq(vec![AST::Char('b')]));
    let or = AST::Or(e1, e2);

    let actual = compile(&or).unwrap();
    assert_eq!(actual, expect);
}