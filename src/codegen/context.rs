use std::{
    cell::{Cell, Ref},
    collections::HashMap,
};

use koopa::ir::entities::ValueData;
use lazy_static_include::lazy_static::lazy_static;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegID(usize);

#[derive(Debug, Clone, Copy)]
pub enum Place {
    #[allow(dead_code)]
    Reg(RegID),
    Mem(i32),
}

pub struct LocalValue {
    place: Place,
    is_pointer: bool,
}

pub struct Context<'i> {
    program: &'i Program,
    global_values: HashMap<Value, String>,
    functions: HashMap<Function, usize>,
    cur_func: Option<FunctionInfo>,
}

pub struct FunctionInfo {
    id: Function,
    local_values: HashMap<Value, LocalValue>,
    blocks: HashMap<BasicBlock, String>,
    ss: i32, // stack size
    is_leaf: bool,
}

impl<'i> Context<'i> {
    thread_local! {
        static NAMETAG: Cell<u32> = Cell::new(0);
    }

    pub fn new_with_program(program: &'i Program) -> Self {
        Self {
            program,
            global_values: HashMap::new(),
            functions: HashMap::new(),
            cur_func: None,
        }
    }

    pub fn value_kind(&self, val: Value) -> ValueKind {
        if self.is_global(val) {
            self.program.borrow_value(val).kind().clone()
        } else {
            self.value_data(val).kind().clone()
        }
    }

    pub fn value_ty(&self, val: Value) -> Type {
        if self.is_global(val) {
            self.global_value_data(val).ty().clone()
        } else {
            self.value_data(val).ty().clone()
        }
    }

    pub fn value_data(&self, val: Value) -> &ValueData {
        self.cur_func_data().dfg().value(val)
    }

    pub fn global_value_data(&self, val: Value) -> Ref<ValueData> {
        self.program.borrow_value(val)
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

    pub fn register_global_var(&mut self, global_var: Value, name: String) {
        self.global_values.insert(global_var, name);
    }

    pub fn get_global_var(&self, global_var: &Value) -> &String {
        self.global_values.get(global_var).unwrap()
    }

    pub fn is_global(&self, val: Value) -> bool {
        self.cur_func.is_none() || self.global_values.contains_key(&val)
    }

    /// If a value is not an alloc and its type is pointer, then return true
    pub fn is_pointer(&self, val: Value) -> bool {
        self.cur_func()
            .local_values
            .get(&val)
            .map(|v| v.is_pointer)
            .unwrap_or(false)
    }
}

impl FunctionInfo {
    pub fn new(id: Function) -> Self {
        FunctionInfo {
            id,
            local_values: HashMap::new(),
            blocks: HashMap::new(),
            ss: 0,
            is_leaf: false,
        }
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

    pub fn get_local_place(&self, val: Value) -> Place {
        self.local_values.get(&val).unwrap().place
    }

    pub fn get_offset(&self, val: &Value) -> i32 {
        if let Place::Mem(offset) = self.local_values.get(val).unwrap().place {
            offset
        } else {
            unreachable!()
        }
    }

    pub fn get_bb_name(&self, bb: &BasicBlock) -> &String {
        self.blocks.get(bb).unwrap()
    }

    pub fn set_ss(&mut self, ss: i32) {
        self.ss = ss;
    }

    pub fn spill_to_mem(&mut self, val: Value, offset: i32, is_pointer: bool) {
        self.local_values.insert(
            val,
            LocalValue {
                place: Place::Mem(offset),
                is_pointer,
            },
        );
    }

    #[allow(dead_code)]
    pub fn alloca_reg(&mut self, val: Value, reg_id: RegID, is_pointer: bool) {
        self.local_values.insert(
            val,
            LocalValue {
                place: Place::Reg(reg_id),
                is_pointer,
            },
        );
    }

    /// Record the name of a basic block
    pub fn register_bb(&mut self, bb: BasicBlock, name: String) {
        self.blocks.insert(bb, name);
    }
}

lazy_static! {
    static ref REG_NAMES: Vec<&'static str> = vec![
        "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4",
        "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4",
        "t5", "t6",
    ];
    static ref NAME_ID_MAPPING: HashMap<&'static str, RegID> = {
        let mut m = HashMap::new();

        REG_NAMES.iter().enumerate().for_each(|(i, n)| {
            m.insert(*n, RegID(i));
        });

        m
    };
}

pub trait IntoID {
    fn into_id(self) -> RegID;
}

pub trait IntoName {
    fn into_name(self) -> &'static str;
}

impl IntoName for RegID {
    fn into_name(self) -> &'static str {
        REG_NAMES.get(self.0).unwrap()
    }
}

impl IntoID for &str {
    fn into_id(self) -> RegID {
        *NAME_ID_MAPPING.get(self).unwrap()
    }
}
