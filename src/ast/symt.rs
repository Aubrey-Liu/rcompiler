use super::exp::*;
use anyhow::{anyhow, Ok, Result};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Symbol {
    ConstVar(i32),
    Var(Box<Exp>),
}

pub type SymbolID = String;

#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub symbols: HashMap<String, Symbol>,
    pub parent_link: Option<Box<SymbolTable>>,
    pub child_link: Option<Box<SymbolTable>>,
}

impl SymbolTable {
    pub fn insert_const(&mut self, name: &String, attr: i32) -> Result<()> {
        if self.symbols.contains_key(name) {
            return Err(anyhow!("duplicate definition"));
        }
        self.symbols
            .insert(name.clone(), Symbol::ConstVar(attr));

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

    pub fn contains_name(&self, name: &SymbolID) -> bool {
        self.symbols.contains_key(name)
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
}
