use std::fs::read_to_string;

use anyhow::Result;
use koopa::back::KoopaGenerator;
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

    ast.generate(&mut global_symt)
}

pub fn generate_ir(ipath: &str, opath: &str) -> Result<()> {
    let program = generate_mem_ir(ipath)?;
    let mut gen = KoopaGenerator::from_path(opath)?;
    gen.generate_on(&program)?;

    Ok(())
}

impl<'input> CompUnit {
    pub fn generate(&'input self, symt: &mut SymbolTable<'input>) -> Result<Program> {
        let mut program = Program::new();
        let fib = new_func(&mut program, &self.func_def.ident);
        let func = program.func_mut(fib);
        // Create the entry block
        self.func_def.block.generate_new_bb(symt, func, "%entry")?;

        Ok(program)
    }
}

impl<'input> Block {
    pub fn generate_new_bb(
        &'input self,
        symt: &mut SymbolTable<'input>,
        func: &mut FunctionData,
        name: &str,
    ) -> Result<()> {
        let bb = new_bb(func, name);
        push_bb(func, bb);
        self.generate(symt, func, bb)?;

        Ok(())
    }

    pub fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        func: &mut FunctionData,
        bb: BasicBlock,
    ) -> Result<()> {
        for item in &self.items {
            let mut insts = Vec::new();

            match item {
                BlockItem::ConstDecl(decls) => {
                    for d in decls {
                        symt.insert_const_var(&d.name, d.init.const_eval(symt))?;
                    }
                }
                BlockItem::Decl(decls) => {
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
                BlockItem::Stmt(stmt) => stmt.generate(symt, func, bb)?,
            }
            push_insts(func, bb, insts);
        }

        Ok(())
    }
}

impl<'input> Stmt {
    pub fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        func: &mut FunctionData,
        bb: BasicBlock,
    ) -> Result<()> {
        let mut insts = Vec::new();

        match self {
            Self::Assign(assign) => {
                let dst = match symt.get(&assign.name).unwrap() {
                    Symbol::Var { val, .. } => *val,
                    Symbol::ConstVar(_) => unreachable!(),
                };
                let val = assign.val.generate(symt, func, &mut insts);
                insts.push(store(func, val, dst));
                symt.initialize(&assign.name)?;
            }
            Self::Block(block) => {
                symt.enter_scope();
                block.generate(symt, func, bb)?;
                symt.exit_scope();
            }
            Self::Exp(exp) => {
                if let Some(e) = exp {
                    e.generate(symt, func, &mut insts);
                }
            }
            Self::Return(r) => {
                let val = match r {
                    Some(exp) => exp.generate(symt, func, &mut insts),
                    None => integer(func, 0),
                };
                insts.push(ret(func, val));
            }
            Self::Cond(_cond) => todo!(),
        }
        push_insts(func, bb, insts);

        Ok(())
    }
}
