use std::rc::Rc;

pub use exp::*;
pub use symt::*;
pub(self) use utils::*;

pub mod exp;
pub mod symt;
mod utils;

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

#[derive(Debug)]
pub enum AstValue {
    Decl(Vec<Decl>),
    ConstDecl(Vec<ConstDecl>),
    Stmt(Stmt),
    Return(Rc<Exp>),
}

#[derive(Debug)]
pub struct ConstDecl {
    pub name: String,
    pub init: Rc<Exp>,
}

#[derive(Debug)]
pub struct Decl {
    pub name: String,
    pub init: Option<Rc<Exp>>,
}

#[derive(Debug)]
pub struct Stmt {
    pub name: String,
    pub val: Rc<Exp>,
}

#[derive(Debug)]
pub struct Block {
    pub values: Vec<AstValue>,
}

impl Block {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn new_with_vec(values: Vec<AstValue>) -> Self {
        Self { values: values }
    }

    pub fn add(&mut self, value: AstValue) {
        self.values.push(value);
    }
}

impl Decl {
    pub fn new_with_init(name: String, init: Rc<Exp>) -> Self {
        Self {
            name: name,
            init: Some(init),
        }
    }

    pub fn new_without_init(name: String) -> Self {
        Self {
            name: name,
            init: None,
        }
    }
}
