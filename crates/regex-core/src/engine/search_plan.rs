use crate::engine::instruction::{Char, Instruction};

/// マッチ候補の開始位置を絞り込むための計画データ
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPlan {
    pub can_match_empty: bool,
    pub has_any_first_byte: bool,
    pub first_byte_mask: [u64; 4],
    pub leading_literal: Option<Vec<u8>>,
}

impl SearchPlan {
    pub fn build(insts: &[Instruction]) -> Self {
        let mut plan = SearchPlan {
            can_match_empty: false,
            has_any_first_byte: false,
            first_byte_mask: [0; 4],
            leading_literal: Self::detect_leading_literal(insts),
        };
        plan.collect_first_bytes(insts);
        plan
    }

    fn detect_leading_literal(insts: &[Instruction]) -> Option<Vec<u8>> {
        let mut bytes = Vec::new();
        let mut pc = 0usize;

        while let Some(inst) = insts.get(pc) {
            match inst {
                Instruction::Char(Char::Literal(b)) => {
                    bytes.push(*b);
                    pc += 1;
                }
                Instruction::Char(Char::Any)
                | Instruction::Match
                | Instruction::Jump(_)
                | Instruction::Split(_, _) => break,
            }
        }

        if bytes.is_empty() { None } else { Some(bytes) }
    }

    fn collect_first_bytes(&mut self, insts: &[Instruction]) {
        if insts.is_empty() {
            self.can_match_empty = true;
            return;
        }

        let mut stack = vec![0usize];
        let mut visited = vec![false; insts.len()];

        while let Some(pc) = stack.pop() {
            let Some(inst) = insts.get(pc) else {
                continue;
            };
            if visited[pc] {
                continue;
            }
            visited[pc] = true;

            match inst {
                Instruction::Match => self.can_match_empty = true,
                Instruction::Char(Char::Any) => self.has_any_first_byte = true,
                Instruction::Char(Char::Literal(b)) => self.add_first_byte(*b),
                Instruction::Jump(next) => {
                    if *next < insts.len() {
                        stack.push(*next);
                    }
                }
                Instruction::Split(left, right) => {
                    if *left < insts.len() {
                        stack.push(*left);
                    }
                    if *right < insts.len() {
                        stack.push(*right);
                    }
                }
            }
        }
    }

    fn add_first_byte(&mut self, byte: u8) {
        let index = (byte / 64) as usize;
        let bit = 1u64 << (byte % 64);
        self.first_byte_mask[index] |= bit;
    }

    fn contains_first_byte(&self, byte: u8) -> bool {
        let index = (byte / 64) as usize;
        let bit = 1u64 << (byte % 64);
        (self.first_byte_mask[index] & bit) != 0
    }

    pub fn accepts_first_byte(&self, byte: u8, ignore_case_ascii: bool) -> bool {
        if self.has_any_first_byte {
            return true;
        }
        if ignore_case_ascii {
            self.contains_first_byte(byte.to_ascii_lowercase())
        } else {
            self.contains_first_byte(byte)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        instruction::{Char, Instruction},
        search_plan::SearchPlan,
    };

    #[test]
    fn test_build_literal_plan() {
        let insts = vec![
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Char(Char::Literal(b'b')),
            Instruction::Match,
        ];
        let plan = SearchPlan::build(&insts);

        assert!(!plan.can_match_empty);
        assert!(!plan.has_any_first_byte);
        assert!(plan.accepts_first_byte(b'a', false));
        assert!(!plan.accepts_first_byte(b'b', false));
        assert_eq!(plan.leading_literal, Some(b"ab".to_vec()));
    }

    #[test]
    fn test_build_split_plan() {
        let insts = vec![
            Instruction::Split(1, 3),
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Jump(5),
            Instruction::Char(Char::Literal(b'b')),
            Instruction::Jump(5),
            Instruction::Match,
        ];
        let plan = SearchPlan::build(&insts);

        assert!(!plan.can_match_empty);
        assert!(!plan.has_any_first_byte);
        assert!(plan.accepts_first_byte(b'a', false));
        assert!(plan.accepts_first_byte(b'b', false));
        assert_eq!(plan.leading_literal, None);
    }

    #[test]
    fn test_build_empty_match_plan() {
        let insts = vec![
            Instruction::Split(1, 3),
            Instruction::Char(Char::Literal(b'a')),
            Instruction::Jump(0),
            Instruction::Match,
        ];
        let plan = SearchPlan::build(&insts);

        assert!(plan.can_match_empty);
        assert!(plan.accepts_first_byte(b'a', false));
        assert!(!plan.accepts_first_byte(b'b', false));
        assert_eq!(plan.leading_literal, None);
    }

    #[test]
    fn test_build_any_plan() {
        let insts = vec![Instruction::Char(Char::Any), Instruction::Match];
        let plan = SearchPlan::build(&insts);

        assert!(plan.has_any_first_byte);
        assert!(plan.accepts_first_byte(0x00, false));
        assert!(plan.accepts_first_byte(0xFF, false));
    }

    #[test]
    fn test_ignore_case_first_byte() {
        let insts = vec![Instruction::Char(Char::Literal(b'a')), Instruction::Match];
        let plan = SearchPlan::build(&insts);

        assert!(plan.accepts_first_byte(b'A', true));
        assert!(!plan.accepts_first_byte(b'A', false));
    }

    #[test]
    fn test_invalid_jump_is_safe() {
        let insts = vec![Instruction::Jump(999)];
        let plan = SearchPlan::build(&insts);
        assert!(!plan.can_match_empty);
        assert!(!plan.has_any_first_byte);
        assert_eq!(plan.first_byte_mask, [0; 4]);
    }
}
