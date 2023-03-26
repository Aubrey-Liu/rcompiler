use super::{BinaryOp, UnaryOp};

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
