use core::panic;

use crate::irgen::record::Symbol;
use crate::irgen::ProgramRecorder;

#[derive(Debug)]
pub enum Exp {
    Integer(i32),
    LVal(LVal),
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
    pub args: Vec<Exp>,
}

#[derive(Debug)]
pub struct LVal {
    pub ident: String,
    pub dims: Vec<Exp>,
}

#[derive(Debug)]
pub enum InitVal {
    Exp(Exp),
    List(Vec<InitVal>),
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
    fn const_eval(&self, recorder: &ProgramRecorder) -> Option<i32>;
}

impl ConstEval for UnaryExp {
    fn const_eval(&self, recorder: &ProgramRecorder) -> Option<i32> {
        match self {
            UnaryExp::Unary(op, exp) => {
                if let Some(opr) = exp.const_eval(recorder) {
                    Some(eval_unary(*op, opr))
                } else {
                    None
                }
            }
            UnaryExp::Call(_) => None,
        }
    }
}

impl ConstEval for BinaryExp {
    fn const_eval(&self, recorder: &ProgramRecorder) -> Option<i32> {
        let lhs = self.lhs.const_eval(recorder);
        let rhs = self.rhs.const_eval(recorder);
        if let (Some(lhs), Some(rhs)) = (lhs, rhs) {
            Some(eval_binary(self.op, lhs, rhs))
        } else {
            None
        }
    }
}

impl ConstEval for Exp {
    fn const_eval(&self, recorder: &ProgramRecorder) -> Option<i32> {
        match self {
            Exp::Integer(i) => Some(*i),
            Exp::Uxp(uxp) => uxp.const_eval(recorder),
            Exp::Bxp(bxp) => bxp.const_eval(recorder),
            Exp::LVal(lval) => {
                let id = &lval.ident;
                if let Ok(Symbol::ConstVar(i)) = recorder.get_symbol(id) {
                    Some(*i)
                } else {
                    None
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

pub fn eval_unary(op: UnaryOp, opr: i32) -> i32 {
    match op {
        UnaryOp::Nop => opr,
        UnaryOp::Neg => -opr,
        UnaryOp::Not => (opr == 0) as i32,
    }
}
