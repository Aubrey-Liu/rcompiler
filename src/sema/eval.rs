use super::*;
use crate::ast::*;

pub trait ConstEval {
    fn const_eval(&self, symbols: &SymbolTable) -> Option<i32>;
}

impl ConstEval for UnaryExp {
    fn const_eval(&self, symbols: &SymbolTable) -> Option<i32> {
        match self {
            Self::Unary(op, exp) => exp.const_eval(symbols).map(|opr| eval_unary(*op, opr)),
            Self::Call(_) => None,
        }
    }
}

impl ConstEval for BinaryExp {
    fn const_eval(&self, symbols: &SymbolTable) -> Option<i32> {
        let lhs = self.lhs.const_eval(symbols);
        let rhs = self.rhs.const_eval(symbols);
        if let (Some(lhs), Some(rhs)) = (lhs, rhs) {
            Some(eval_binary(self.op, lhs, rhs))
        } else {
            None
        }
    }
}

impl ConstEval for Exp {
    fn const_eval(&self, symbols: &SymbolTable) -> Option<i32> {
        match self {
            Self::Integer(i) => Some(*i),
            Self::Uxp(uxp) => uxp.const_eval(symbols),
            Self::Bxp(bxp) => bxp.const_eval(symbols),
            Self::LVal(lval) => match symbols.get(&lval.ident) {
                Symbol::ConstVar(i) => Some(*i),
                _ => None,
            },
            Self::Error => panic!("expected an expression"),
        }
    }
}

impl Exp {
    pub fn get_i32(&self) -> i32 {
        if let Self::Integer(i) = self {
            *i
        } else {
            panic!("attempt to retrieve an integer from a non-const expression")
        }
    }
}
