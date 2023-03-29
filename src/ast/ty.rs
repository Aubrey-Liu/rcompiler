use super::*;
use koopa::ir::Type as IrType;

#[derive(Debug)]
pub enum Type {
    Array(Box<Type>),
    Int,
    Void,
}

impl Type {
    pub fn into_ty(&self) -> IrType {
        match self {
            Self::Int => IrType::get_i32(),
            Self::Void => IrType::get_unit(),
            _ => todo!(),
        }
    }

    pub fn new_from_dims(dims: &[Exp]) -> Self {
        if dims.is_empty() {
            return Self::Int;
        }
        let sub_ty = Self::new_from_dims(&dims[1..]);
        Self::Array(Box::new(sub_ty))
    }
}
