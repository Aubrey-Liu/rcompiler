use koopa::ir::Value;

use super::*;
use crate::irgen::SymbolTable;

#[derive(Debug)]
pub enum Exp {
    Integer(i32),
    LVal(String, Option<Value>),
    Uxp(UnaryExp),
    Bxp(BinaryExp),
    Error,
}

#[derive(Debug)]
pub struct BinaryExp {
    pub op: BinaryOp,
    pub lhs: Box<Exp>,
    pub rhs: Box<Exp>,
}

#[derive(Debug)]
pub struct UnaryExp {
    pub op: UnaryOp,
    pub rhs: Box<Exp>,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Nop,
    Neg,
    Not,
}

impl Exp {
    pub fn is_logical(&self) -> bool {
        if let Self::Bxp(bxp) = self {
            matches!(bxp.op, BinaryOp::And | BinaryOp::Or)
        } else {
            false
        }
    }

    pub fn get_binary_op(&self) -> Option<BinaryOp> {
        if let Self::Bxp(bxp) = self {
            Some(bxp.op)
        } else {
            None
        }
    }

    pub fn get_bxp(&self) -> Option<&BinaryExp> {
        if let Self::Bxp(bxp) = self {
            Some(bxp)
        } else {
            None
        }
    }
}

pub trait ConstEval {
    fn const_eval(&self, symt: &SymbolTable) -> i32;
}

impl ConstEval for UnaryExp {
    fn const_eval(&self, symt: &SymbolTable) -> i32 {
        let rhs = self.rhs.const_eval(symt);
        eval_unary(self.op, rhs)
    }
}

impl ConstEval for BinaryExp {
    fn const_eval(&self, symt: &SymbolTable) -> i32 {
        let lhs = self.lhs.const_eval(symt);
        let rhs = self.rhs.const_eval(symt);
        eval_binary(self.op, lhs, rhs)
    }
}

impl ConstEval for Exp {
    fn const_eval(&self, symt: &SymbolTable) -> i32 {
        match self {
            Exp::Integer(i) => *i,
            Exp::Uxp(uxp) => uxp.const_eval(symt),
            Exp::Bxp(bxp) => bxp.const_eval(symt),
            Exp::LVal(name, ..) => symt.get_from_const_var(name.as_str()).unwrap(),
            Exp::Error => panic!("expected an expression"),
        }
    }
}
