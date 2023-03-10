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

pub fn eval_binary(op: BinaryOp, left: i32, right: i32) -> i32 {
    match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Div => left / right,
        BinaryOp::Mod => left % right,
        BinaryOp::And => lnd(left, right),
        BinaryOp::Or => lor(left, right),
        BinaryOp::Eq => is_zero(left - right),
        BinaryOp::Neq => not_zero(left - right),
        BinaryOp::Lt => positive(right - left),
        BinaryOp::Lte => non_negative(right - left),
        BinaryOp::Gt => positive(left - right),
        BinaryOp::Gte => non_negative(left - right),
    }
}

pub fn eval_unary(op: UnaryOp, right: i32) -> i32 {
    match op {
        UnaryOp::Nop => right,
        UnaryOp::Neg => -right,
        UnaryOp::Not => is_zero(right),
    }
}
