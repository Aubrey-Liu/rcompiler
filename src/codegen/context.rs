use std::{cell::Cell, collections::HashMap};

use super::*;

type InstRegistry = HashMap<Value, i32>;

pub struct Context<'i> {
    program: &'i Program,
    functions: HashMap<Function, usize>,
    cur_func: Option<FunctionInfo>,
}

pub struct FunctionInfo {
    id: Function,
    registry: InstRegistry,
    bbs: HashMap<BasicBlock, String>,
    // stack size
    ss: i32,
}

impl<'i> Context<'i> {
    thread_local! {
        static NAMETAG: Cell<u32> = Cell::new(0);
    }

    pub fn new_with_program(program: &'i Program) -> Self {
        Self {
            program,
            functions: HashMap::new(),
            cur_func: None,
        }
    }

    pub fn value_kind(&self, val: Value) -> &ValueKind {
        self.func_data().dfg().value(val).kind()
    }

    pub fn func_data(&self) -> &FunctionData {
        self.program.func(self.cur_func().id())
    }

    pub fn cur_func(&self) -> &FunctionInfo {
        self.cur_func.as_ref().unwrap()
    }

    pub fn cur_func_mut(&mut self) -> &mut FunctionInfo {
        self.cur_func.as_mut().unwrap()
    }

    pub fn set_func(&mut self, func: Function) {
        self.cur_func = Some(FunctionInfo::new(func));
        Self::NAMETAG.with(|id| id.set(0));
    }

    pub fn new_func(&mut self, func: Function) {
        let id = self.functions.len();
        self.functions
            .insert(func, id)
            .map_or((), |_| panic!("redifinition of function"));
    }

    pub fn register_bb(&mut self, bb: BasicBlock) {
        let id = Self::NAMETAG.with(|id| id.replace(id.get() + 1));
        let func_id = self.functions.get(&self.cur_func().id()).unwrap();
        let name = format!(".LBB{}_{}", func_id, id);
        self.cur_func_mut().register_bb(bb, name);
    }

    pub fn is_used(&self, val: Value) -> bool {
        !self.func_data().dfg().value(val).used_by().is_empty()
    }
}

impl FunctionInfo {
    pub fn new(id: Function) -> Self {
        FunctionInfo {
            id,
            registry: InstRegistry::new(),
            bbs: HashMap::new(),
            ss: 0,
        }
    }

    pub fn id(&self) -> Function {
        self.id
    }

    /// Allocated space of the stack of the current function
    pub fn ss(&self) -> i32 {
        self.ss
    }

    pub fn get_offset(&self, val: &Value) -> i32 {
        *self.registry.get(val).unwrap()
    }

    pub fn get_bb_name(&self, bb: &BasicBlock) -> &String {
        self.bbs.get(bb).unwrap()
    }

    pub fn set_ss(&mut self, ss: i32) {
        self.ss = ss;
    }

    pub fn register_var(&mut self, inst: Value, off: i32) {
        self.registry.insert(inst, off);
    }

    /// Record the name of a basic block
    pub fn register_bb(&mut self, bb: BasicBlock, name: String) {
        self.bbs.insert(bb, name);
    }
}
