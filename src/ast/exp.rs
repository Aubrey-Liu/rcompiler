use koopa::ir::{self, ValueKind};
use koopa::ir::{FunctionData, Value};

use crate::generate::ir::inst_builder;

use super::*;

#[derive(Debug, Clone)]
pub enum Exp {
    Integer(i32),
    LVal(SymbolID, Option<Value>),
    Uxp(UnaryExp),
    Bxp(BinaryExp),
}

#[derive(Debug, Clone)]
pub struct BinaryExp {
    pub op: BinaryOp,
    pub left: Box<Exp>,
    pub right: Box<Exp>,
}

#[derive(Debug, Clone)]
pub struct UnaryExp {
    pub op: UnaryOp,
    pub right: Box<Exp>,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Nop,
    Neg,
    Not,
}

pub trait ConstEval {
    fn const_eval(&self, symt: &SymbolTable) -> i32;
}

impl ConstEval for UnaryExp {
    fn const_eval(&self, symt: &SymbolTable) -> i32 {
        let right = self.right.const_eval(symt);
        eval_unary(self.op, right)
    }
}

impl ConstEval for BinaryExp {
    fn const_eval(&self, symt: &SymbolTable) -> i32 {
        let left = self.left.const_eval(symt);
        let right = self.right.const_eval(symt);
        eval_binary(self.op, left, right)
    }
}

impl ConstEval for Exp {
    fn const_eval(&self, symt: &SymbolTable) -> i32 {
        match self {
            Exp::Integer(i) => *i,
            Exp::Uxp(uxp) => uxp.const_eval(symt),
            Exp::Bxp(bxp) => bxp.const_eval(symt),
            Exp::LVal(name, ..) => {
                if let Ok(Symbol::ConstVar(i)) = symt.get(name) {
                    *i
                } else {
                    panic!("non-const variable used in a const expression");
                }
            }
        }
    }
}

pub trait IntoValue {
    fn into_value(
        &self,
        symt: &SymbolTable,
        func: &mut FunctionData,
        insts: &mut Vec<Value>,
    ) -> Value;
}

impl IntoValue for UnaryExp {
    fn into_value(
        &self,
        symt: &SymbolTable,
        func: &mut FunctionData,
        insts: &mut Vec<Value>,
    ) -> Value {
        let right = self.right.into_value(symt, func, insts);
        
        let rkind = func.dfg().value(right).kind();
        if let ValueKind::Integer(r) = rkind {
            return inst_builder::integer(func, eval_unary(self.op, r.value()));
        }

        let val = match self.op {
            UnaryOp::Nop => right,
            UnaryOp::Neg => inst_builder::neg(func, right),
            UnaryOp::Not => inst_builder::not(func, right),
        };
        insts.push(val);

        val
    }
}

impl IntoValue for BinaryExp {
    fn into_value(
        &self,
        symt: &SymbolTable,
        func: &mut FunctionData,
        insts: &mut Vec<Value>,
    ) -> Value {
        let left = self.left.into_value(symt, func, insts);
        let right = self.right.into_value(symt, func, insts);

        // evaluate when expression is const
        let lkind = func.dfg().value(left).kind();
        let rkind = func.dfg().value(right).kind();
        if let (ValueKind::Integer(l), ValueKind::Integer(r)) = (lkind, rkind) {
            return inst_builder::integer(func, eval_binary(self.op, l.value(), r.value()));
        }

        let val = inst_builder::binary(func, self.op.into(), left, right);
        insts.push(val);

        val
    }
}

impl IntoValue for Exp {
    fn into_value(
        &self,
        symt: &SymbolTable,
        func: &mut FunctionData,
        insts: &mut Vec<Value>,
    ) -> Value {
        match self {
            Exp::Integer(i) => inst_builder::integer(func, *i),
            Exp::Uxp(uxp) => uxp.into_value(symt, func, insts),
            Exp::Bxp(bxp) => bxp.into_value(symt, func, insts),
            Exp::LVal(name, ..) => {
                assert!(symt.is_initialized(name), "used an uninitialized variable");
                match symt.get(name).unwrap() {
                    Symbol::ConstVar(i) => inst_builder::integer(func, *i),
                    Symbol::Var { val, .. } => {
                        let load = inst_builder::load(func, *val);
                        insts.push(load);

                        load
                    }
                }
            }
        }
    }
}

impl Exp {
    pub fn is_const(&self, symt: &SymbolTable) -> bool {
        match self {
            Exp::Integer(_) => true,
            Exp::LVal(name, _) => symt.is_const(name),
            _ => false,
        }
    }
}

impl From<BinaryOp> for ir::BinaryOp {
    fn from(value: BinaryOp) -> Self {
        match value {
            BinaryOp::Add => ir::BinaryOp::Add,
            BinaryOp::Sub => ir::BinaryOp::Sub,
            BinaryOp::Mul => ir::BinaryOp::Mul,
            BinaryOp::Div => ir::BinaryOp::Div,
            BinaryOp::Mod => ir::BinaryOp::Mod,
            BinaryOp::And => ir::BinaryOp::And,
            BinaryOp::Or => ir::BinaryOp::Or,
            BinaryOp::Eq => ir::BinaryOp::Eq,
            BinaryOp::Neq => ir::BinaryOp::NotEq,
            BinaryOp::Lt => ir::BinaryOp::Lt,
            BinaryOp::Lte => ir::BinaryOp::Le,
            BinaryOp::Gt => ir::BinaryOp::Gt,
            BinaryOp::Gte => ir::BinaryOp::Ge,
        }
    }
}
