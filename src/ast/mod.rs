pub(crate) mod exp;

pub(crate) use exp::*;
use koopa::ir::Type;

#[derive(Debug)]
pub struct CompUnit {
    pub items: Vec<GlobalItem>,
}

#[derive(Debug)]
pub enum GlobalItem {
    Decl(Decl),
    Func(FuncDef),
}

// #[derive(Debug)]
// pub enum GlobalDecl {
//     VarDecl(Vec<VarDecl>),
//     ConstDecl(Vec<ConstDecl>),
// }

#[derive(Debug, Clone, Copy)]
pub enum DataType {
    Int,
    Void,
}

#[derive(Debug)]
pub struct FuncDef {
    pub ret_ty: DataType,
    pub ident: String,
    pub params: Vec<FuncParam>,
    pub block: Block,
}

#[derive(Debug)]
pub struct FuncParam {
    pub ty: DataType,
    pub ident: String,
}

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

impl Block {
    pub fn new_with_vec(items: Vec<BlockItem>) -> Self {
        Self { items }
    }
}

impl VarDecl {
    pub fn new(name: String, init: Option<Box<Exp>>) -> Self {
        Self { name, init }
    }
}

impl DataType {
    pub fn into_ty(&self) -> Type {
        match self {
            DataType::Int => Type::get_i32(),
            DataType::Void => Type::get_unit(),
        }
    }

    // pub fn into_pointer_ty(&self) -> Type {
    //     match self {
    //         DataType::Int => Type::get_pointer(Type::get_i32()),
    //         _ => panic!("attempt to get a pointer type of void"),
    //     }
    // }
}
