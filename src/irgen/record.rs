use std::{collections::HashMap, vec};

use koopa::ir::builder::{GlobalBuilder, LocalBuilder};
use koopa::ir::builder_traits::*;
use koopa::ir::entities::ValueData;
use koopa::ir::Type as IrType;
use smallvec::SmallVec;

use super::*;
use crate::ast::FuncDef;
use crate::sema::symbol::SymbolTable;
use crate::sema::ty::{Type, TypeKind};

pub struct ProgramRecorder<'i> {
    pub program: Program,
    symbols: &'i SymbolTable,
    values: HashMap<&'i str, Value>,
    funcs: HashMap<&'i str, Function>,
    cur_func: Option<FunctionInfo>,
    loops: SmallVec<[LoopInfo; 6]>,
}

#[derive(Debug)]
pub struct FunctionInfo {
    func: Function,
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

impl<'i> ProgramRecorder<'i> {
    pub fn new(symbols: &'i SymbolTable) -> Self {
        Self {
            program: Program::new(),
            symbols,
            values: HashMap::new(),
            funcs: HashMap::new(),
            cur_func: None,
            loops: SmallVec::new(),
        }
    }

    pub fn insert_value(&mut self, name: &'i str, val: Value) {
        self.values.insert(name, val);
    }

    pub fn enter_func(&mut self, func_def: &'i FuncDef) {
        let (ret_ty, param_tys) =
            if let TypeKind::Func(ret_ty, param_tys) = self.get_ty(&func_def.ident).kind() {
                (ret_ty, param_tys)
            } else {
                unreachable!()
            };
        let param_ir_tys: Vec<_> = param_tys.iter().map(|t| t.get_ir_ty()).collect();
        let params: Vec<_> = vec![None; param_ir_tys.len()]
            .into_iter()
            .zip(param_ir_tys)
            .collect();
        let func = self.program.new_func(FunctionData::with_param_names(
            format!("@{}", &func_def.ident),
            params,
            ret_ty.get_ir_ty(),
        ));
        let builder = self.program.func_mut(func).dfg_mut();
        let entry_bb = builder.new_bb().basic_block(Some("%entry".to_owned()));
        let end_bb = builder.new_bb().basic_block(Some("%end".to_owned()));
        self.cur_func = Some(FunctionInfo {
            func,
            entry_bb,
            end_bb,
            cur_bb: entry_bb,
            ret_val: None,
        });
        self.funcs.insert(&func_def.ident, func);
    }

    pub fn exit_func(&mut self) {
        self.cur_func = None;
    }

    fn declare_func(&mut self, name: &'i str, params_ty: Vec<IrType>, ret_ty: IrType) {
        let func_id = self.program.new_func(FunctionData::new_decl(
            format!("@{}", name),
            params_ty,
            ret_ty,
        ));
        self.funcs.insert(name, func_id);
    }
}

impl ProgramRecorder<'_> {
    pub fn cur_func_id(&self) -> Function {
        self.func().func
    }

    pub fn func(&self) -> &FunctionInfo {
        self.cur_func.as_ref().unwrap()
    }

    pub fn func_mut(&mut self) -> &mut FunctionInfo {
        self.cur_func.as_mut().unwrap()
    }

    pub fn get_func_id(&self, name: &str) -> Function {
        *self.funcs.get(name).unwrap()
    }

    pub fn get_func_data(&self) -> &FunctionData {
        self.program.func(self.cur_func_id())
    }

    pub fn get_ty(&self, name: &str) -> &Type {
        self.symbols.get(name)
    }

    pub fn get_value(&self, name: &str) -> Value {
        match self.values.get(name) {
            Some(value) => *value,
            None => panic!("`{}` doesn't have a value", name),
        }
    }

    pub fn get_value_data(&self, val: Value) -> &ValueData {
        self.program.func(self.cur_func_id()).dfg().value(val)
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

    pub fn new_value(&mut self) -> LocalBuilder<'_> {
        self.program
            .func_mut(self.cur_func_id())
            .dfg_mut()
            .new_value()
    }

    pub fn new_global_value(&mut self) -> GlobalBuilder<'_> {
        self.program.new_value()
    }

    pub fn new_anonymous_bb(&mut self) -> BasicBlock {
        let func_id = self.cur_func_id();
        self.program
            .func_mut(func_id)
            .dfg_mut()
            .new_bb()
            .basic_block(None)
    }

    pub fn set_value_name(&mut self, name: String, val: Value) {
        self.program
            .func_mut(self.cur_func_id())
            .dfg_mut()
            .set_value_name(val, Some(name));
    }

    pub fn set_global_value_name(&mut self, name: String, val: Value) {
        self.program.set_value_name(val, Some(name));
    }

    pub fn push_inst(&mut self, inst: Value) {
        let func_id = self.cur_func_id();
        let cur_bb = self.func().cur_bb;
        self.program
            .func_mut(func_id)
            .layout_mut()
            .bb_mut(cur_bb)
            .insts_mut()
            .push_key_back(inst)
            .unwrap();
    }

    pub fn push_inst_to(&mut self, bb: BasicBlock, inst: Value) {
        self.program
            .func_mut(self.cur_func_id())
            .layout_mut()
            .bb_mut(bb)
            .insts_mut()
            .push_key_back(inst)
            .unwrap()
    }

    pub fn push_bb(&mut self, bb: BasicBlock) {
        let func_id = self.cur_func_id();
        self.program
            .func_mut(func_id)
            .layout_mut()
            .bbs_mut()
            .push_key_back(bb)
            .unwrap();

        self.func_mut().set_cur_bb(bb);
    }

    pub fn is_global(&self) -> bool {
        self.cur_func.is_none()
    }

    pub fn install_lib(&mut self) {
        self.declare_func("getint", vec![], IrType::get_i32());
        self.declare_func("getch", vec![], IrType::get_i32());
        self.declare_func(
            "getarray",
            vec![IrType::get_pointer(IrType::get_i32())],
            IrType::get_i32(),
        );
        self.declare_func("putint", vec![IrType::get_i32()], IrType::get_unit());
        self.declare_func("putch", vec![IrType::get_i32()], IrType::get_unit());
        self.declare_func(
            "putarray",
            vec![IrType::get_i32(), IrType::get_pointer(IrType::get_i32())],
            IrType::get_unit(),
        );
        self.declare_func("starttime", vec![], IrType::get_unit());
        self.declare_func("stoptime", vec![], IrType::get_unit());
    }
}

impl FunctionInfo {
    pub fn get_ret_val(&self) -> Option<Value> {
        self.ret_val
    }

    pub fn get_entry_bb(&self) -> BasicBlock {
        self.entry_bb
    }

    pub fn get_end_bb(&self) -> BasicBlock {
        self.end_bb
    }

    pub fn set_cur_bb(&mut self, cur_bb: BasicBlock) {
        self.cur_bb = cur_bb;
    }

    pub fn set_ret_val(&mut self, ret_val: Value) {
        self.ret_val = Some(ret_val);
    }
}
