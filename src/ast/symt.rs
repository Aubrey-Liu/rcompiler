use std::collections::HashMap;

use anyhow::{anyhow, Result};
use koopa::ir::Value;

#[derive(Debug)]
pub enum Symbol {
    ConstVar(i32),
    // val: the allocated position on the stack 
    // init: is the variable initialized with a expression?
    Var { val: Value, init: bool }, 
}

#[derive(Debug)]
pub struct SymbolTable<'input> {
    pub symbols: HashMap<&'input str, Symbol>,
    pub parent_link: Option<Box<SymbolTable<'input>>>,
    pub child_link: Option<Box<SymbolTable<'input>>>,
}

impl<'input> SymbolTable<'input> {
    pub fn insert_var(&mut self, name: &'input str, val: Value, init: bool) -> Result<()> {
        self.get(&name).unwrap_err();
        self.symbols.insert(name, Symbol::Var { val, init: init });

        Ok(())
    }

    pub fn insert_const(&mut self, name: &'input str, init: i32) -> Result<()> {
        self.get(&name).unwrap_err();
        self.symbols.insert(name, Symbol::ConstVar(init));

        Ok(())
    }

    pub fn get_mut(&mut self, name: &'input str) -> Result<&mut Symbol> {
        self.symbols
            .get_mut(name)
            .ok_or(anyhow!("cannot find {} in this scope", name))
    }

    pub fn get(&self, name: &'input str) -> Result<&Symbol> {
        self.symbols
            .get(name)
            .ok_or(anyhow!("cannot find {} in this scope", name))
    }

    pub fn is_global(&self) -> bool {
        self.parent_link.is_none()
    }

    pub fn new() -> Self {
        SymbolTable {
            symbols: HashMap::new(),
            parent_link: None,
            child_link: None,
        }
    }

    pub fn assert_initialized(&self, name: &'input str) {
        if let Symbol::Var { init, .. } = self.symbols.get(name).unwrap() {
            if !init {
                panic!("{} has to be initialized before used", name);
            }
        }
    }

    pub fn get_const_val(&self, name: &'input str) -> i32 {
        if let Ok(Symbol::ConstVar(i)) = self.get(name) {
            return *i;
        }
        panic!("{} has to be a const variable", name);
    }

    pub fn is_var(&self, name: &'input str) -> bool {
        if let Some(Symbol::Var { .. }) = self.symbols.get(name) {
            true
        } else {
            false
        }
    }

    pub fn is_const(&self, name: &'input str) -> bool {
        if let Some(Symbol::ConstVar(_)) = self.symbols.get(name) {
            true
        } else {
            false
        }
    }

    pub fn initialize(&mut self, name: &'input str) {
        if let Symbol::Var { init, .. } = self.get_mut(name).unwrap() {
            *init = true;
        }
    }
}
