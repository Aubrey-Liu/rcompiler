use core::panic;

#[derive(Debug)]
pub enum Expr {
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Integer(i32),
    LVal(LVal),
    Error,
}

#[derive(Debug)]
pub struct BinaryExpr {
    pub op: BinaryOp,
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
}

#[derive(Debug)]
pub enum UnaryExpr {
    Unary(UnaryOp, Box<Expr>),
    Call(Call),
}

#[derive(Debug)]
pub struct Call {
    pub ident: String,
    pub args: Vec<Expr>,
}

#[derive(Debug)]
pub struct LVal {
    pub ident: String,
    pub dims: Vec<Expr>,
}

#[derive(Debug)]
pub enum InitVal {
    Expr(Expr),
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
                panic!("attempt to calculate the remainder of integer with a divisor of zero");
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
