use super::{BinaryOp, UnaryOp};

pub fn lnd(x: i32, y: i32) -> i32 {
    // logical and
    (x != 0 && y != 0) as i32
}

pub fn lor(x: i32, y: i32) -> i32 {
    // logical or
    (x != 0 || y != 0) as i32
}

pub fn is_zero(i: i32) -> i32 {
    (i == 0) as i32
}

pub fn not_zero(i: i32) -> i32 {
    (i != 0) as i32
}

pub fn positive(x: i32) -> i32 {
    x.is_positive() as i32
}

pub fn non_negative(x: i32) -> i32 {
    !x.is_negative() as i32
}

pub fn eval_binary(op: BinaryOp, lhs: i32, rhs: i32) -> i32 {
    match op {
        BinaryOp::Add => lhs + rhs,
        BinaryOp::Sub => lhs - rhs,
        BinaryOp::Mul => lhs * rhs,
        BinaryOp::Div => lhs / rhs,
        BinaryOp::Mod => lhs % rhs,
        BinaryOp::And => lnd(lhs, rhs),
        BinaryOp::Or => lor(lhs, rhs),
        BinaryOp::Eq => is_zero(lhs - rhs),
        BinaryOp::Neq => not_zero(lhs - rhs),
        BinaryOp::Lt => positive(rhs - lhs),
        BinaryOp::Le => non_negative(rhs - lhs),
        BinaryOp::Gt => positive(lhs - rhs),
        BinaryOp::Ge => non_negative(lhs - rhs),
    }
}

pub fn eval_unary(op: UnaryOp, rhs: i32) -> i32 {
    match op {
        UnaryOp::Nop => rhs,
        UnaryOp::Neg => -rhs,
        UnaryOp::Not => is_zero(rhs),
    }
}
