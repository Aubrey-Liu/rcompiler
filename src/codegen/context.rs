use std::{cell::Cell, collections::HashMap};

use super::*;

pub struct Context<'i> {
    program: &'i Program,
    global_vars: HashMap<Value, usize>,
    functions: HashMap<Function, usize>,
    cur_func: Option<FunctionInfo>,
}

pub struct FunctionInfo {
    id: Function,
    registry: HashMap<Value, i32>,
    params: HashMap<Value, i32>,
    bbs: HashMap<BasicBlock, String>,
    // stack size
    ss: i32,
    is_leaf: bool,
}

impl<'i> Context<'i> {
    thread_local! {
        static NAMETAG: Cell<u32> = Cell::new(0);
    }

    pub fn new_with_program(program: &'i Program) -> Self {
        Self {
            program,
            global_vars: HashMap::new(),
            functions: HashMap::new(),
            cur_func: None,
        }
    }

    pub fn value_kind(&self, val: Value) -> &ValueKind {
        self.cur_func_data().dfg().value(val).kind()
    }

    pub fn global_alloc(&mut self, gen: &mut AsmGenerator, alloc: Value) {
        if let ValueKind::GlobalAlloc(g) = self.program.borrow_value(alloc).kind() {
            g.generate(gen, self, alloc).unwrap();
        } else {
            unreachable!()
        }
    }

    pub fn global_init(&self, init: Value) -> i32 {
        match self.program.borrow_value(init).kind() {
            ValueKind::ZeroInit(_) => 0,
            ValueKind::Integer(i) => i.value(),
            _ => unreachable!(),
        }
    }

    pub fn get_func_name(&'i self, id: Function) -> &'i str {
        self.program.func(id).name()
    }

    pub fn func_data(&self, id: Function) -> &FunctionData {
        self.program.func(id)
    }

    pub fn cur_func_data(&self) -> &FunctionData {
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

    pub fn register_global_var(&mut self, global_var: Value) {
        let id = self.global_vars.len();
        self.global_vars.insert(global_var, id);
    }

    pub fn get_global_var(&self, global_var: &Value) -> usize {
        *self.global_vars.get(global_var).unwrap()
    }

    pub fn is_used(&self, val: Value) -> bool {
        !self.cur_func_data().dfg().value(val).used_by().is_empty()
    }

    pub fn is_global(&self, val: &Value) -> bool {
        self.global_vars.contains_key(val)
    }
}

impl FunctionInfo {
    pub fn new(id: Function) -> Self {
        FunctionInfo {
            id,
            registry: HashMap::new(),
            params: HashMap::new(),
            bbs: HashMap::new(),
            ss: 0,
            is_leaf: false,
        }
    }

    pub fn set_params(&mut self, params: &[Value]) {
        params.iter().enumerate().for_each(|(id, &val)| {
            self.params.insert(val, id as i32);
        });
    }

    pub fn params(&self) -> &HashMap<Value, i32> {
        &self.params
    }

    pub fn id(&self) -> Function {
        self.id
    }

    /// Allocated space of the stack of the current function
    pub fn ss(&self) -> i32 {
        self.ss
    }

    pub fn is_leaf(&self) -> bool {
        self.is_leaf
    }

    pub fn set_is_leaf(&mut self, is_leaf: bool) {
        self.is_leaf = is_leaf;
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
