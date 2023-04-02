use koopa::ir::Type as IrType;
use std::collections::HashMap;

use crate::ast::{AstType, ConstDecl, InitVal, VarDecl};

use super::*;

#[derive(Debug, Clone)]
pub enum Symbol {
    ConstVar(i32),
    Var(bool), // whether it's initialized or not
    ConstArray(Type, Vec<i32>),
    Array(Type, Option<Vec<i32>>),
    Pointer(Type),
    Func(Type, Vec<Type>), // return type and parameter's type
}

#[derive(Debug)]
pub struct SymbolTable {
    pub data: HashMap<String, Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: &str, symbol: Symbol) {
        // unique name for each symbol is guaranteed after the renaming process
        self.data.insert(name.to_owned(), symbol);
    }

    pub fn assign(&mut self, name: &str) {
        if let Some(Symbol::Var(init)) = self.data.get_mut(name) {
            *init = true;
        }
    }

    pub fn get(&self, name: &str) -> &Symbol {
        // use before define is also prevented after the renaming process
        self.data.get(name).unwrap()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.data.contains_key(name)
    }
}

impl Symbol {
    pub fn from_const_decl(value: &ConstDecl) -> Self {
        let dims: Vec<_> = value
            .lval
            .dims
            .iter()
            .map(|d| d.get_i32() as usize)
            .collect();
        let ty = Type::infer_from_dims(&dims);

        match value.ty {
            AstType::Int => match &value.init {
                InitVal::Exp(e) => Self::ConstVar(e.get_i32()),
                InitVal::List(_) => panic!("incompatible initializer type"),
            },
            AstType::Array => {
                let elems = eval_array(&value.init, &ty);
                Self::ConstArray(ty, elems)
            }
            _ => unreachable!(),
        }
    }

    pub fn from_var_decl(value: &VarDecl) -> Self {
        let dims: Vec<_> = value
            .lval
            .dims
            .iter()
            .map(|d| d.get_i32() as usize)
            .collect();
        let ty = Type::infer_from_dims(&dims);

        match value.ty {
            AstType::Int => match &value.init {
                Some(InitVal::Exp(_)) => Self::Var(true),
                Some(InitVal::List(_)) => panic!("incompatible initializer type"),
                None => Self::Var(false),
            },
            AstType::Array => match &value.init {
                Some(InitVal::List(_)) => {
                    let elems = eval_array(value.init.as_ref().unwrap(), &ty);
                    Self::Array(ty, Some(elems))
                }
                None => Self::Array(ty, None),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    pub fn get_var_ir_ty(&self) -> IrType {
        match self {
            Self::Array(ty, _) => ty.get_ir_ty(),
            Self::ConstArray(ty, _) => ty.get_ir_ty(),
            Self::ConstVar(_) | Self::Var(_) => IrType::get_i32(),
            _ => unreachable!(),
        }
    }

    pub fn get_func_ir_ty(&self) -> (IrType, Vec<IrType>) {
        if let Self::Func(ret_ty, param_ty) = self {
            let ret_ty = ret_ty.get_ir_ty();
            let param_ty: Vec<_> = param_ty.iter().map(|p| p.get_ir_ty()).collect();

            (ret_ty, param_ty)
        } else {
            panic!("incompatible symbol type")
        }
    }
}

pub fn eval_array(init: &InitVal, ty: &Type) -> Vec<i32> {
    let mut elems = Vec::new();
    let mut dims: Vec<usize> = Vec::new();
    ty.get_dims(&mut dims);

    let mut acc = 1;
    let boundaries: Vec<_> = dims
        .iter()
        .rev()
        .map(|d| {
            acc *= d;
            acc
        })
        .collect();

    fn fill_array(init: &[InitVal], dims: &[usize], pos: usize, elems: &mut Vec<i32>) -> usize {
        let mut pos = pos;
        let stride = dims
            .iter()
            .rev()
            .find(|&&d| pos % d == 0)
            .expect("invalid initializer");

        for e in init {
            match e {
                InitVal::Exp(e) => {
                    if pos > elems.len() {
                        elems.resize_with(pos, Default::default);
                    }
                    elems.push(e.get_i32());
                    pos += 1;
                }
                InitVal::List(list) => {
                    pos = fill_array(list, &dims[..dims.len() - 1], pos, elems);
                }
            };
        }

        (pos + stride - 1) / stride * stride
    }

    if let InitVal::List(list) = init {
        fill_array(list, &boundaries, 0, &mut elems);
    } else {
        panic!("incompatible initializer type")
    }

    elems
}
