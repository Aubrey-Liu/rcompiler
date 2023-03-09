use std::collections::HashMap;
use super::exp::*;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: SymbolType,
}

pub type SymbolID = u32;

#[derive(Debug, Clone)]
pub struct SymbolTable {
    // todo: make fields private
    pub names: HashMap<SymbolID, String>,
    pub symbols: HashMap<String, Symbol>,
    pub parent_link: Option<Box<SymbolTable>>,
    pub id: SymbolID,
}

#[derive(Debug, Clone)]
pub enum SymbolType {
    Var { ty: VarType, value: Box<Exp> },
}

#[derive(Debug, Clone)]
pub enum VarType {
    Int,
}
