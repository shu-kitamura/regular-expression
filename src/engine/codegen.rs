use super::{
    instruction::Instruction,
    parser::AST
};
use crate::{
    helper::safe_add,
    error::CodeGenError,
};


/// コード生成器の型
#[derive(Default, Debug)]
struct Generator {
    p_counter: usize,
    instructions: Vec<Instruction>,
}

impl Generator {
    fn increment_p_counter(&mut self) -> Result<(), CodeGenError> {
        safe_add(&mut self.p_counter, &1, || CodeGenError::PCOverFlow)
    }

    fn gen_expr(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        match ast {
            AST::Char(c) => self.gen_char(*c),
            AST::Or(e1, e2) => self.gen_or(e1, e2),
            AST::Plus(ast) => self.gen_plus(ast),
            AST::Star(ast) => self.gen_star(ast),
            AST::Question(ast) => self.gen_question(ast),
            AST::Seq(v) => self.gen_seq(v),
        }
    }

    fn gen_char(&mut self, c: char) -> Result<(), CodeGenError> {
        let inst: Instruction = Instruction::Char(c);
        match self.increment_p_counter() {
            Ok(()) => {
                self.instructions.push(inst);
                Ok(())
            },
            Err(e) => Err(e),
        }
    }

    fn gen_star(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        let split_count: usize = self.p_counter;

        // カウンタをインクリメントし、split を挿入
        match self.increment_p_counter() {
            // 第二引数は、後に出てくる Jump のカウンタの数値を示すものであり、この時点では決まらないので仮の数値(ここでは 0 )を入れる
            // 仮の数値は、Jump を挿入した後に更新する
            Ok(()) => self.instructions.push(Instruction::Split(self.p_counter, 0)),
            Err(e) => return Err(e),
        };

        // ast を再帰的に処理
        match self.gen_expr(ast) {
            Ok(()) => {},
            Err(e) => return Err(e),
        };

        // カウンタをインクリメントし、Jump を挿入
        match self.increment_p_counter() {
            Ok(()) => self.instructions.push(Instruction::Jump(split_count)),
            Err(e) => return Err(e),
        }

        // 仮の数値としていた Split の第二引数を更新
        if let Some(Instruction::Split(_, right)) = self.instructions.get_mut(split_count) {
            *right = self.p_counter;
            Ok(())
        } else {
            Err(CodeGenError::FailStar)
        }
    }

    fn gen_plus(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        let left: usize = self.p_counter;
        // ast を再帰的に処理
        match self.gen_expr(ast) {
            Ok(()) => {},
            Err(e) => return Err(e),
        };

        match self.increment_p_counter() {
            Ok(()) => {
                self.instructions.push(Instruction::Split(left, self.p_counter));
                Ok(())
            },
            Err(e) => return Err(e),
        }
    }

    fn gen_question(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        let split_count: usize = self.p_counter;

        match self.increment_p_counter() {
            // 第二引数は、この時点では決まらないので仮の数値(ここでは 0 )を入れる
            // 仮の数値は、後に更新する
            Ok(()) => self.instructions.push(Instruction::Split(self.p_counter, 0)),
            Err(e) => return Err(e),
        };

        match self.gen_expr(ast) {
            Ok(()) => {},
            Err(e) => return Err(e),
        };

        // 仮の数値としていた Split の第二引数を更新
        if let Some(Instruction::Split(_, right)) = self.instructions.get_mut(split_count) {
            *right = self.p_counter;
            Ok(())
        } else {
            Err(CodeGenError::FailQuestion)
        }
    }

    fn gen_or(&mut self, expr1: &AST, expr2: &AST) -> Result<(), CodeGenError> {
        let split_counter: usize = self.p_counter;
        match self.increment_p_counter() {
            Ok(()) => self.instructions.push(
                // 第二引数は、expr2のコードの開始のカウンタを指定するため、この時点では決まらない
                // ここでは仮の数値(0)を入れて。数値は後で更新する
                Instruction::Split(self.p_counter, 0)
            ),
            Err(e) => return Err(e),
        };

        match self.gen_expr(expr1) {
            Ok(()) => {},
            Err(e) => return Err(e),
        };

        let jump_counter: usize = self.p_counter;

        match self.increment_p_counter() {
            Ok(()) => self.instructions.push(
                // 引数は、expr2 のコードの次のカウンタを指定するため、この時点では決まらない
                // ここでは仮の数値(0)を入れて。数値は後で更新する
                Instruction::Jump(0)
            ),
            Err(e) => return Err(e),
        };

        // Splitの第二引数を更新
        if let Some(Instruction::Split(_, right)) = self.instructions.get_mut(split_counter) {
            *right = self.p_counter;
        } else {
            return Err(CodeGenError::FailOr);
        }

        match self.gen_expr(expr2) {
            Ok(()) => {},
            Err(e) => return Err(e),
        };

        // Jumpの引数を更新
        if let Some(Instruction::Jump(arg)) = self.instructions.get_mut(jump_counter) {
            *arg = self.p_counter;
            Ok(())
        } else {
            return Err(CodeGenError::FailOr)
        }
    }

    fn gen_seq(&mut self, vec:&Vec<AST>) -> Result<(), CodeGenError> {
        for ast in vec {
            match self.gen_expr(ast){
                Ok(()) => {},
                Err(e) => return Err(e),
            };
        }
        Ok(())
    }

