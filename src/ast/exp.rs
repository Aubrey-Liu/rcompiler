use super::*;
use crate::irgen::record::Symbol;
use crate::irgen::ProgramRecorder;

impl Exp {
    pub fn is_logical_exp(&self) -> bool {
        if let Self::Bxp(bxp) = self {
            matches!(bxp.op, BinaryOp::And | BinaryOp::Or)
        } else {
            false
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
    fn const_eval(&self, recorder: &ProgramRecorder) -> i32;
}

impl ConstEval for UnaryExp {
    fn const_eval(&self, recorder: &ProgramRecorder) -> i32 {
        let rhs = self.rhs.const_eval(recorder);
        eval_unary(self.op, rhs)
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
