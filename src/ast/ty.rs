use koopa::ir::Type as IrType;

#[derive(Debug, Clone)]
pub enum Type {
    Pointer(Box<Type>),
    Int,
    Void,
}

impl Type {
    pub fn into_ty(&self) -> IrType {
        match self {
            Type::Int => IrType::get_i32(),
            Type::Void => IrType::get_unit(),
            _ => todo!(),
        }
    }
}
