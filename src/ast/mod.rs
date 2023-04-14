pub(crate) mod exp;
pub(crate) mod visit;

pub(crate) use exp::*;
pub(crate) use visit::*;

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
    pub ret_kind: ExprKind,
    pub ident: String,
    pub params: Vec<FuncParam>,
    pub block: Block,
}

#[derive(Debug)]
pub struct FuncParam {
    pub kind: ExprKind,
    pub ident: String,
    pub dims: Vec<Expr>,
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
    Expr(Option<Expr>),
    While(While),
    Return(Return),
}

#[derive(Debug)]
pub struct Return {
    pub ret_val: Option<Expr>,
}

#[derive(Debug)]
pub struct Continue;

#[derive(Debug)]
pub struct Break;

#[derive(Debug)]
pub struct While {
    pub cond: Expr,
    pub stmt: Box<Stmt>,
}

#[derive(Debug)]
pub struct Branch {
    pub cond: Expr,
    pub if_stmt: Box<Stmt>,
    pub el_stmt: Option<Box<Stmt>>,
}

#[derive(Debug)]
pub struct Assign {
    /// Assignment
    pub lval: LVal,
    pub val: Expr,
}

#[derive(Debug)]
pub struct ConstDecl {
    pub lval: LVal,
    pub init: InitVal,
    pub kind: ExprKind,
}

#[derive(Debug)]
pub struct VarDecl {
    pub lval: LVal,
    pub init: Option<InitVal>,
    pub kind: ExprKind,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Array,
    Int,
    Void,
}

impl Block {
    pub fn new_with_vec(items: Vec<BlockItem>) -> Self {
        Self { items }
    }
}

impl VarDecl {
    pub fn new(lval: LVal, init: Option<InitVal>) -> Self {
        let kind = if !lval.dims.is_empty() {
            ExprKind::Array
        } else {
            ExprKind::Int
        };
        Self { lval, init, kind }
    }
}

impl ConstDecl {
    pub fn new(lval: LVal, init: InitVal) -> Self {
        let kind = if matches!(init, InitVal::List(_)) {
            ExprKind::Array
        } else {
            ExprKind::Int
        };
        Self { lval, init, kind }
    }
}

impl CompUnit {
    pub fn accept<'ast, V: MutVisitor<'ast>>(&'ast mut self, v: &mut V) {
        v.visit_comp_unit(self);
    }
}
