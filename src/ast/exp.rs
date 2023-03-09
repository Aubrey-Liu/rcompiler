use super::*;

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

pub trait Eval {
    fn eval(&self, symt: &SymbolTable, is_const: bool) -> i32;
}

impl Eval for UnaryExp {
    fn eval(&self, symt: &SymbolTable, is_const: bool) -> i32 {
        let right = self.right.eval(symt, is_const);

        match self.op {
            UnaryOp::Nop => right,
            UnaryOp::Neg => -right,
            UnaryOp::Not => is_zero(right),
        }
    }
}

impl Eval for BinaryExp {
    fn eval(&self, symt: &SymbolTable, is_const: bool) -> i32 {
        let left = self.left.eval(symt, is_const);
        let right = self.right.eval(symt, is_const);

        match self.op {
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
}

impl Eval for Exp {
    fn eval(&self, symt: &SymbolTable, is_const: bool) -> i32 {
        match self {
            Exp::Integer(i) => *i,
            Exp::Uxp(uxp) => uxp.eval(symt, is_const),
            Exp::Bxp(bxp) => bxp.eval(symt, is_const),
            Exp::LVal(name) => {
                let sym = symt.get(name).unwrap();
                match sym {
                    Symbol::ConstVar(val) => *val,
                    Symbol::Var(exp) => {
                        if is_const {
                            panic!("Non-const variable in a const expression")
                        } else {
                            exp.eval(symt, is_const)
                        }
                    }
                }
            }
        }
    }
}
