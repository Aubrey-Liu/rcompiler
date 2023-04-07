use smallvec::{smallvec, SmallVec};
use std::collections::HashMap;

use crate::ast::visit::MutVisitor;
use crate::ast::*;

use super::ty::Type;

#[derive(Debug)]
pub struct SymbolTable {
    pub data: HashMap<String, Type>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: &str, ty: Type) {
        // unique name for each symbol is guaranteed after the renaming process
        self.data.insert(name.to_owned(), ty);
    }

    pub fn get(&self, name: &str) -> &Type {
        // use before define is also prevented after the renaming process
        self.data.get(name).unwrap()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.data.contains_key(name)
    }
}

impl Type {
    pub fn from_const_decl(value: &ConstDecl) -> Self {
        let dims: SmallVec<[_; 4]> = value
            .lval
            .dims
            .iter()
            .map(|d| d.get_i32() as usize)
            .collect();
        Self::infer_from_dims(&dims)
    }

    pub fn from_var_decl(value: &VarDecl) -> Self {
        let dims: SmallVec<[_; 4]> = value
            .lval
            .dims
            .iter()
            .map(|d| d.get_i32() as usize)
            .collect();
        Type::infer_from_dims(&dims)
    }
}

impl<'ast> MutVisitor<'ast> for SymbolTable {
    fn visit_comp_unit(&mut self, c: &'ast mut CompUnit) {
        self.insert("getint", Type::get_func(Type::get_int(), smallvec![]));
        self.insert("getch", Type::get_func(Type::get_int(), smallvec![]));
        self.insert(
            "getarray",
            Type::get_func(
                Type::get_int(),
                smallvec![Type::get_pointer(Type::get_int())],
            ),
        );
        self.insert(
            "putint",
            Type::get_func(Type::get_void(), smallvec![Type::get_int()]),
        );
        self.insert(
            "putch",
            Type::get_func(Type::get_void(), smallvec![Type::get_int()]),
        );
        self.insert(
            "putarray",
            Type::get_func(
                Type::get_void(),
                smallvec![Type::get_int(), Type::get_pointer(Type::get_int())],
            ),
        );
        self.insert("starttime", Type::get_func(Type::get_void(), smallvec![]));
        self.insert("stoptime", Type::get_func(Type::get_void(), smallvec![]));

        walk_comp_unit(self, c);

        if !self.contains("main") {
            panic!("main function is not defined")
        }
    }

    fn visit_func_def(&mut self, f: &'ast mut FuncDef) {
        walk_func_def(self, f);

        let ret_ty = match &f.ret_kind {
            ExprKind::Int => Type::get_int(),
            ExprKind::Void => Type::get_void(),
            _ => unreachable!(),
        };

        let param_tys: SmallVec<[_; 6]> = f
            .params
            .iter()
            .map(|p| self.get(&p.ident).clone())
            .collect();
        self.insert(&f.ident, Type::get_func(ret_ty, param_tys));
    }

    fn visit_func_param(&mut self, f: &'ast mut FuncParam) {
        walk_func_param(self, f);

        let dims: SmallVec<[_; 4]> = f.dims.iter().map(|d| d.get_i32() as usize).collect();
        let ty = match &f.kind {
            ExprKind::Int => Type::get_int(),
            ExprKind::Array => Type::get_pointer(Type::infer_from_dims(&dims)),
            _ => unreachable!(),
        };
        self.insert(&f.ident, ty);
    }

    fn visit_const_decl(&mut self, c: &'ast mut ConstDecl) {
        if matches!(c.kind, ExprKind::Array) {
            walk_const_decl(self, c);
            let ty = Type::from_const_decl(c);
            self.insert(&c.lval.ident, ty);
        }
    }

    fn visit_var_decl(&mut self, v: &'ast mut VarDecl) {
        walk_var_decl(self, v);

        let symbol = Type::from_var_decl(v);
        self.insert(&v.lval.ident, symbol);
    }
}
