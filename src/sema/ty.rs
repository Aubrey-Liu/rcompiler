use super::*;
use crate::ast::{Exp, Type as AstType};
use koopa::ir::Type as IrType;

#[derive(Debug, Clone)]
pub enum Type {
    Int,
    Void,
    Array(Box<Type>, usize),
    #[allow(dead_code)]
    Pointer(Box<Type>),
}

impl Type {
    pub fn infer_from_dims(symbols: &SymbolTable, dims: &[Exp]) -> Self {
        if dims.is_empty() {
            return Self::Int;
        }
        let len = dims.first().unwrap().const_eval(symbols).unwrap();
        let base_ty = Self::infer_from_dims(symbols, &dims[1..]);
        Self::Array(Box::new(base_ty), len as usize)
    }

    pub fn is_compatible(&self, ast_type: &AstType) -> bool {
        match (self, ast_type) {
            (Type::Int, AstType::Int) => true,
            (Type::Array(_, _), AstType::Array) => true,
            _ => false,
        }
    }

    pub fn into_ir_ty(&self) -> IrType {
        match self {
            Self::Int => IrType::get_i32(),
            Self::Array(base_ty, len) => IrType::get_array(base_ty.into_ir_ty(), *len),
            Self::Void => IrType::get_unit(),
            Self::Pointer(base_ty) => IrType::get_pointer(base_ty.into_ir_ty()),
        }
    }
}
