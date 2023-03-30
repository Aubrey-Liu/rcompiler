use std::collections::{HashMap, HashSet};

use crate::ast::*;

pub trait Renamer {
    fn rename(&mut self, manager: &mut NameManager);
}

impl Renamer for CompUnit {
    fn rename(&mut self, manager: &mut NameManager) {
        manager.install_lib();
        self.items.iter_mut().for_each(|item| item.rename(manager));
    }
}

impl Renamer for GlobalItem {
    fn rename(&mut self, manager: &mut NameManager) {
        match self {
            Self::Decl(i) => i.rename(manager),
            Self::Func(i) => i.rename(manager),
        }
    }
}

impl Renamer for FuncDef {
    fn rename(&mut self, manager: &mut NameManager) {
        manager.insert_name(&self.ident);
        manager.rename(&mut self.ident);

        manager.enter_scope();
        self.params
            .iter_mut()
            .for_each(|param| param.rename(manager));
        self.block.rename(manager);
        manager.exit_scope();
    }
}

impl Renamer for FuncParam {
    fn rename(&mut self, manager: &mut NameManager) {
        manager.insert_name(&self.ident);
        manager.rename(&mut self.ident);
    }
}

impl Renamer for Block {
    fn rename(&mut self, manager: &mut NameManager) {
        manager.enter_scope();
        self.items.iter_mut().for_each(|item| item.rename(manager));
        manager.exit_scope();
    }
}

impl Renamer for BlockItem {
    fn rename(&mut self, manager: &mut NameManager) {
        match self {
            Self::Decl(i) => i.rename(manager),
            Self::Stmt(i) => i.rename(manager),
        }
    }
}

impl Renamer for Decl {
    fn rename(&mut self, manager: &mut NameManager) {
        match self {
            Self::ConstDecl(d) => d.iter_mut().for_each(|d| d.rename(manager)),
            Self::VarDecl(d) => d.iter_mut().for_each(|d| d.rename(manager)),
        }
    }
}

impl Renamer for Stmt {
    fn rename(&mut self, manager: &mut NameManager) {
        match self {
            Self::Assign(s) => s.rename(manager),
            Self::Block(s) => s.rename(manager),
            Self::Branch(s) => s.rename(manager),
            Self::Exp(s) => s.as_mut().map_or((), |s| s.rename(manager)),
            Self::Return(s) => s.rename(manager),
            Self::While(s) => s.rename(manager),
            _ => {}
        }
    }
}

impl Renamer for Return {
    fn rename(&mut self, manager: &mut NameManager) {
        if let Some(exp) = &mut self.ret_val {
            exp.rename(manager)
        }
    }
}

impl Renamer for While {
    fn rename(&mut self, manager: &mut NameManager) {
        self.cond.rename(manager);
        self.stmt.rename(manager);
    }
}

impl Renamer for Branch {
    fn rename(&mut self, manager: &mut NameManager) {
        self.cond.rename(manager);
        self.if_stmt.rename(manager);
        if let Some(s) = &mut self.el_stmt {
            s.rename(manager);
        }
    }
}

impl Renamer for Assign {
    fn rename(&mut self, manager: &mut NameManager) {
        self.val.rename(manager);
        manager.rename(&mut self.lval.ident);
    }
}

impl Renamer for ConstDecl {
    fn rename(&mut self, manager: &mut NameManager) {
        manager.insert_name(&self.lval.ident);
        self.lval.rename(manager);
    }
}

impl Renamer for VarDecl {
    fn rename(&mut self, manager: &mut NameManager) {
        manager.insert_name(&self.lval.ident);
        self.lval.rename(manager);
    }
}

impl Renamer for LVal {
    fn rename(&mut self, manager: &mut NameManager) {
        manager.rename(&mut self.ident);
    }
}

impl Renamer for Exp {
    fn rename(&mut self, manager: &mut NameManager) {
        match self {
            Self::Bxp(e) => e.rename(manager),
            Self::Uxp(e) => e.rename(manager),
            Self::LVal(e) => e.rename(manager),
            Self::Error => panic!("expected an expression"),
            _ => {}
        }
    }
}

impl Renamer for BinaryExp {
    fn rename(&mut self, manager: &mut NameManager) {
        self.lhs.rename(manager);
        self.rhs.rename(manager);
    }
}

impl Renamer for UnaryExp {
    fn rename(&mut self, manager: &mut NameManager) {
        match self {
            Self::Unary(_, e) => e.rename(manager),
            Self::Call(e) => e.rename(manager),
        }
    }
}

impl Renamer for Call {
    fn rename(&mut self, manager: &mut NameManager) {
        self.args.iter_mut().for_each(|arg| arg.rename(manager));
        manager.rename(&mut self.func_id);
    }
}

#[derive(Debug)]
pub struct NameManager {
    mapping: Vec<HashMap<String, u32>>,
    pool: HashSet<String>,
}

impl NameManager {
    pub fn new() -> Self {
        NameManager {
            mapping: vec![HashMap::new()],
            pool: HashSet::new(),
        }
    }

    pub fn install_lib(&mut self) {
        self.mapping.last_mut().unwrap().extend([
            ("getint".to_owned(), 0),
            ("getch".to_owned(), 0),
            ("getarray".to_owned(), 0),
            ("putint".to_owned(), 0),
            ("putch".to_owned(), 0),
            ("putarray".to_owned(), 0),
            ("starttime".to_owned(), 0),
            ("stoptime".to_owned(), 0),
        ]);
    }

    pub fn enter_scope(&mut self) {
        self.mapping.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.mapping.pop();
    }

    pub fn insert_name(&mut self, old_name: &String) {
        let mut name = String::from(old_name);
        let mut possible_suffix = self.get_suffix(&old_name);

        while self.pool.contains(&name) {
            possible_suffix += 1;
            name = String::from(old_name) + &possible_suffix.to_string();
        }
        self.pool.insert(name);

        match self
            .mapping
            .last_mut()
            .unwrap()
            .insert(old_name.clone(), possible_suffix)
        {
            Some(_) => panic!("redifinition of `{}`", old_name),
            None => {}
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
            .or(Some(&0))
            .unwrap()
    }
}
