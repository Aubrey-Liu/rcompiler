use super::*;

#[derive(Debug)]
pub struct Block {
    pub items: Vec<BlockItem>,
}

#[derive(Debug)]
pub enum BlockItem {
    Decl(Decl),
    Stmt(Stmt),
}

#[derive(Debug)]
pub enum Decl {
    VarDecl(Vec<VarDecl>),
    ConstDecl(Vec<ConstDecl>),
}

#[derive(Debug)]
pub enum Stmt {
    Assign(Assign),
    Block(Box<Block>),
    Branch(Branch),
    Break(Break),
    Continue(Continue),
    Exp(Option<Box<Exp>>),
    While(While),
    Return(Return),
}

#[derive(Debug)]
pub struct Return {
    pub ret_val: Option<Box<Exp>>,
}

#[derive(Debug)]
pub struct Continue;

#[derive(Debug)]
pub struct Break;

#[derive(Debug)]
pub struct While {
    pub cond: Box<Exp>,
    pub stmt: Box<Stmt>,
}

#[derive(Debug)]
pub struct Branch {
    pub cond: Box<Exp>,
    pub if_stmt: Box<Stmt>,
    pub el_stmt: Option<Box<Stmt>>,
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
pub struct VarDecl {
    pub name: String,
    pub init: Option<Box<Exp>>,
}

impl VarDecl {
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
