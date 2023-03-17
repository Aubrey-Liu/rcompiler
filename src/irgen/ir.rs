use std::fs::read_to_string;

use anyhow::Result;
use koopa::back::KoopaGenerator;
use koopa::ir::builder::BasicBlockBuilder;
use koopa::ir::*;

use crate::ast::*;
use crate::sysy;

pub fn into_mem_ir(ipath: &str) -> Result<Program> {
    let input = read_to_string(ipath)?;
    let mut global_symt = SymbolTable::new();
    let mut errors = Vec::new();
    let ast = sysy::CompUnitParser::new()
        .parse(&mut errors, &input)
        .unwrap();

    ast.into_program(&mut global_symt)
}

pub fn into_text_ir(ipath: &str, opath: &str) -> Result<()> {
    let program = into_mem_ir(ipath)?;
    let mut gen = KoopaGenerator::from_path(opath)?;
    gen.generate_on(&program)?;

    Ok(())
}

impl<'input> CompUnit {
    pub fn into_program(&'input self, symt: &mut SymbolTable<'input>) -> Result<Program> {
        let mut program = Program::new();
        let fib = self.func_def.new_func(&mut program);
        let func = program.func_mut(fib);
        // Create the entry block
        let entry = self.func_def.block.new_bb(func, "%entry");
        self.func_def.push_bb(func, entry);
        self.func_def.block.parse_bb(func, entry, symt)?;

        Ok(program)
    }
}

impl FuncDef {
    pub fn new_func(&self, program: &mut Program) -> Function {
        let name = "@".to_owned() + &self.ident;
        program.new_func(FunctionData::with_param_names(
            name,
            Vec::new(),
            Type::get_i32(),
        ))
    }

    pub fn push_bb(&self, func: &mut FunctionData, bb: BasicBlock) {
        func.layout_mut().bbs_mut().extend([bb]);
    }
}

impl<'input> Block {
    pub fn new_bb(&self, func: &mut FunctionData, name: &str) -> BasicBlock {
        func.dfg_mut().new_bb().basic_block(Some(name.into()))
    }

    pub fn parse_bb(
        &'input self,
        func: &mut FunctionData,
        bb: BasicBlock,
        symt: &mut SymbolTable<'input>,
    ) -> Result<()> {
        for value in &self.values {
            let mut insts = Vec::new();
            match value {
                AstValue::Return(ret) => {
                    let val = match ret {
                        Some(exp) => exp.into_value(symt, func, &mut insts),
                        None => inst_builder::integer(func, 0),
                    };
                    insts.push(inst_builder::ret(func, val));
                }
                AstValue::ConstDecl(decls) => {
                    for d in decls {
                        symt.insert_const_var(&d.name, d.init.const_eval(symt))?;
                    }
                }
                AstValue::Decl(decls) => {
                    for d in decls {
                        let dst = inst_builder::alloc(func);

                        insts.push(dst);
                        func.dfg_mut().set_value_name(
                            dst,
                            Some("@".to_owned() + &symt.generate_name(&d.name)),
                        );

                        if let Some(exp) = &d.init {
                            let val = exp.into_value(symt, func, &mut insts);
                            insts.push(inst_builder::store(func, val, dst));
                            symt.insert_var(&d.name, dst, true)?;
                        } else {
                            symt.insert_var(&d.name, dst, false)?;
                        }
                    }
                }
                AstValue::Stmt(stmt) => {
                    let dst = match symt.get(&stmt.name).unwrap() {
                        Symbol::Var { val, .. } => *val,
                        Symbol::ConstVar(_) => unreachable!(),
                    };
                    let val = stmt.val.into_value(symt, func, &mut insts);
                    insts.push(inst_builder::store(func, val, dst));
                    symt.initialize(&stmt.name)?;
                }
                AstValue::Block(block) => {
                    symt.enter_scope();
                    block.parse_bb(func, bb, symt)?;
                    symt.exit_scope();
                }
                AstValue::Exp(exp) => {
                    if let Some(e) = exp {
                        e.into_value(symt, func, &mut insts);
                    }
                }
            }
            func.layout_mut().bb_mut(bb).insts_mut().extend(insts);
        }

        Ok(())
    }
}

pub mod inst_builder {
    use koopa::ir::builder_traits::{LocalInstBuilder, ValueBuilder};
    use koopa::ir::{BinaryOp, FunctionData, Type, Value};

    pub fn integer(func: &mut FunctionData, i: i32) -> Value {
        func.dfg_mut().new_value().integer(i)
    }

    pub fn ret(func: &mut FunctionData, v: Value) -> Value {
        func.dfg_mut().new_value().ret(Some(v))
    }

    pub fn alloc(func: &mut FunctionData) -> Value {
        // allocate a pointer for an integer
        func.dfg_mut().new_value().alloc(Type::get_i32())
    }

    pub fn store(func: &mut FunctionData, val: Value, dst: Value) -> Value {
        func.dfg_mut().new_value().store(val, dst)
    }

    pub fn load(func: &mut FunctionData, src: Value) -> Value {
        func.dfg_mut().new_value().load(src)
    }

    pub fn binary(func: &mut FunctionData, op: BinaryOp, lhs: Value, rhs: Value) -> Value {
        func.dfg_mut().new_value().binary(op, lhs, rhs)
    }

    pub fn neg(func: &mut FunctionData, val: Value) -> Value {
        let zero = zero(func);
        func.dfg_mut().new_value().binary(BinaryOp::Sub, zero, val)
    }

    pub fn not(func: &mut FunctionData, val: Value) -> Value {
        let zero = zero(func);
        func.dfg_mut().new_value().binary(BinaryOp::Eq, zero, val)
    }

    fn zero(func: &mut FunctionData) -> Value {
        func.dfg_mut().new_value().integer(0)
    }
}
