use core::panic;

use crate::irgen::record::Symbol;
use crate::irgen::ProgramRecorder;

#[derive(Debug)]
pub enum Exp {
    Integer(i32),
    LVal(String),
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
pub enum UnaryExp {
    Unary(UnaryOp, Box<Exp>),
    Call(Call),
}

#[derive(Debug)]
pub struct Call {
    pub func_id: String,
    pub args: Vec<Box<Exp>>,
}

#[derive(Debug)]
pub struct LVal {
    pub ident: String,
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

pub trait ConstEval {
    fn const_eval(&self, recorder: &ProgramRecorder) -> i32;
}

impl ConstEval for UnaryExp {
    fn const_eval(&self, recorder: &ProgramRecorder) -> i32 {
        if let UnaryExp::Unary(op, exp) = self {
            let opr = exp.const_eval(recorder);
            return eval_unary(*op, opr);
        }
        panic!("attempt to const evaluate a function call");
    }
}

impl ConstEval for BinaryExp {
    fn const_eval(&self, recorder: &ProgramRecorder) -> i32 {
        let lhs = self.lhs.const_eval(recorder);
        let rhs = self.rhs.const_eval(recorder);
        eval_binary(self.op, lhs, rhs)
    }
}

impl ConstEval for Exp {
    fn const_eval(&self, recorder: &ProgramRecorder) -> i32 {
        match self {
            Exp::Integer(i) => *i,
            Exp::Uxp(uxp) => uxp.const_eval(recorder),
            Exp::Bxp(bxp) => bxp.const_eval(recorder),
            Exp::LVal(name) => {
                if let Symbol::ConstVar(i) = recorder.get_symbol(name).unwrap() {
                    *i
                } else {
                    panic!("attempt to use a non-constant value in a constant")
                }
            }
            Exp::Error => panic!("expected an expression"),
        }
    }
}

pub fn eval_binary(op: BinaryOp, lhs: i32, rhs: i32) -> i32 {
    match op {
        BinaryOp::Add => lhs + rhs,
        BinaryOp::Sub => lhs - rhs,
        BinaryOp::Mul => lhs * rhs,
        BinaryOp::And => (lhs != 0 && rhs != 0) as i32,
        BinaryOp::Or => (lhs != 0 || rhs != 0) as i32,
        BinaryOp::Eq => (lhs == rhs) as i32,
        BinaryOp::Neq => (lhs != rhs) as i32,
        BinaryOp::Lt => (lhs < rhs) as i32,
        BinaryOp::Le => (lhs <= rhs) as i32,
        BinaryOp::Gt => (lhs > rhs) as i32,
        BinaryOp::Ge => (lhs >= rhs) as i32,
        BinaryOp::Div => {
            if rhs != 0 {
                lhs / rhs
            } else {
                panic!("attempt to divide an integer by zero");
            }
        }
        BinaryOp::Mod => {
            if rhs != 0 {
                lhs % rhs
            } else {
                panic!("attempt to calculate the remainder of `1_i32` with a divisor of zero");
            }
        }
    }
}

pub fn eval_unary(op: UnaryOp, rhs: i32) -> i32 {
    match op {
        UnaryOp::Nop => rhs,
        UnaryOp::Neg => -rhs,
        UnaryOp::Not => (rhs == 0) as i32,
    }
}
