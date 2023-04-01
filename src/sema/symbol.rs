use koopa::ir::Type as IrType;
use std::cmp::min;
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
    pub fn from_const_decl(value: &ConstDecl, symbols: &SymbolTable) -> Self {
        let ty = Type::infer_from_dims(symbols, &value.lval.dims);
        assert!(ty.is_compatible(&value.ty));

        match value.ty {
            AstType::Int => match &value.init {
                InitVal::Exp(e) => Self::ConstVar(e.const_eval(symbols).unwrap()),
                InitVal::List(_) => panic!("incompatible initializer type"),
            },
            AstType::Array => {
                let elems = init_array(&value.init, &ty);
                Self::ConstArray(ty, elems)
            }
            _ => unreachable!(),
        }
    }

    pub fn from_var_decl(value: &VarDecl, symbols: &SymbolTable) -> Self {
        let ty = Type::infer_from_dims(symbols, &value.lval.dims);
        assert!(ty.is_compatible(&value.ty));

        match value.ty {
            AstType::Int => match &value.init {
                Some(InitVal::Exp(_)) => Self::Var(true),
                Some(InitVal::List(_)) => panic!("incompatible initializer type"),
                None => Self::Var(false),
            },
            AstType::Array => match &value.init {
                Some(InitVal::List(_)) => {
                    let elems = init_array(value.init.as_ref().unwrap(), &ty);
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
            Self::Array(ty, _) => ty.into_ir_ty(),
            Self::ConstArray(ty, _) => ty.into_ir_ty(),
            Self::ConstVar(_) | Self::Var(_) => IrType::get_i32(),
            _ => unreachable!(),
        }
    }

    pub fn get_func_ir_ty(&self) -> (IrType, Vec<IrType>) {
        if let Self::Func(ret_ty, param_ty) = self {
            let ret_ty = ret_ty.into_ir_ty();
            let param_ty: Vec<_> = param_ty.iter().map(|p| p.into_ir_ty()).collect();

            (ret_ty, param_ty)
        } else {
            panic!("incompatible symbol type")
        }
    }
}

pub fn capacity(ty: &Type) -> usize {
    match ty {
        Type::Int => 1,
        Type::Array(base_ty, len) => len * capacity(base_ty),
        _ => unreachable!(),
    }
}

pub fn init_array(init: &InitVal, ty: &Type) -> Vec<i32> {
    let mut elems = vec![0; capacity(ty)];
    let mut dims: Vec<usize> = Vec::new();
    ty.get_dims(&mut dims);
    dims.reverse();

    fn fill_array(
        init: &[InitVal],
        dims: &[usize],
        depth: usize,
        pos: usize,
        elems: &mut Vec<i32>,
    ) -> usize {
        let mut pos = pos;
        let mut depth = depth;
        let mut next_dim = *dims.iter().skip(depth + 1).next().or(Some(&1)).unwrap();
        let mut stride = dims.iter().take(depth + 1).fold(1, |acc, &x| acc * x);

        for e in init {
            match e {
                InitVal::Exp(e) => {
                    elems[pos] = e.get_i32();
                    pos += 1;
                }
                InitVal::List(list) => {
                    if pos % stride != 0 {
                        panic!("invalid list initializer");
                    }
                    let span = min(depth + 2, dims.len());
                    pos = fill_array(list, &dims[0..span], depth, pos, elems);
                }
            };
            if pos % (next_dim * stride) == 0 {
                depth += 1;
                stride *= next_dim;
                next_dim = *dims.iter().skip(depth + 1).next().or(Some(&1)).unwrap();
                if depth >= dims.len() - 1 {
                    break;
                }
            }
        }

        return (pos + stride - 1) / stride * stride;
    }

    if let InitVal::List(list) = init {
        fill_array(list, &dims, 0, 0, &mut elems);
    } else {
        panic!("incompatible initializer type")
    }

    elems
}
