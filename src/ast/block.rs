pub use super::*;

#[derive(Debug)]
pub struct Block {
    pub items: Vec<BlockItem>,
}

#[derive(Debug)]
pub enum BlockItem {
    ConstDecl(Vec<ConstDecl>),
    Decl(Vec<Decl>),
    Stmt(Stmt),
}

#[derive(Debug)]
pub enum Stmt {
    Assign(Assign),
    Exp(Option<Box<Exp>>),
    Return(Option<Box<Exp>>),
    Block(Box<Block>),
    Branch(Branch),
}

#[derive(Debug)]
pub struct Branch {
    pub cond: Box<Exp>,
    pub if_stmt: Box<Stmt>,
    pub el_stmt: Option<Box<Stmt>>,
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

impl Block {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn new_with_vec(items: Vec<BlockItem>) -> Self {
        Self { items }
    }
}

#[derive(Debug)]
pub struct Assign {
    /// Assignment
    pub name: String,
    pub val: Box<Exp>,
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

impl Decl {
    pub fn new_with_init(name: String, init: Box<Exp>) -> Self {
        Self {
            name,
            init: Some(init),
        }
    }

    pub fn new_without_init(name: String) -> Self {
        Self { name, init: None }
    }
}
