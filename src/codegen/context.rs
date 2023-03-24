use std::{cell::Cell, collections::HashMap};

use super::*;

type InstRegistry = HashMap<Value, i32>;

pub struct Context<'i> {
    program: &'i Program,
    cur_func: Option<FunctionInfo>,
}

pub struct FunctionInfo {
    id: Function,
    registry: InstRegistry,
    names: HashMap<BasicBlock, String>,
    // stack size
    ss: i32,
}

impl<'i> Context<'i> {
    thread_local! {static NAMETAG: Cell<u32> = Cell::default(); }

    pub fn new_with_program(program: &'i Program) -> Self {
        Self {
            program,
            cur_func: None,
        }
    }

    pub fn set_func(&mut self, func: Function) {
        self.cur_func = Some(FunctionInfo::new(func))
    }

    pub fn cur_func(&self) -> &FunctionInfo {
        self.cur_func.as_ref().unwrap()
    }

    pub fn cur_func_mut(&mut self) -> &mut FunctionInfo {
        self.cur_func.as_mut().unwrap()
    }

    pub fn func_data(&self) -> &FunctionData {
        self.program.func(self.cur_func().id())
    }

    pub fn value_kind(&self, val: Value) -> &ValueKind {
        self.func_data().dfg().value(val).kind()
    }

    pub fn register_bb(&mut self, bb: BasicBlock) {
        let id = Self::NAMETAG.with(|id| id.replace(id.get() + 1));
        let name = match self.func_data().dfg().bb(bb).name() {
            Some(name) => format!(".L{}_{}", id, &name[1..]),
            None => format!(".L{}", id),
        };
        self.cur_func_mut().register_bb(bb, name);
    }
}

impl FunctionInfo {
    pub fn new(id: Function) -> Self {
        FunctionInfo {
            id,
            registry: InstRegistry::new(),
            names: HashMap::new(),
            ss: 0,
        }
    }

    pub fn set_ss(&mut self, ss: i32) {
        self.ss = ss;
    }

    pub fn get_offset(&self, val: &Value) -> i32 {
        *self.registry.get(val).unwrap()
    }

    pub fn ss(&self) -> i32 {
        self.ss
    }

    pub fn id(&self) -> Function {
        self.id
    }

    pub fn register_var(&mut self, inst: Value, off: i32) {
        self.registry.insert(inst, off);
    }

    pub fn register_bb(&mut self, bb: BasicBlock, name: String) {
        self.names.insert(bb, name);
    }

    pub fn get_bb_name(&self, bb: BasicBlock) -> &String {
        self.names.get(&bb).unwrap()
    }
}