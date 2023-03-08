use std::collections::HashMap;

#[derive(Debug)]
pub struct CompUnit {
    pub func_def: FuncDef,
}

#[derive(Debug)]
pub struct FuncDef {
    pub func_type: FuncType,
    pub ident: String,
    pub block: Block,
}

#[derive(Debug)]
pub struct FuncType(pub String);

#[derive(Debug, Clone)]
pub enum AstValue {
    Return(Exp),
    End, // ; is the end of a line
}

#[derive(Debug, Clone)]
pub enum Exp {
    Uxp(UnaryExp),
    Bxp(BinaryExp),
    Integer(i32),
}

#[derive(Debug)]
pub struct Symbol {
    pub name: String,
    pub ty: SymbolType,
}

use u32 as SymbolID;

#[derive(Debug)]
pub struct SymbolTable {
    pub symbols: HashMap<SymbolID, Symbol>,
}

#[derive(Debug)]
pub enum SymbolType {
    Var,
    Fun,
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

#[derive(Debug)]
pub struct Block {
    pub values: Vec<AstValue>,
}

impl Block {
    pub fn new() -> Self {
        Block { values: Vec::new() }
    }

    pub fn new_with_vec(values: &Vec<AstValue>) -> Self {
        Block {
            values: values.clone(),
        }
    }

    pub fn add(&mut self, value: AstValue) {
        self.values.push(value);
    }
}

impl Exp {
    pub fn parse(&self) -> i32 {
        use utils::*;

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
        }
    }
}

pub(in crate::ast) mod utils {
    // logical and
    pub fn lnd(x: i32, y: i32) -> i32 {
        (x != 0 && y != 0) as i32
    }

    // logical or
    pub fn lor(x: i32, y: i32) -> i32 {
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
}



