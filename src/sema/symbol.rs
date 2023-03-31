#![allow(dead_code)]
use koopa::ir::Type as IrType;
use std::collections::HashMap;

use crate::ast::{AstType, ConstDecl, FuncDef, InitVal, VarDecl};

use super::*;

#[derive(Debug, Clone)]
pub enum Symbol {
    ConstVar(i32),
    Var(bool), // whether it's initialized or not
    ConstArray(Type, Vec<i32>),
    Array(Type, Option<Vec<i32>>),
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
    pub fn from_func_def(value: &FuncDef) -> Self {
        let ret_ty = match &value.ret_ty {
            AstType::Int => Type::Int,
            AstType::Void => Type::Void,
            _ => unreachable!(),
        };
        let param_tys: Vec<_> = value.params.iter().map(|_| Type::Int).collect();

        Symbol::Func(ret_ty, param_tys)
    }

    pub fn from_const_decl(value: &ConstDecl, symbols: &SymbolTable) -> Self {
        let ty = Type::infer_from_dims(symbols, &value.lval.dims);
        assert!(ty.is_compatible(&value.ty));

        match value.ty {
            AstType::Int => match &value.init {
                InitVal::Exp(e) => Self::ConstVar(e.const_eval(symbols).unwrap()),
                InitVal::List(_) => panic!("incompatible initializer type"),
            },
            AstType::Array => {
                let elems = init_array(symbols, &value.init, &ty);
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
            AstType::Array => {
                let elems = value
                    .init
                    .as_ref()
                    .map(|i| init_array(symbols, &i, &ty))
                    .or(Some(vec![0; capacity(&ty)]));
                Self::Array(ty, elems)
            }
            _ => unreachable!(),
        }
    }

    pub fn get_ty(&self) -> &Type {
        match self {
            Self::Array(ty, _) => ty,
            Self::ConstArray(ty, _) => ty,
            Self::Func(ret_ty, _) => ret_ty,
            Self::ConstVar(_) | Self::Var(_) => &Type::Int,
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

pub fn init_array(symbols: &SymbolTable, init: &InitVal, ty: &Type) -> Vec<i32> {
    let mut elems = Vec::new();
    let mut first_dim = match ty {
        Type::Array(_, len) => *len,
        _ => panic!("incompatible initializer type"),
    };

    if let InitVal::List(list) = init {
        list.iter().for_each(|e| {
            first_dim -= 1;
            match e {
                InitVal::Exp(e) => elems.push(e.const_eval(symbols).unwrap()),
                _ => panic!("multi-dim array is not supported yet"),
            }
        })
    } else {
        panic!("incompatible initializer type")
    }
    // fill all the remaining space with 0
    (0..first_dim).for_each(|_| elems.push(0));

    elems
}
