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
    Return(Exp),
    End, // ; is the end of a line
}

#[derive(Debug, Clone)]
pub enum Exp {
    Uxp(UnaryExp),
    Bxp(BinaryExp),
    Integer(i32),
}

#[derive(Debug, Clone)]
pub enum UnaryExp {
    Neg(Box<Exp>),
    Not(Box<Exp>),
}

#[derive(Debug, Clone)]
pub enum BinaryExp {
    Add(Oprand),
    Sub(Oprand),
    Mul(Oprand),
    Div(Oprand),
    Mod(Oprand),
    And(Oprand),
    Or(Oprand),
    Eq(Oprand),
    Neq(Oprand),
    Lt(Oprand),
    Lte(Oprand),
    Gt(Oprand),
    Gte(Oprand),
}

#[derive(Debug, Clone)]
pub struct Oprand {
    pub left: Box<Exp>,
    pub right: Box<Exp>,
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
