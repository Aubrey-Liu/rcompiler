pub use exp::*;
pub(self) use utils::*;

pub mod exp;
mod utils;

#[derive(Debug)]
pub struct CompUnit {
    pub func_def: FuncDef,
}

#[derive(Debug)]
pub struct FuncType(pub String);

#[derive(Debug)]
pub struct FuncDef {
    pub func_type: FuncType,
    pub ident: String,
    pub block: Block,
}

#[derive(Debug)]
pub struct Block {
    pub values: Vec<AstValue>,
}

#[derive(Debug)]
pub enum AstValue {
    Block(Box<Block>),
    ConstDecl(Vec<ConstDecl>),
    Decl(Vec<Decl>),
    Exp(Option<Box<Exp>>),
    Return(Option<Box<Exp>>),
    Stmt(Stmt),
}

#[derive(Debug)]
pub struct ConstDecl {
    pub name: String,
    pub init: Box<Exp>,
}

#[derive(Debug)]
pub struct Decl {
    pub name: String,
    pub init: Option<Box<Exp>>,
}

#[derive(Debug)]
pub struct Stmt {
    pub name: String,
    pub val: Box<Exp>,
}

impl Block {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn new_with_vec(values: Vec<AstValue>) -> Self {
        Self { values }
    }

    pub fn add(&mut self, value: AstValue) {
        self.values.push(value);
    }
}

impl Decl {
    pub fn new_with_init(name: String, init: Box<Exp>) -> Self {
        Self {
            name,
            init: Some(init),
        }
    }

    pub fn new_without_init(name: String) -> Self {
        Self {
            name,
            init: None,
        }
    }
}
