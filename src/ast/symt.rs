use anyhow::{anyhow, Result};
use koopa::ir::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Symbol {
    ConstVar(i32),
    Var { val: Value, init: bool },
}

pub type SymbolID = String;

#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub symbols: HashMap<String, Symbol>,
    pub parent_link: Option<Box<SymbolTable>>,
    pub child_link: Option<Box<SymbolTable>>,
}

impl SymbolTable {
    pub fn insert_var(&mut self, name: &SymbolID, val: Value, init: bool) -> Result<()> {
        self.get(name).unwrap_err();
        self.symbols.insert(
            name.clone(),
            Symbol::Var {
                val: val,
                init: init,
            },
        );

        Ok(())
    }

    pub fn insert_const(&mut self, name: &SymbolID, init: i32) -> Result<()> {
        self.get(name).unwrap_err();
        self.symbols.insert(name.clone(), Symbol::ConstVar(init));

        Ok(())
    }

    pub fn get_mut(&mut self, name: &SymbolID) -> Result<&mut Symbol> {
        self.symbols
            .get_mut(name)
            .ok_or(anyhow!("Used an undefined variable: {}", name))
    }

    pub fn get(&self, name: &SymbolID) -> Result<&Symbol> {
        self.symbols
            .get(name)
            .ok_or(anyhow!("Used an undefined variable: {}", name))
    }

    pub fn is_global(&self) -> bool {
        self.parent_link.is_none()
    }

    pub fn new() -> SymbolTable {
        SymbolTable {
            symbols: HashMap::new(),
            parent_link: None,
            child_link: None,
        }
    }

    pub fn is_initialized(&self, name: &SymbolID) -> bool {
        match self.symbols.get(name).unwrap() {
            Symbol::ConstVar(_) => true,
            Symbol::Var { init, .. } => *init,
        }
    }

    pub fn is_var(&self, name: &SymbolID) -> bool {
        if let Some(Symbol::Var { .. }) = self.symbols.get(name) {
            true
        } else {
            false
        }
    }

    pub fn is_const(&self, name: &SymbolID) -> bool {
        if let Some(Symbol::ConstVar(_)) = self.symbols.get(name) {
            true
        } else {
            false
        }
    }

    pub fn initialize(&mut self, name: &SymbolID) {
        if let Symbol::Var { init, .. } = self.get_mut(name).unwrap() {
            *init = true;
        }
    }
}
