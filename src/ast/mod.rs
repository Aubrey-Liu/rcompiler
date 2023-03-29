pub(crate) mod exp;
pub(crate) mod ty;

pub(crate) use exp::*;
pub(crate) use ty::*;

#[derive(Debug)]
pub struct CompUnit {
    pub items: Vec<GlobalItem>,
}

#[derive(Debug)]
pub enum GlobalItem {
    Decl(Decl),
    Func(FuncDef),
}

#[derive(Debug)]
pub struct FuncDef {
    pub ret_ty: Type,
    pub ident: String,
    pub params: Vec<FuncParam>,
    pub block: Block,
}

#[derive(Debug)]
pub struct FuncParam {
    pub ty: Type,
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
    Block(Block),
    Branch(Branch),
    Break(Break),
    Continue(Continue),
    Exp(Option<Exp>),
    While(While),
    Return(Return),
}

#[derive(Debug)]
pub struct Return {
    pub ret_val: Option<Exp>,
}

#[derive(Debug)]
pub struct Continue;

#[derive(Debug)]
pub struct Break;

#[derive(Debug)]
pub struct While {
    pub cond: Exp,
    pub stmt: Box<Stmt>,
}

#[derive(Debug)]
pub struct Branch {
    pub cond: Exp,
    pub if_stmt: Box<Stmt>,
    pub el_stmt: Option<Box<Stmt>>,
}

#[derive(Debug)]
pub struct Assign {
    /// Assignment
    pub lval: LVal,
    pub val: Exp,
}

#[derive(Debug)]
pub struct ConstDecl {
    pub lval: LVal,
    pub init: InitVal,
    pub ty: Type,
}

#[derive(Debug)]
pub struct VarDecl {
    pub lval: LVal,
    pub init: Option<InitVal>,
    pub ty: Type,
}

impl Block {
    pub fn new_with_vec(items: Vec<BlockItem>) -> Self {
        Self { items }
    }
}

impl VarDecl {
    pub fn new(lval: LVal, init: Option<InitVal>) -> Self {
        let ty = Type::new_from_dims(&lval.dims);
        Self { lval, init, ty }
    }
}

impl ConstDecl {
    pub fn new(lval: LVal, init: InitVal) -> Self {
        let ty = Type::new_from_dims(&lval.dims);
        Self { lval, init, ty }
    }
}
