pub use exp::*;
pub use symt::*;
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
    ConstDecl(SymbolID, Exp),
    Return(Exp),
    End, // ; is the end of a line
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
