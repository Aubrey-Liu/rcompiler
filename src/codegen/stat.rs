use std::collections::HashMap;

use super::*;

type InstRegistry = HashMap<Value, i32>;

pub struct Context<'i> {
    program: &'i Program,
    cur_func: Option<FunctionInfo>,
}

pub struct FunctionInfo {
    id: Function,
    registry: InstRegistry,
    // stack size
    ss: i32,
}

impl<'i> Context<'i> {
    pub fn new_with_program(program: &'i Program) -> Self {
        Self {
            program,
            cur_func: None,
        }
    }

    pub fn set_func(&mut self, func: Function) {
        self.cur_func = Some(FunctionInfo::new(func))
    }

    pub fn curr_func(&self) -> &FunctionInfo {
        self.cur_func.as_ref().unwrap()
    }

    pub fn curr_func_mut(&mut self) -> &mut FunctionInfo {
        self.cur_func.as_mut().unwrap()
    }

    pub fn func_data(&self) -> &FunctionData {
        self.program.func(self.curr_func().id())
    }

    pub fn value_kind(&self, val: Value) -> &ValueKind {
        self.func_data().dfg().value(val).kind()
    }
}

impl FunctionInfo {
    pub fn new(id: Function) -> Self {
        FunctionInfo {
            id,
            registry: InstRegistry::new(),
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

    pub fn register(&mut self, inst: Value, off: i32) {
        self.registry.insert(inst, off);
    }
}
