use std::{collections::HashMap, vec};

use koopa::ir::builder::LocalBuilder;
use koopa::ir::builder_traits::*;

use super::*;
use crate::ast::FuncDef;

#[derive(Debug)]
pub struct ProgramRecorder<'i> {
    symbols: SymbolTable<'i>,
    cur_func: Option<FunctionInfo>,
    loops: Vec<LoopInfo>,
}

#[derive(Debug)]
pub struct FunctionInfo {
    id: Function,
    entry_bb: BasicBlock,
    end_bb: BasicBlock,
    cur_bb: BasicBlock,
    ret_val: Option<Value>,
}

#[derive(Debug)]
pub struct LoopInfo {
    entry: BasicBlock,
    exit: BasicBlock,
}

type SymbolTableID = usize;

#[derive(Debug)]
pub struct SymbolTable<'i> {
    #[allow(dead_code)]
    global_node_id: SymbolTableID,
    current_node_id: SymbolTableID,
    nodes: Vec<SymbolTableNode>,
    data: Vec<HashMap<&'i str, Symbol>>,
    functions: HashMap<&'i str, Function>,
}

#[derive(Debug)]
struct SymbolTableNode {
    pub children: Vec<SymbolTableID>,
    pub parent: Option<SymbolTableID>,
}

#[derive(Debug)]
pub enum Symbol {
    ConstVar(i32),
    Var {
        val: Value, // the allocated position on the stack
        init: bool, // is the variable initialized with a expression?
    },
}

impl<'i> ProgramRecorder<'i> {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            cur_func: None,
            loops: vec![],
        }
    }

    pub fn new_value<'p>(&self, program: &'p mut Program) -> LocalBuilder<'p> {
        program.func_mut(self.func().id()).dfg_mut().new_value()
    }

    pub fn new_func(&mut self, program: &mut Program, func_def: &'i FuncDef) -> Function {
        let params: Vec<(Option<String>, Type)> = func_def
            .params
            .iter()
            .map(|p| (Some(format!("@{}", &p.ident)), p.ty.into_ty()))
            .collect();

        let id = program.new_func(FunctionData::with_param_names(
            format!("@{}", &func_def.ident),
            params,
            func_def.ret_ty.into_ty(),
        ));

        let builder = program.func_mut(id).dfg_mut();
        let entry_bb = builder.new_bb().basic_block(Some("%entry".to_owned()));
        let end_bb = builder.new_bb().basic_block(Some("%end".to_owned()));
        self.cur_func = Some(FunctionInfo {
            id,
            entry_bb,
            end_bb,
            cur_bb: entry_bb,
            ret_val: None,
        });

        id
    }

    pub fn declare_func(
        &mut self,
        program: &mut Program,
        name: &'i str,
        params_ty: Vec<Type>,
        ret_ty: Type,
    ) -> Result<()> {
        let id = program.new_func(FunctionData::new_decl(
            format!("@{}", name),
            params_ty,
            ret_ty,
        ));

        self.insert_func(name, id)
    }

    pub fn func(&self) -> &FunctionInfo {
        self.cur_func.as_ref().unwrap()
    }

    pub fn func_mut(&mut self) -> &mut FunctionInfo {
        self.cur_func.as_mut().unwrap()
    }

    pub fn get_symbol(&self, name: &str) -> Result<&Symbol> {
        self.symbols.get(name)
    }

    pub fn get_func(&self, name: &str) -> Option<&Function> {
        self.symbols.get_func(name)
    }

    pub fn insert_var(&mut self, name: &'i str, val: Value, init: bool) -> Result<()> {
        self.symbols.insert_var(name, val, init)
    }

    pub fn insert_const_var(&mut self, name: &'i str, val: i32) -> Result<()> {
        self.symbols.insert_const_var(name, val)
    }

    pub fn insert_func(&mut self, name: &'i str, id: Function) -> Result<()> {
        self.symbols.insert_func(name, id)
    }

    pub fn initialize(&mut self, name: &str) -> Result<()> {
        self.symbols.initialize(name)
    }

    pub fn is_global(&self) -> bool {
        self.symbols.current_node_id == self.symbols.global_node_id
    }

    pub fn enter_scope(&mut self) {
        self.symbols.enter_scope();
    }

    pub fn exit_scope(&mut self) {
        self.symbols.exit_scope();
    }

    pub fn enter_loop(&mut self, entry: BasicBlock, exit: BasicBlock) {
        self.loops.push(LoopInfo { entry, exit });
    }

    pub fn exit_loop(&mut self) {
        self.loops.pop();
    }

    pub fn inside_loop(&self) -> bool {
        !self.loops.is_empty()
    }

    pub fn loop_entry(&self) -> BasicBlock {
        self.loops.last().unwrap().entry
    }

    pub fn loop_exit(&self) -> BasicBlock {
        self.loops.last().unwrap().exit
    }
}

