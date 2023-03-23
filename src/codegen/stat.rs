use std::collections::HashMap;

use super::*;

type InstRegistry = HashMap<Value, i32>;

pub struct ProgramStat<'p> {
    program: &'p Program,
    func_stat: Option<FunctionStat>,
}

pub struct FunctionStat {
    func: Function,
    registry: InstRegistry,
    // stack size
    ss: i32,
}

impl<'p> ProgramStat<'p> {
    pub fn new_with_program(program: &'p Program) -> Self {
        Self {
            program,
            func_stat: None,
        }
    }

    pub fn set_func(&mut self, func: Function) {
        self.func_stat = Some(FunctionStat::new(func))
    }

    pub fn curr_func(&self) -> &FunctionStat {
        self.func_stat.as_ref().unwrap()
    }

    pub fn curr_func_mut(&mut self) -> &mut FunctionStat {
        self.func_stat.as_mut().unwrap()
    }

    pub fn func_data(&self) -> &FunctionData {
        self.program.func(self.curr_func().func())
    }

    pub fn value_kind(&self, val: Value) -> &ValueKind {
        self.func_data().dfg().value(val).kind()
    }
}

impl FunctionStat {
    pub fn new(func: Function) -> Self {
        FunctionStat {
            func,
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

    pub fn func(&self) -> Function {
        self.func
    }

    pub fn register(&mut self, inst: Value, off: i32) {
        self.registry.insert(inst, off);
    }
}
