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

    /// ヘルパー関数: 指定されたインデックスの命令が Split または Jump 命令であれば、
    /// そのアドレスを現在のプログラムカウンタに更新する。
    ///
    /// - Split 命令の場合、右側アドレス (second argument) を更新する。
    /// - Jump 命令の場合、ジャンプ先アドレスを更新する。
    fn update_instruction_address(&mut self, index: usize, err: CompileError) -> Result<(), CompileError> {
        match self.instructions.get_mut(index) {
            Some(Instruction::Split(_, right)) => {
                *right = self.p_counter;
                Ok(())
            },
            Some(Instruction::Jump(addr)) => {
                *addr = self.p_counter;
                Ok(())
            },
            _ => Err(err)
        }
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
        self.update_instruction_address(split_count, CompileError::FailStar)
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
        self.update_instruction_address(split_count, CompileError::FailQuestion)

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
        self.update_instruction_address(split_counter, CompileError::FailOr)?;

        // 2つ目の AST を再帰的に処理する。
        self.gen_expr(expr2)?;

        // Jump の引数を更新する。
        self.update_instruction_address(jump_counter, CompileError::FailOr)
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

#[cfg(test)]
mod tests {

    use crate::{
        engine::{
            parser::AST,
            instruction::{Instruction, Char},
            compiler::{Compiler, compile}
        },
        error::CompileError
    };

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
    fn test_update_instruction_address_success() {
        // Compiler を初期化。p_counter は 42 とする
        let mut compiler = Compiler {
            p_counter: 42,
            instructions: vec![
                Instruction::Split(10, 0), // 左側は任意の値、右側は仮値 0
                Instruction::Jump(0)       // 仮値 0
            ], 
        };

        // インデックス 0 の Split 命令の右側アドレスを更新する
        let result = compiler.update_instruction_address(0, CompileError::FailOr);
        assert!(result.is_ok());
        if let Some(Instruction::Split(_, right)) = compiler.instructions.get(0) {
            assert_eq!(*right, 42);
        } else {
            panic!("インデックス 0 には Split 命令があるはずです");
        }

        // インデックス 1 の Jump 命令のアドレスを更新する
        let result = compiler.update_instruction_address(1, CompileError::FailOr);
        assert!(result.is_ok());
        if let Some(Instruction::Jump(addr)) = compiler.instructions.get(1) {
            assert_eq!(*addr, 42);
        } else {
            panic!("インデックス 1 には Jump 命令があるはずです");
        }
    }

    #[test]
    fn test_update_instruction_address_failure_invalid_index() {
        let mut compiler = Compiler {
            p_counter: 99,
            instructions: vec![], // 空の命令リスト
        };

        let result = compiler.update_instruction_address(0, CompileError::FailOr);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CompileError::FailOr);
    }

    #[test]
    fn test_update_instruction_address_failure_invalid_instruction() {
        let mut compiler = Compiler {
            p_counter: 99,
            instructions: vec![
                Instruction::Char(Char::Literal('a')),
                Instruction::Match
            ],
        };

        // インデックス 0 の命令は Char なのでエラーになる
        let result = compiler.update_instruction_address(0, CompileError::FailOr);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CompileError::FailOr);

        // インデックス 1 の命令は Match なのでエラーになる
        let result = compiler.update_instruction_address(1, CompileError::FailOr);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CompileError::FailOr);
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
}