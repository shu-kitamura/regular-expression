//! Ast を命令列(Instruction)にコンパイルするための型・関数  
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
    engine::{
        instruction::{Char, Instruction},
        parser::Ast,
        safe_add,
    },
    error::CompileError,
};

/// コンパイラの型
#[derive(Default, Debug)]
struct Compiler {
    p_counter: usize,
    instructions: Vec<Instruction>,
}

impl Compiler {
    /// p_counter をインクリメントさせる
    fn increment_p_counter(&mut self) -> Result<(), CompileError> {
        safe_add(&mut self.p_counter, &1, || CompileError::PCOverFlow)
    }

    /// ヘルパー関数: 第二引数が仮値の Split 命令を命令列に挿入する。
    ///
    /// 命令列に split を挿入する際、第二引数に指定するコードが、挿入時点ではまだ値がわからない。  
    /// そのため、ここでは仮の数値(0)を入れて、数値は後で更新する。
    ///
    /// この関数は、以下の手順で動作する。
    /// 1. 現在のプログラムカウンタ値を記録する。
    /// 2. p_counter をインクリメントする（Split 命令の左側アドレスとして使用）。
    /// 3. Instruction::Split(左側アドレス, 仮の右側アドレス(0)) を命令列に追加。
    /// 4. 挿入した命令のインデックスを返す。
    fn insert_split_placeholder(&mut self) -> Result<usize, CompileError> {
        let placeholder_index: usize = self.p_counter;
        self.increment_p_counter()?;
        self.instructions
            .push(Instruction::Split(self.p_counter, 0));
        Ok(placeholder_index)
    }

    /// ヘルパー関数: 指定されたインデックスの命令が Split または Jump 命令であれば、
    /// そのアドレスを現在のプログラムカウンタに更新する。
    ///
    /// - Split 命令の場合、右側アドレス (second argument) を更新する。
    /// - Jump 命令の場合、ジャンプ先アドレスを更新する。
    fn update_instruction_address(
        &mut self,
        index: usize,
        err: CompileError,
    ) -> Result<(), CompileError> {
        match self.instructions.get_mut(index) {
            Some(Instruction::Split(_, right)) => {
                *right = self.p_counter;
                Ok(())
            }
            Some(Instruction::Jump(addr)) => {
                *addr = self.p_counter;
                Ok(())
            }
            _ => Err(err),
        }
    }

    /// 入力された Ast の型に応じた関数を実行する
    fn gen_expr(&mut self, ast: &Ast) -> Result<(), CompileError> {
        match ast {
            Ast::AnyChar => self.gen_anychar(),
            Ast::Char(c) => self.gen_char(*c),
            Ast::Or(e1, e2) => self.gen_or(e1, e2),
            Ast::Plus(ast) => self.gen_plus(ast),
            Ast::Star(ast) => self.gen_star(ast),
            Ast::Question(ast) => self.gen_question(ast),
            Ast::Seq(v) => self.gen_seq(v),
        }
    }

    /// Ast::Char 型に対応する Instruction を生成し、instructions に push する
    fn gen_char(&mut self, c: char) -> Result<(), CompileError> {
        let inst: Instruction = Instruction::Char(Char::Literal(c));
        self.increment_p_counter()?;
        self.instructions.push(inst);
        Ok(())
    }

    /// Ast::AnyChar 型に対応する Instruction を生成し、instructions に push する
    fn gen_anychar(&mut self) -> Result<(), CompileError> {
        let inst: Instruction = Instruction::Char(Char::Any);
        self.increment_p_counter()?;
        self.instructions.push(inst);
        Ok(())
    }

    /// Ast::Star 型に対応する Instruction を生成し、instructions に push する  
    /// a* 入力された場合、以下のような Instruction を生成する  
    ///
    /// ```text
    /// 0 : split 1, 3
    /// 1 : Char(a)
    /// 2 : jump 0
    /// 3 : ... 続き
    /// ```
    fn gen_star(&mut self, ast: &Ast) -> Result<(), CompileError> {
        // split を挿入する。後で更新するため、格納した index を split_count に保持する。
        let split_count: usize = self.insert_split_placeholder()?;

        // Ast を再帰的に処理する
        self.gen_expr(ast)?;

        // カウンタをインクリメントし、Jump を挿入する
        self.increment_p_counter()?;
        self.instructions.push(Instruction::Jump(split_count));

        // Split の第二引数を更新する
        self.update_instruction_address(split_count, CompileError::FailStar)
    }