    fn gen_code(&mut self, ast: &AST) -> Result<(), CodeGenError> {
        match self.gen_expr(ast) {
            Ok(()) => {},
            Err(e) => return Err(e),
        };

        match self.increment_p_counter() {
            Ok(()) => {
                self.instructions.push(Instruction::Match);
                Ok(())
            }
            Err(e) => Err(e)
        }
    }
}

pub fn get_code(ast: &AST) -> Result<Vec<Instruction>, CodeGenError> {
    let mut generator = Generator::default();
    match generator.gen_code(ast) {
        Ok(()) => Ok(generator.instructions),
        Err(e) => Err(e)
    }
}

// ----- テストコード -----

#[test]
fn test_increment_p_counter_success() {
    let count: usize = 10;
    let mut generator: Generator = Generator {
        p_counter: count,
        instructions : Vec::new()
    };

    generator.increment_p_counter().unwrap();
    let actual: usize = generator.p_counter;
    assert_eq!(actual, count+1);
}

#[test]
fn test_increment_p_counter_failure() {
    let count: usize = 18446744073709551615;
    let mut generator: Generator = Generator {
        p_counter: count,
        instructions : Vec::new()
    };

    let actual = generator.increment_p_counter();
    assert_eq!(actual, Err(CodeGenError::PCOverFlow));
}

#[test]
fn test_gen_char_success() {
    let expect: Vec<Instruction> = vec![Instruction::Char('a')];
    let mut generator: Generator = Generator {
        p_counter: 0,
        instructions : Vec::new()
    };

    let _ = generator.gen_char('a');
    let actual: Vec<Instruction> = generator.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_char_failure() {
    let expect = Err(CodeGenError::PCOverFlow);
    let mut generator: Generator = Generator {
        p_counter: 18446744073709551615,
        instructions : Vec::new()
    };

    let actual = generator.gen_char('a');
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_star_success() {
    // a* が入力されたケース
    let expect: Vec<Instruction> = vec![
        Instruction::Split(1, 3),
        Instruction::Char('a'),
        Instruction::Jump(0),
    ];

    let mut generator: Generator = Generator {
        p_counter: 0,
        instructions : Vec::new()
    };

    let ast: Box<AST> = Box::new(AST::Char('a'));

    let _ = generator.gen_star(&ast);
    let actual: Vec<Instruction> = generator.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_plus_success() {
    // a+ が入力されたケース
    let expect: Vec<Instruction> = vec![
        Instruction::Char('a'),
        Instruction::Split(0, 2),
    ];

    let mut generator: Generator = Generator {
        p_counter: 0,
        instructions : Vec::new()
    };

    let ast: Box<AST> = Box::new(AST::Char('a'));

    let _ = generator.gen_plus(&ast);
    let actual: Vec<Instruction> = generator.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_question_success() {
    // a+ が入力されたケース
    let expect: Vec<Instruction> = vec![
        Instruction::Split(1, 2),
        Instruction::Char('a'),
    ];

    let mut generator: Generator = Generator {
        p_counter: 0,
        instructions : Vec::new()
    };

    let ast: Box<AST> = Box::new(AST::Char('a'));

    let _ = generator.gen_question(&ast);
    let actual: Vec<Instruction> = generator.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_or_success() {
    // a|b が入力されたケース
    let expect: Vec<Instruction> = vec![
        Instruction::Split(1, 3),
        Instruction::Char('a'),
        Instruction::Jump(4),
        Instruction::Char('b'),
    ];

    let mut generator: Generator = Generator {
        p_counter: 0,
        instructions : Vec::new()
    };
 
    let e1: Box<AST> = Box::new(AST::Seq(vec![AST::Char('a')]));
    let e2: Box<AST> = Box::new(AST::Seq(vec![AST::Char('b')]));

    let _ = generator.gen_or(&e1, &e2);
    let actual: Vec<Instruction> = generator.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_seq_success() {
    let expect: Vec<Instruction> = vec![
        Instruction::Char('a'),
        Instruction::Char('b'),
        Instruction::Char('c'),
    ];

    let v: Vec<AST> = vec![AST::Char('a'), AST::Char('b'), AST::Char('c')];

    let mut generator: Generator = Generator {
        p_counter: 0,
        instructions : Vec::new()
    };

    let _ = generator.gen_seq(&v);
    let actual: Vec<Instruction> = generator.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_gen_code_success() {
    // a|b が入力されたケース
    let expect: Vec<Instruction> = vec![
        Instruction::Split(1, 3),
        Instruction::Char('a'),
        Instruction::Jump(4),
        Instruction::Char('b'),
        Instruction::Match
    ];

    let mut generator: Generator = Generator {
        p_counter: 0,
        instructions : Vec::new()
    };
 
    let e1: Box<AST> = Box::new(AST::Seq(vec![AST::Char('a')]));
    let e2: Box<AST> = Box::new(AST::Seq(vec![AST::Char('b')]));
    let or = AST::Or(e1, e2);

    let _ = generator.gen_code(&or);
    let actual: Vec<Instruction> = generator.instructions;
    assert_eq!(actual, expect);
}

#[test]
fn test_get_code_success() {
    let expect: Vec<Instruction> = vec![
        Instruction::Split(1, 3),
        Instruction::Char('a'),
        Instruction::Jump(4),
        Instruction::Char('b'),
        Instruction::Match
    ];

    let e1: Box<AST> = Box::new(AST::Seq(vec![AST::Char('a')]));
    let e2: Box<AST> = Box::new(AST::Seq(vec![AST::Char('b')]));
    let or = AST::Or(e1, e2);

    let actual = get_code(&or).unwrap();
    assert_eq!(actual, expect);
}