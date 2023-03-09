use super::symt::*;
use super::utils::*;

#[derive(Debug, Clone)]
pub enum Exp {
    Integer(i32),
    LVal(SymbolID),
    Uxp(UnaryExp),
    Bxp(BinaryExp),
}

#[derive(Debug, Clone)]
pub struct BinaryExp {
    pub op: BinaryOp,
    pub left: Box<Exp>,
    pub right: Box<Exp>,
}


#[derive(Debug, Clone)]
pub struct UnaryExp {
    pub op: UnaryOp,
    pub right: Box<Exp>,
}

#[derive(Debug, Clone)]
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
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Nop,
    Neg,
    Not,
}


impl Exp {
    pub fn parse(&self) -> i32 {
        match self {
            Exp::Integer(i) => *i,
            Exp::Uxp(uxp) => match uxp.op {
                UnaryOp::Nop => uxp.right.parse(),
                UnaryOp::Neg => -uxp.right.parse(),
                UnaryOp::Not => is_zero(uxp.right.parse()),
            },
            Exp::Bxp(bxp) => match bxp.op {
                BinaryOp::Add => bxp.left.parse() + bxp.right.parse(),
                BinaryOp::Sub => bxp.left.parse() - bxp.right.parse(),
                BinaryOp::Mul => bxp.left.parse() * bxp.right.parse(),
                BinaryOp::Div => bxp.left.parse() / bxp.right.parse(),
                BinaryOp::Mod => bxp.left.parse() % bxp.right.parse(),
                BinaryOp::And => lnd(bxp.left.parse(), bxp.right.parse()),
                BinaryOp::Or => lor(bxp.left.parse(), bxp.right.parse()),
                BinaryOp::Eq => is_zero(bxp.left.parse() - bxp.right.parse()),
                BinaryOp::Neq => not_zero(bxp.left.parse() - bxp.right.parse()),
                BinaryOp::Lt => positive(bxp.right.parse() - bxp.left.parse()),
                BinaryOp::Lte => non_negative(bxp.right.parse() - bxp.left.parse()),
                BinaryOp::Gt => positive(bxp.left.parse() - bxp.right.parse()),
                BinaryOp::Gte => non_negative(bxp.left.parse() - bxp.right.parse()),
            },
            _ => todo!(),
        }
    }
}

