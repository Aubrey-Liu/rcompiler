use std::rc::Rc;
use std::{collections::HashMap, vec};

use koopa::ir::builder::LocalBuilder;
use koopa::ir::builder_traits::*;

use super::*;
use crate::ast::FuncDef;
use crate::sema::symbol::{Symbol, SymbolTable};

#[derive(Debug)]
pub struct ProgramRecorder<'i> {
    symbols: Rc<RefCell<SymbolTable>>,
    values: HashMap<&'i str, Value>,
    funcs: HashMap<&'i str, Function>,
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

impl<'i> ProgramRecorder<'i> {
    pub fn new(symbols: &Rc<RefCell<SymbolTable>>) -> Self {
        Self {
            symbols: Rc::clone(symbols),
            values: HashMap::new(),
            funcs: HashMap::new(),
            cur_func: None,
            loops: vec![],
        }
    }

    pub fn new_value<'p>(&self, program: &'p mut Program) -> LocalBuilder<'p> {
        program.func_mut(self.func().id()).dfg_mut().new_value()
    }

    pub fn get_symbol(&self, name: &str) -> Symbol {
        self.symbols.borrow().get(name).clone()
    }

    pub fn insert_value(&mut self, name: &'i str, val: Value) {
        self.values.insert(name, val);
    }

    pub fn get_value(&self, name: &str) -> Value {
        match self.values.get(name) {
            Some(value) => *value,
            None => panic!("`{}` doesn't have a value", name),
        }
    }

    pub fn is_global(&self) -> bool {
        self.cur_func.is_none()
    }

    pub fn enter_func(&mut self, program: &mut Program, func_def: &'i FuncDef) {
        let (ret_ty, param_ty) = self.get_symbol(&func_def.ident).get_func_ir_ty();
        let params: Vec<_> = func_def
            .params
            .iter()
            .map(|p| Some(format!("@{}", &p.ident)))
            .zip(param_ty.into_iter())
            .collect();
        let id = program.new_func(FunctionData::with_param_names(
            format!("@{}", &func_def.ident),
            params,
            ret_ty,
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
        self.funcs.insert(&func_def.ident, id);
    }

    pub fn exit_func(&mut self) {
        self.cur_func = None;
    }

    fn declare_func(
        &mut self,
        program: &mut Program,
        name: &'i str,
        params_ty: Vec<IrType>,
        ret_ty: IrType,
    ) {
        let func_id = program.new_func(FunctionData::new_decl(
            format!("@{}", name),
            params_ty,
            ret_ty,
        ));
        self.funcs.insert(name, func_id);
    }
}

impl ProgramRecorder<'_> {
    pub fn func(&self) -> &FunctionInfo {
        self.cur_func.as_ref().unwrap()
    }

    pub fn func_mut(&mut self) -> &mut FunctionInfo {
        self.cur_func.as_mut().unwrap()
    }

    pub fn get_func_id(&self, name: &str) -> Function {
        *self.funcs.get(name).unwrap()
    }
}

impl ProgramRecorder<'_> {
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

impl ProgramRecorder<'_> {
    pub fn install_lib(&mut self, program: &mut Program) {
        self.declare_func(program, "getint", vec![], IrType::get_i32());
        self.declare_func(program, "getch", vec![], IrType::get_i32());
        self.declare_func(
            program,
            "getarray",
            vec![IrType::get_pointer(IrType::get_i32())],
            IrType::get_i32(),
        );
        self.declare_func(
            program,
            "putint",
            vec![IrType::get_i32()],
            IrType::get_unit(),
        );
        self.declare_func(
            program,
            "putch",
            vec![IrType::get_i32()],
            IrType::get_unit(),
        );
        self.declare_func(
            program,
            "putarray",
            vec![IrType::get_i32(), IrType::get_pointer(IrType::get_i32())],
            IrType::get_unit(),
        );
        self.declare_func(program, "starttime", vec![], IrType::get_unit());
        self.declare_func(program, "stoptime", vec![], IrType::get_unit());
    }
}

impl FunctionInfo {
    #[allow(unused)]
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
