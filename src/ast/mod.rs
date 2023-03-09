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
    pub init: Box<Exp>,
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