impl FunctionInfo {
    pub fn new_bb(&self, program: &mut Program, name: &str) -> BasicBlock {
        program
            .func_mut(self.id)
            .dfg_mut()
            .new_bb()
            .basic_block(Some(name.to_owned()))
    }

    pub fn new_anonymous_bb(&self, program: &mut Program) -> BasicBlock {
        program
            .func_mut(self.id)
            .dfg_mut()
            .new_bb()
            .basic_block(None)
    }

    pub fn push_bb(&mut self, program: &mut Program, bb: BasicBlock) {
        program
            .func_mut(self.id)
            .layout_mut()
            .bbs_mut()
            .push_key_back(bb)
            .unwrap();

        self.cur_bb = bb;
    }

    pub fn push_inst(&self, program: &mut Program, inst: Value) {
        program
            .func_mut(self.id)
            .layout_mut()
            .bb_mut(self.cur_bb)
            .insts_mut()
            .push_key_back(inst)
            .unwrap();
    }

    pub fn push_inst_to(&self, program: &mut Program, bb: BasicBlock, inst: Value) {
        program
            .func_mut(self.id)
            .layout_mut()
            .bb_mut(bb)
            .insts_mut()
            .push_key_back(inst)
            .unwrap()
    }

    pub fn set_value_name(&self, program: &mut Program, name: String, val: Value) {
        program
            .func_mut(self.id)
            .dfg_mut()
            .set_value_name(val, Some(name))
    }

    pub fn set_ret_val(&mut self, ret_val: Value) {
        self.ret_val = Some(ret_val);
    }

    pub fn ret_val(&self) -> Option<Value> {
        self.ret_val
    }

    pub fn entry_bb(&self) -> BasicBlock {
        self.entry_bb
    }

    pub fn end_bb(&self) -> BasicBlock {
        self.end_bb
    }

    pub fn id(&self) -> Function {
        self.id
    }
}

impl<'i> SymbolTable<'i> {
    pub fn insert_var(&mut self, name: &'i str, val: Value, init: bool) -> Result<()> {
        self.data[self.current_node_id]
            .insert(name, Symbol::Var { val, init })
            .map_or(Ok(()), |_| Err(anyhow!("redefinition of '{}'", name)))
    }

    pub fn insert_const_var(&mut self, name: &'i str, val: i32) -> Result<()> {
        self.data[self.current_node_id]
            .insert(name, Symbol::ConstVar(val))
            .map_or(Ok(()), |_| Err(anyhow!("redefinition of '{}'", name)))
    }

    pub fn insert_func(&mut self, name: &'i str, id: Function) -> Result<()> {
        self.functions
            .insert(name, id)
            .map_or(Ok(()), |_| Err(anyhow!("redefinition of '{}'", name)))
    }

    pub fn get_func(&self, name: &str) -> Option<&Function> {
        self.functions.get(name)
    }

    pub fn new() -> Self {
        Self {
            global_node_id: 0,
            current_node_id: 0,
            nodes: vec![SymbolTableNode::new()],
            data: vec![HashMap::new()],
            functions: HashMap::new(),
        }
    }

    pub fn enter_scope(&mut self) {
        let id = self.next_id();
        self.nodes[self.current_node_id].children.push(id);
        self.nodes
            .push(SymbolTableNode::new_with_parent(self.current_node_id));
        self.data.push(HashMap::new());
        self.current_node_id = id;
    }

    pub fn exit_scope(&mut self) {
        self.current_node_id = self.nodes[self.current_node_id].parent.unwrap();
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

    fn next_id(&self) -> SymbolTableID {
        self.data.len()
    }
}

impl SymbolTableNode {
    pub fn new() -> Self {
        Self {
            children: vec![],
            parent: None,
        }
    }

    pub fn new_with_parent(parent_id: SymbolTableID) -> Self {
        Self {
            children: vec![],
            parent: Some(parent_id),
        }
    }
}