    /// Ast::Plus 型に対応する Instruction を生成し、instructions に push する  
    /// a+ 入力された場合、以下のような Instruction を生成する  
    ///
    /// ```text
    /// 0 : Char(a)
    /// 1 : split 0, 2
    /// 2 : ... 続き
    /// ```
    fn gen_plus(&mut self, ast: &Ast) -> Result<(), CompileError> {
        let left: usize = self.p_counter;
        // Ast を再帰的に処理する
        self.gen_expr(ast)?;

        // カウンタをインクリメントし Split を挿入する
        self.increment_p_counter()?;
        self.instructions
            .push(Instruction::Split(left, self.p_counter));
        Ok(())
    }

    /// Ast::Question 型に対応する Instruction を生成し、instructions に push する  
    /// a? 入力された場合、以下のような Instruction を生成する  
    ///
    /// ```text
    /// 0 : split 1, 2
    /// 1 : Char(a)
    /// 2 : ... 続き
    /// ```
    fn gen_question(&mut self, ast: &Ast) -> Result<(), CompileError> {
        // split を挿入する。後で更新するため、格納した index を split_count に保持する。
        let split_count: usize = self.insert_split_placeholder()?;
        // Ast を再帰的に処理する。
        self.gen_expr(ast)?;

        // Split の第二引数を更新する。
        self.update_instruction_address(split_count, CompileError::FailQuestion)
    }

    /// Ast::Or 型に対応する Instruction を生成し、instructions に push する  
    /// a|b が入力された場合、以下のような Instruction を生成する。  
    ///
    /// ```text
    /// 0 : split 1, 3
    /// 1 : Char(a)
    /// 2 : jump 4
    /// 3 : Char(b)
    /// 4 : ... 続き
    /// ```
    fn gen_or(&mut self, expr1: &Ast, expr2: &Ast) -> Result<(), CompileError> {
        // split を挿入する。後で更新するため、格納した index を split_count に保持する。
        let split_count: usize = self.insert_split_placeholder()?;
        // 1つ目の Ast を再帰的に処理する。
        self.gen_expr(expr1)?;

        let jump_count: usize = self.p_counter;
        self.increment_p_counter()?;
        // Jump を挿入する。引数には expr2 のコードの次のカウンタを指定する必要があるが、
        // この時点ではまだ値がわからないので、ここでは仮の数値(0)を入れて、数値は後で更新する。
        self.instructions.push(Instruction::Jump(0));

        // Splitの第二引数を更新する。
        self.update_instruction_address(split_count, CompileError::FailOr)?;

        // 2つ目の Ast を再帰的に処理する。
        self.gen_expr(expr2)?;

        // Jump の引数を更新する。
        self.update_instruction_address(jump_count, CompileError::FailOr)
    }

    /// Ast::Seq 型に対応する Instruction を生成し、instructions に push する
    fn gen_seq(&mut self, vec: &Vec<Ast>) -> Result<(), CompileError> {
        for ast in vec {
            self.gen_expr(ast)?;
        }
        Ok(())
    }

    /// Ast から Instruction を生成し、instructions に push する  
    /// 最後に Match を instructions に push する
    fn gen_code(&mut self, ast: &Ast) -> Result<(), CompileError> {
        // Ast から Instruction を生成し、instructions に挿入する
        self.gen_expr(ast)?;

        // Match を instructions に挿入する
        self.increment_p_counter()?;
        self.instructions.push(Instruction::Match);
        Ok(())
    }
}

/// コード生成を行う関数
pub fn compile(ast: &Ast) -> Result<Vec<Instruction>, CompileError> {
    let mut compiler: Compiler = Compiler::default();
    compiler.gen_code(ast)?;
    Ok(compiler.instructions)
}

// ----- テストコード -----

#[cfg(test)]
mod tests {

    use crate::{
        engine::{
            compiler::{compile, Compiler},
            instruction::{Char, Instruction},
            parser::Ast,
        },
        error::CompileError,
    };

    #[test]
    fn test_increment_p_counter_success() {
        let count: usize = 10;
        let mut compiler: Compiler = Compiler {
            p_counter: count,
            instructions: Vec::new(),
        };

        compiler.increment_p_counter().unwrap();
        let actual: usize = compiler.p_counter;
        assert_eq!(actual, count + 1);
    }

    #[test]
    fn test_increment_p_counter_failure() {
        let count: usize = usize::MAX;
        let mut compiler: Compiler = Compiler {
            p_counter: count,
            instructions: Vec::new(),
        };

        let actual = compiler.increment_p_counter();
        assert_eq!(actual, Err(CompileError::PCOverFlow));
    }

