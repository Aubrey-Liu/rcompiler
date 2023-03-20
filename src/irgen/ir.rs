use std::fs::read_to_string;

use anyhow::Result;
use koopa::back::KoopaGenerator;
use koopa::ir::builder::BasicBlockBuilder;
use koopa::ir::*;

use super::*;
use crate::ast::*;
use crate::sysy;

pub fn generate_mem_ir(ipath: &str) -> Result<Program> {
    let input = read_to_string(ipath)?;
    let mut global_symt = SymbolTable::new();
    let mut errors = Vec::new();
    let ast = sysy::CompUnitParser::new()
        .parse(&mut errors, &input)
        .unwrap();

    ast.into_program(&mut global_symt)
}

pub fn generate_ir(ipath: &str, opath: &str) -> Result<()> {
    let program = generate_mem_ir(ipath)?;
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
                AstValue::Return(r) => {
                    let val = match r {
                        Some(exp) => exp.generate(symt, func, &mut insts),
                        None => integer(func, 0),
                    };
                    insts.push(ret(func, val));
                }
                AstValue::ConstDecl(decls) => {
                    for d in decls {
                        symt.insert_const_var(&d.name, d.init.const_eval(symt))?;
                    }
                }
                AstValue::Decl(decls) => {
                    for d in decls {
                        let dst = alloc(func);

                        insts.push(dst);
                        func.dfg_mut().set_value_name(
                            dst,
                            Some("@".to_owned() + &symt.generate_name(&d.name)),
                        );

                        if let Some(exp) = &d.init {
                            let val = exp.generate(symt, func, &mut insts);
                            insts.push(store(func, val, dst));
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
                    let val = stmt.val.generate(symt, func, &mut insts);
                    insts.push(store(func, val, dst));
                    symt.initialize(&stmt.name)?;
                }
                AstValue::Block(block) => {
                    symt.enter_scope();
                    block.parse_bb(func, bb, symt)?;
                    symt.exit_scope();
                }
                AstValue::Exp(exp) => {
                    if let Some(e) = exp {
                        e.generate(symt, func, &mut insts);
                    }
                }
            }
            func.layout_mut().bb_mut(bb).insts_mut().extend(insts);
        }

        Ok(())
    }
}
