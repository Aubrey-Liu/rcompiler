use std::collections::HashMap;

use anyhow::{anyhow, Ok, Result};
use koopa::ir::Value;

type SymbolTableID = usize;

#[derive(Debug, Clone)]
pub enum Symbol {
    ConstVar(i32),
    // val: the allocated position on the stack
    // init: is the variable initialized with a expression?
    Var { val: Value, init: bool },
}

#[derive(Debug)]
pub struct SymbolTable<'input> {
    #[allow(dead_code)]
    global_node_id: SymbolTableID,
    current_node_id: SymbolTableID,
    nodes: Vec<SymbolTableNode>,
    data: Vec<HashMap<&'input str, Symbol>>,
}

#[derive(Debug, Clone)]
struct SymbolTableNode {
    pub children: Vec<SymbolTableID>,
    pub parent: Option<SymbolTableID>,
}

impl<'input> SymbolTable<'input> {
    pub fn new() -> Self {
        Self {
            global_node_id: 0,
            current_node_id: 1,
            nodes: vec![SymbolTableNode::new(); 2],
            data: vec![HashMap::new(); 2],
        }
    }

    pub fn enter_scope(&mut self) {
        let id = self.allocate_id();
        self.nodes[self.current_node_id].children.push(id);
        self.nodes
            .push(SymbolTableNode::new_as_child(self.current_node_id));
        self.data.push(HashMap::new());
        self.current_node_id = id;
    }

    pub fn exit_scope(&mut self) {
        self.current_node_id = self.nodes[self.current_node_id].parent.unwrap();
    }

    fn allocate_id(&self) -> SymbolTableID {
        self.data.len()
    }

    pub fn get(&self, name: &str) -> Result<&Symbol> {
        let mut id = self.current_node_id;
        loop {
            if let Some(sym) = self.data[id].get(name) {
                return Ok(sym);
            }
            match self.nodes[id].parent {
                Some(i) => id = i,
                None => break,
            }
        }
        Err(anyhow!("{} is not defined in the current scope", name))
    }

    pub fn get_mut(&mut self, name: &str) -> Result<&mut Symbol> {
        let mut id = self.current_node_id;
        loop {
            if self.data[id].contains_key(name) {
                return self.data[id]
                    .get_mut(name)
                    .ok_or(anyhow!("unexpected error"));
            }
            match self.nodes[id].parent {
                Some(i) => id = i,
                None => break,
            }
        }
        Err(anyhow!("{} is not defined in the current scope", name))
    }

    pub fn get_from_var(&self, name: &str) -> Result<Value> {
        self.get(name).and_then(|sym| match sym {
            Symbol::Var { val, .. } => Ok(*val),
            Symbol::ConstVar(_) => Err(anyhow!("{} has to be a variable", name)),
        })
    }

    pub fn get_from_const_var(&self, name: &str) -> Result<i32> {
        self.get(name).and_then(|sym| match sym {
            Symbol::ConstVar(i) => Ok(*i),
            Symbol::Var { .. } => Err(anyhow!("{} has to be a variable", name)),
        })
    }

    pub fn initialize(&mut self, name: &str) -> Result<()> {
        self.get_mut(name).and_then(|sym| {
            if let Symbol::Var { init, .. } = sym {
                *init = true;
                Ok(())
            } else {
                Err(anyhow!("{} has to be a variable", name))
            }
        })
    }

    pub fn insert_var(&mut self, name: &'input str, val: Value, init: bool) -> Result<()> {
        self.data[self.current_node_id]
            .insert(name, Symbol::Var { val, init })
            .map_or(Ok(()), |_| Err(anyhow!("{}: duplicate definition", name)))
    }

    pub fn insert_const_var(&mut self, name: &'input str, init: i32) -> Result<()> {
        self.data[self.current_node_id]
            .insert(name, Symbol::ConstVar(init))
            .map_or(Ok(()), |_| Err(anyhow!("{}: duplicate definition", name)))
    }

    pub fn current_id(&self) -> i32 {
        self.current_node_id as i32
    }
}

impl SymbolTableNode {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            parent: None,
        }
    }

    pub fn new_as_child(parent_id: SymbolTableID) -> Self {
        Self {
            children: Vec::new(),
            parent: Some(parent_id),
        }
    }
}

impl<'input> Default for SymbolTable<'input> {
    fn default() -> Self {
        Self::new()
    }
}
