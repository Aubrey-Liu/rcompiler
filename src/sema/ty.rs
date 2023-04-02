use koopa::ir::Type as IrType;

#[derive(Debug, Clone)]
pub enum Type {
    Int,
    Void,
    Array(Box<Type>, usize),
    Pointer(Box<Type>),
}

impl Type {
    pub fn infer_from_dims(dims: &[usize]) -> Self {
        if dims.is_empty() {
            return Self::Int;
        }
        let len = *dims.first().unwrap();
        let base_ty = Self::infer_from_dims(&dims[1..]);
        Self::Array(Box::new(base_ty), len)
    }

    pub fn get_ir_ty(&self) -> IrType {
        match self {
            Self::Int => IrType::get_i32(),
            Self::Array(base_ty, len) => IrType::get_array(base_ty.get_ir_ty(), *len),
            Self::Void => IrType::get_unit(),
            Self::Pointer(base_ty) => IrType::get_pointer(base_ty.get_ir_ty()),
        }
    }

    pub fn get_dims(&self, dims: &mut Vec<usize>) {
        match self {
            Self::Array(base_ty, len) => {
                dims.push(*len);
                base_ty.get_dims(dims);
            }
            Self::Int => {}
            _ => unreachable!(),
        }
    }
}
