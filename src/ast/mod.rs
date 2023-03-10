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

#[derive(Debug, Clone)]
pub enum AstValue {
    Decl(Vec<Decl>),
    ConstDecl(Vec<ConstDecl>),
    Stmt(Stmt),
    Return(Box<Exp>),
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: SymbolID,
    pub init: Box<Exp>,
}

#[derive(Debug, Clone)]
pub struct Decl {
    pub name: SymbolID,
    pub init: Option<Box<Exp>>,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub name: SymbolID,
    pub val: Box<Exp>,
}

#[derive(Debug)]
pub struct Block {
    pub values: Vec<AstValue>,
}

impl Block {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn new_with_vec(values: &Vec<AstValue>) -> Self {
        Self {
            values: values.clone(),
        }
    }

    pub fn add(&mut self, value: AstValue) {
        self.values.push(value);
    }
}

impl Decl {
    pub fn new_with_init(name: &String, init: &Box<Exp>) -> Self {
        Self {
            name: name.clone(),
            init: Some(init.clone()),
        }
    }

    pub fn new_without_init(name: &String) -> Self {
        Self {
            name: name.clone(),
            init: None,
        }
    }
}
