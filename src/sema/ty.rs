use koopa::ir::{Type as IrType, TypeKind as IrTypeKind};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Type(Rc<TypeKind>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeKind {
    Void,
    Integer,
    Array(Type, usize),
    Pointer(Type),
    Func(Type, Vec<Type>),
}

impl Type {
    thread_local! {
        static POOL: RefCell<HashMap<TypeKind, Type>> = RefCell::new(HashMap::new());
    }

    pub fn get(kind: TypeKind) -> Type {
        Self::POOL.with(|p| {
            let mut pool = p.borrow_mut();
            pool.get(&kind).cloned().unwrap_or_else(|| {
                let ty = Self(Rc::new(kind.clone()));
                pool.insert(kind, ty.clone());

                ty
            })
        })
    }

    pub fn get_void() -> Type {
        Self::get(TypeKind::Void)
    }

    pub fn get_int() -> Type {
        Self::get(TypeKind::Integer)
    }

    pub fn get_array(base_ty: Type, len: usize) -> Type {
        Self::get(TypeKind::Array(base_ty, len))
    }

    pub fn get_pointer(base_ty: Type) -> Type {
        Self::get(TypeKind::Pointer(base_ty))
    }

    pub fn get_func(ret_ty: Type, param_tys: Vec<Type>) -> Type {
        Self::get(TypeKind::Func(ret_ty, param_tys))
    }

    pub fn kind(&self) -> &TypeKind {
        &self.0
    }

    pub fn size(&self) -> usize {
        match self.kind() {
            TypeKind::Integer => 1,
            TypeKind::Array(base_ty, len) => len * base_ty.size(),
            _ => unreachable!(),
        }
    }

    pub fn get_ir_ty(&self) -> IrType {
        match self.kind() {
            TypeKind::Integer => IrType::get_i32(),
            TypeKind::Array(base_ty, len) => IrType::get_array(base_ty.get_ir_ty(), *len),
            TypeKind::Void => IrType::get_unit(),
            TypeKind::Pointer(base_ty) => IrType::get_pointer(base_ty.get_ir_ty()),
            TypeKind::Func(ret_ty, param_tys) => {
                let param_ir_tys: Vec<_> = param_tys.iter().map(|t| t.get_ir_ty()).collect();
                IrType::get(IrTypeKind::Function(param_ir_tys, ret_ty.get_ir_ty()))
            }
        }
    }

    pub fn infer_from_dims(dims: &[usize]) -> Self {
        if dims.is_empty() {
            return Self::get_int();
        }
        let len = *dims.first().unwrap();
        let base_ty = Self::infer_from_dims(&dims[1..]);
        Self::get_array(base_ty, len)
    }

    pub fn get_dims(&self, dims: &mut Vec<usize>) {
        match self.kind() {
            TypeKind::Array(base_ty, len) => {
                dims.push(*len);
                base_ty.get_dims(dims);
            }
            TypeKind::Integer => {}
            _ => unreachable!(),
        }
    }
}
