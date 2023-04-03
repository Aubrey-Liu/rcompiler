use koopa::ir::Type as IrType;
use std::collections::HashMap;

use crate::ast::visit::MutVisitor;
use crate::ast::*;
use crate::walk_list;

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

        match value.kind {
            ExpKind::Int => match &value.init {
                InitVal::Exp(e) => Self::ConstVar(e.get_i32()),
                InitVal::List(_) => panic!("incompatible initializer type"),
            },
            ExpKind::Array => {
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

        match value.kind {
            ExpKind::Int => match &value.init {
                Some(InitVal::Exp(_)) => Self::Var(true),
                Some(InitVal::List(_)) => panic!("incompatible initializer type"),
                None => Self::Var(false),
            },
            ExpKind::Array => match &value.init {
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

    pub fn get_var_ty(&self) -> Type {
        match self {
            Self::Array(ty, _) => ty.clone(),
            Self::ConstArray(ty, _) => ty.clone(),
            Self::ConstVar(_) | Self::Var(_) => Type::Int,
            Self::Pointer(ty) => ty.clone(),
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
        let (idx, stride) = dims
            .iter()
            .rev()
            .enumerate()
            .find(|&(_, d)| pos % d == 0)
            .expect("invalid initializer");
        let mut pos = pos;
        let next_pos = pos + stride;
        let base_dim = dims.len() - idx - 1;

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
                    pos = fill_array(list, &dims[0..base_dim], pos, elems);
                }
            };
        }

        next_pos
    }

    if let InitVal::List(list) = init {
        fill_array(list, &boundaries, 0, &mut elems);
    } else {
        panic!("incompatible initializer type")
    }

    elems
}

impl<'ast> MutVisitor<'ast> for SymbolTable {
    fn visit_comp_unit(&mut self, c: &'ast mut CompUnit) {
        self.insert("getint", Symbol::Func(Type::Int, vec![]));
        self.insert("getch", Symbol::Func(Type::Int, vec![]));
        self.insert(
            "getarray",
            Symbol::Func(Type::Int, vec![Type::Pointer(Box::new(Type::Int))]),
        );
        self.insert("putint", Symbol::Func(Type::Void, vec![Type::Int]));
        self.insert("putch", Symbol::Func(Type::Void, vec![Type::Int]));
        self.insert(
            "putarray",
            Symbol::Func(
                Type::Void,
                vec![Type::Int, Type::Pointer(Box::new(Type::Int))],
            ),
        );
        self.insert("starttime", Symbol::Func(Type::Void, vec![]));
        self.insert("stoptime", Symbol::Func(Type::Void, vec![]));

        walk_list!(self, visit_global_item, &mut c.items);

        if !self.contains("main") {
            panic!("main function is not defined")
        }
    }

    fn visit_func_def(&mut self, f: &'ast mut FuncDef) {
        walk_list!(self, visit_func_param, &mut f.params);
        self.visit_block(&mut f.block);

        let ret_ty = match &f.ret_kind {
            ExpKind::Int => Type::Int,
            ExpKind::Void => Type::Void,
            _ => unreachable!(),
        };

        let param_tys: Vec<_> = f
            .params
            .iter()
            .map(|p| self.get(&p.ident).get_var_ty())
            .collect();
        self.insert(&f.ident, Symbol::Func(ret_ty, param_tys));
    }

    fn visit_func_param(&mut self, f: &'ast mut FuncParam) {
        walk_list!(self, visit_exp, &mut f.dims);

        let dims: Vec<_> = f.dims.iter().map(|d| d.get_i32() as usize).collect();
        let ty = match &f.kind {
            ExpKind::Int => Type::Int,
            ExpKind::Array => Type::Pointer(Box::new(Type::infer_from_dims(&dims))),
            _ => unreachable!(),
        };
        let symbol = match &ty {
            Type::Int => Symbol::Var(true),
            Type::Pointer(_) => Symbol::Pointer(ty.clone()),
            _ => unreachable!(),
        };
        self.insert(&f.ident, symbol);
    }

    fn visit_const_decl(&mut self, c: &'ast mut ConstDecl) {
        self.visit_initval(&mut c.init);
        self.visit_lval(&mut c.lval);

        let symbol = Symbol::from_const_decl(c);
        self.insert(&c.lval.ident, symbol);
    }

    fn visit_var_decl(&mut self, v: &'ast mut VarDecl) {
        if let Some(init) = &mut v.init {
            self.visit_initval(init);
        }
        self.visit_lval(&mut v.lval);

        let symbol = Symbol::from_var_decl(v);
        self.insert(&v.lval.ident, symbol);
    }

    fn visit_assign(&mut self, a: &'ast mut Assign) {
        self.visit_exp(&mut a.val);
        self.visit_lval(&mut a.lval);
        self.assign(&a.lval.ident);
    }

    fn visit_exp(&mut self, e: &'ast mut Exp) {
        if let Some(i) = e.const_eval(self) {
            *e = Exp::Integer(i);
        }

        match e {
            Exp::Bxp(bxp) => self.visit_binary_exp(bxp),
            Exp::Uxp(uxp) => self.visit_unary_exp(uxp),
            Exp::LVal(lval) => match self.get(&lval.ident) {
                Symbol::ConstVar(i) => *e = Exp::Integer(*i),
                _ => self.visit_lval(lval),
            },
            Exp::Integer(_) => {}
            _ => unreachable!(),
        }
    }
}