    #[test]
    fn test_insert_split_placeholder_success() {
        // 初期状態: p_counter = 0, instructions は空
        let mut compiler = Compiler {
            p_counter: 0,
            instructions: Vec::new(),
        };

        // 仮の Split 命令を挿入する
        let result = compiler.insert_split_placeholder();
        // 戻り値は初期の p_counter (0) であることを確認
        assert_eq!(result, Ok(0));

        // p_counter はインクリメントされ、1 になっているはず
        assert_eq!(compiler.p_counter, 1);

        // instructions に Split 命令が 1 つ挿入され、左側アドレスが p_counter (1)、
        // 右側は仮値 (0) となっていることを確認
        assert_eq!(compiler.instructions.len(), 1);
        match &compiler.instructions[0] {
            Instruction::Split(left, right) => {
                assert_eq!(*left, 1);
                assert_eq!(*right, 0);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_insert_split_placeholder_failure() {
        // p_counter が usize::MAX の場合、increment_p_counter が失敗するのでエラーが返るはず
        let mut compiler = Compiler {
            p_counter: usize::MAX,
            instructions: Vec::new(),
        };

        let result = compiler.insert_split_placeholder();
        // エラーとして CompileError::PCOverFlow が返ることを確認
        assert_eq!(result, Err(CompileError::PCOverFlow));
        // instructions は更新されず、空のままであることを確認
        assert!(compiler.instructions.is_empty());
    }

    #[test]
    fn test_update_instruction_address_success() {
        // Compiler を初期化。p_counter は 42 とする
        let mut compiler = Compiler {
            p_counter: 42,
            instructions: vec![
                Instruction::Split(10, 0), // 左側は任意の値、右側は仮値 0
                Instruction::Jump(0),      // 仮値 0
            ],
        };

        // インデックス 0 の Split 命令の右側アドレスを更新する
        let result = compiler.update_instruction_address(0, CompileError::FailOr);
        assert!(result.is_ok());
        if let Some(Instruction::Split(_, right)) = compiler.instructions.get(0) {
            assert_eq!(*right, 42);
        }

        // インデックス 1 の Jump 命令のアドレスを更新する
        let result = compiler.update_instruction_address(1, CompileError::FailOr);
        assert!(result.is_ok());
        if let Some(Instruction::Jump(addr)) = compiler.instructions.get(1) {
            assert_eq!(*addr, 42);
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
            instructions: vec![Instruction::Char(Char::Literal('a')), Instruction::Match],
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
            instructions: Vec::new(),
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
            instructions: Vec::new(),
        };

        let actual = compiler.gen_char('a');
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_gen_anychar() {
        let expect: Vec<Instruction> = vec![Instruction::Char(Char::Any)];

        let mut compiler: Compiler = Compiler {
            p_counter: 0,
            instructions: Vec::new(),
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
            instructions: Vec::new(),
        };

        let ast: Box<Ast> = Box::new(Ast::Char('a'));

        let _ = compiler.gen_star(&ast);
        let actual: Vec<Instruction> = compiler.instructions;
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_gen_star_failure() {
        // a* が入力されたケース
        let expect: Result<(), CompileError> = Err(CompileError::FailStar);

        let mut compiler: Compiler = Compiler {
            p_counter: 100,
            instructions: Vec::new(),
        };

        let ast: Box<Ast> = Box::new(Ast::Char('a'));

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
            instructions: Vec::new(),
        };

        let ast: Box<Ast> = Box::new(Ast::Char('a'));

        let _ = compiler.gen_plus(&ast);
        let actual: Vec<Instruction> = compiler.instructions;
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_gen_question_success() {
        // a? が入力されたケース
        let expect: Vec<Instruction> = vec![
            Instruction::Split(1, 2),
            Instruction::Char(Char::Literal('a')),
        ];

        let mut compiler: Compiler = Compiler {
            p_counter: 0,
            instructions: Vec::new(),
        };

        let ast: Box<Ast> = Box::new(Ast::Char('a'));

        let _ = compiler.gen_question(&ast);
        let actual: Vec<Instruction> = compiler.instructions;
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_gen_question_failure() {
        let expect: Result<(), CompileError> = Err(CompileError::FailQuestion);

        let mut compiler: Compiler = Compiler {
            p_counter: 100,
            instructions: Vec::new(),
        };
        let ast: Box<Ast> = Box::new(Ast::Char('a'));

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
            instructions: Vec::new(),
        };

        let e1: Box<Ast> = Box::new(Ast::Seq(vec![Ast::Char('a')]));
        let e2: Box<Ast> = Box::new(Ast::Seq(vec![Ast::Char('b')]));

        let _ = compiler.gen_or(&e1, &e2);
        let actual: Vec<Instruction> = compiler.instructions;
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_gen_or_failure() {
        let expect: Result<(), CompileError> = Err(CompileError::FailOr);

        let mut compiler: Compiler = Compiler {
            p_counter: 100,
            instructions: Vec::new(),
        };

        let e1: Box<Ast> = Box::new(Ast::Seq(vec![Ast::Char('a')]));
        let e2: Box<Ast> = Box::new(Ast::Seq(vec![Ast::Char('b')]));

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

        let v: Vec<Ast> = vec![Ast::Char('a'), Ast::Char('b'), Ast::Char('c')];

        let mut compiler: Compiler = Compiler {
            p_counter: 0,
            instructions: Vec::new(),
        };

        let _ = compiler.gen_seq(&v);
        let actual: Vec<Instruction> = compiler.instructions;
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_gen_code_containe_or() {
        // a|b が入力されたケース
        let expect: Vec<Instruction> = vec![
            Instruction::Split(1, 3),
            Instruction::Char(Char::Literal('a')),
            Instruction::Jump(4),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];

        let mut compiler: Compiler = Compiler {
            p_counter: 0,
            instructions: Vec::new(),
        };

        let e1: Box<Ast> = Box::new(Ast::Seq(vec![Ast::Char('a')]));
        let e2: Box<Ast> = Box::new(Ast::Seq(vec![Ast::Char('b')]));
        let or = Ast::Or(e1, e2);

        let _ = compiler.gen_code(&or);
        let actual: Vec<Instruction> = compiler.instructions;
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_gen_code_contain_any() {
        // a.b が入力されたケース
        let expect: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Any),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];
        let mut compiler: Compiler = Compiler {
            p_counter: 0,
            instructions: Vec::new(),
        };
        let ast: Box<Ast> = Box::new(Ast::Seq(vec![Ast::Char('a'), Ast::AnyChar, Ast::Char('b')]));
        let _ = compiler.gen_code(&ast);
        let actual: Vec<Instruction> = compiler.instructions;
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_gen_code_contain_star() {
        // a*b が入力されたケース
        let expect: Vec<Instruction> = vec![
            Instruction::Split(1, 3),
            Instruction::Char(Char::Literal('a')),
            Instruction::Jump(0),
            Instruction::Match,
        ];
        let mut compiler: Compiler = Compiler {
            p_counter: 0,
            instructions: Vec::new(),
        };
        let ast: Box<Ast> = Box::new(Ast::Star(Box::new(Ast::Char('a'))));
        let _ = compiler.gen_code(&ast);
        let actual: Vec<Instruction> = compiler.instructions;
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_gen_code_contain_plus() {
        // a+b が入力されたケース
        let expect: Vec<Instruction> = vec![
            Instruction::Char(Char::Literal('a')),
            Instruction::Split(0, 2),
            Instruction::Match,
        ];
        let mut compiler: Compiler = Compiler {
            p_counter: 0,
            instructions: Vec::new(),
        };
        let ast: Box<Ast> = Box::new(Ast::Plus(Box::new(Ast::Char('a'))));
        let _ = compiler.gen_code(&ast);
        let actual: Vec<Instruction> = compiler.instructions;
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_gen_code_contain_question() {
        // a?b が入力されたケース
        let expect: Vec<Instruction> = vec![
            Instruction::Split(1, 2),
            Instruction::Char(Char::Literal('a')),
            Instruction::Char(Char::Literal('b')),
            Instruction::Match,
        ];
        let mut compiler: Compiler = Compiler {
            p_counter: 0,
            instructions: Vec::new(),
        };
        let ast: Box<Ast> = Box::new(Ast::Seq(vec![
            Ast::Question(Box::new(Ast::Char('a'))),
            Ast::Char('b'),
        ]));
        let _ = compiler.gen_code(&ast);
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
            Instruction::Match,
        ];

        let e1: Box<Ast> = Box::new(Ast::Seq(vec![Ast::Char('a')]));
        let e2: Box<Ast> = Box::new(Ast::Seq(vec![Ast::Char('b')]));
        let or = Ast::Or(e1, e2);

        let actual = compile(&or).unwrap();
        assert_eq!(actual, expect);
    }
}
