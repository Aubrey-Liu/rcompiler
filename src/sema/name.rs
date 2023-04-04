use std::collections::{HashMap, HashSet};

use crate::ast::*;

#[derive(Debug)]
pub struct NameManager {
    mapping: Vec<HashMap<String, u32>>,
    pool: HashSet<String>,
}

impl NameManager {
    pub fn new() -> Self {
        NameManager {
            mapping: vec![],
            pool: HashSet::new(),
        }
    }

    pub fn install_lib(&mut self) {
        self.insert_name("getint");
        self.insert_name("getch");
        self.insert_name("getarray");
        self.insert_name("putint");
        self.insert_name("putch");
        self.insert_name("putarray");
        self.insert_name("starttime");
        self.insert_name("stoptime");
    }

    pub fn enter_scope(&mut self) {
        self.mapping.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.mapping.pop();
    }

    pub fn insert_name(&mut self, old_name: &str) {
        let mut name = String::from(old_name);
        let mut possible_suffix = self.get_suffix(old_name);

        while self.pool.contains(&name) {
            possible_suffix += 1;
            name = format!("{}{}", old_name, possible_suffix);
        }
        self.pool.insert(name);

        if self
            .mapping
            .last_mut()
            .unwrap()
            .insert(old_name.to_owned(), possible_suffix)
            .is_some()
        {
            panic!("redifinition of `{}`", old_name);
        }
    }

    pub fn rename(&self, name: &mut String) {
        let suffix = match self.mapping.iter().rev().find_map(|scope| scope.get(name)) {
            Some(name) => name,
            None => panic!("variable used before definition"),
        };
        match suffix {
            0 => {}
            _ => name.push_str(&suffix.to_string()),
        }
    }

    fn get_suffix(&self, name: &str) -> u32 {
        *self
            .mapping
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .unwrap_or(&0)
    }
}

impl<'ast> MutVisitor<'ast> for NameManager {
    fn visit_comp_unit(&mut self, c: &'ast mut CompUnit) {
        self.enter_scope();
        self.install_lib();
        // preserved names
        self.insert_name("entry");
        self.insert_name("end");
        walk_comp_unit(self, c);
        self.exit_scope();
    }

    fn visit_func_def(&mut self, f: &'ast mut FuncDef) {
        self.insert_name(&f.ident);
        self.rename(&mut f.ident);
        self.enter_scope();
        walk_func_def(self, f);
        self.exit_scope();
    }

    fn visit_func_param(&mut self, f: &'ast mut FuncParam) {
        self.insert_name(&f.ident);
        self.rename(&mut f.ident);
    }

    fn visit_block(&mut self, b: &'ast mut Block) {
        self.enter_scope();
        walk_block(self, b);
        self.exit_scope()
    }

    fn visit_const_decl(&mut self, c: &'ast mut ConstDecl) {
        // the order cannot be changed
        self.visit_initval(&mut c.init);
        self.insert_name(&c.lval.ident);
        self.visit_lval(&mut c.lval);
    }

    fn visit_var_decl(&mut self, v: &'ast mut VarDecl) {
        if let Some(init) = &mut v.init {
            self.visit_initval(init);
        }
        self.insert_name(&v.lval.ident);
        self.visit_lval(&mut v.lval);
    }

    fn visit_lval(&mut self, l: &'ast mut LVal) {
        self.rename(&mut l.ident);
        walk_lval(self, l);
    }

    fn visit_call(&mut self, c: &'ast mut Call) {
        self.rename(&mut c.ident);
        walk_call(self, c);
    }
}
