use std::collections::HashMap;
use std::fs::read_to_string;

use anyhow::Result;
use koopa::back::KoopaGenerator;
use koopa::ir::*;

use super::*;
use crate::ast::*;
use crate::sysy;

#[derive(Debug, Clone, Copy)]
enum Flow {
    Branch(Value, BasicBlock, BasicBlock),
    Jump(BasicBlock),
}

type FlowGraph = HashMap<BasicBlock, Flow>;

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
        self.func_def.generate(symt, &mut program)?;
        Ok(program)
    }
}

impl<'input> FuncDef {
    pub fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        program: &mut Program,
    ) -> Result<()> {
        let fib = new_func(program, &self.ident);
        let func = program.func_mut(fib);
        let mut flow = FlowGraph::new();
        self.block.generate_entry(symt, func, &mut flow)?;

        for (bb, flow) in &flow {
            match flow {
                Flow::Branch(cond, true_bb, false_bb) => {
                    branch_from(func, *cond, *bb, *true_bb, *false_bb);
                }
                Flow::Jump(target) => {
                    check_and_jump(func, *bb, *target);
                }
            }
        }

        Ok(())
    }
}

impl<'input> Block {
    fn generate_entry(
        &'input self,
        symt: &mut SymbolTable<'input>,
        func: &mut FunctionData,
        flow: &mut FlowGraph,
    ) -> Result<()> {
        // Create the entry block
        let bb = new_bb(func, "%entry");
        self.generate(symt, func, bb, bb, flow)?;

        Ok(())
    }

    fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        func: &mut FunctionData,
        bb: BasicBlock,
        link_to: BasicBlock,
        flow: &mut FlowGraph,
    ) -> Result<()> {
        let mut bb = bb;

        for item in &self.items {
            match item {
                BlockItem::Decl(decl) => decl.generate(symt, func, bb)?,
                BlockItem::Stmt(stmt) => bb = stmt.generate(symt, func, bb, link_to, flow)?,
            }
        }

        Ok(())
    }
}

impl<'input> Decl {
    fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        func: &mut FunctionData,
        bb: BasicBlock,
    ) -> Result<()> {
        let mut insts = Vec::new();
        match self {
            Decl::ConstDecl(decls) => {
                for d in decls {
                    symt.insert_const_var(&d.name, d.init.const_eval(symt))?;
                }
            }
            Decl::VarDecl(decls) => {
                for d in decls {
                    let dst = alloc(func);

                    insts.push(dst);
                    func.dfg_mut()
                        .set_value_name(dst, Some(generate_var_name(&d.name)));

                    if let Some(exp) = &d.init {
                        let val = exp.generate(symt, func, &mut insts);
                        insts.push(store(func, val, dst));
                        symt.insert_var(&d.name, dst, true)?;
                    } else {
                        symt.insert_var(&d.name, dst, false)?;
                    }
                }
            }
        }
        push_insts(func, bb, &mut insts);

        Ok(())
    }
}

impl<'input> Stmt {
    fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        func: &mut FunctionData,
        bb: BasicBlock,
        link_to: BasicBlock,
        flow: &mut FlowGraph,
    ) -> Result<BasicBlock> {
        let mut insts = Vec::new();
        let mut move_to = bb;

        match self {
            Self::Assign(assign) => {
                let dst = match symt.get(&assign.name).unwrap() {
                    Symbol::Var { val, .. } => *val,
                    Symbol::ConstVar(_) => {
                        bail!("\"{}\" must be a modifiable lvalue", assign.name);
                    }
                };
                let val = assign.val.generate(symt, func, &mut insts);
                insts.push(store(func, val, dst));
                symt.initialize(&assign.name)?;
            }
            Self::Block(block) => {
                symt.enter_scope();
                block.generate(symt, func, bb, link_to, flow)?;
                symt.exit_scope();
            }
            Self::Exp(exp) => {
                if let Some(e) = exp {
                    // evaluation result is ignored here
                    e.generate(symt, func, &mut insts);
                }
            }
            Self::Return(val) => {
                check_and_return(symt, func, bb, val);
            }
            Self::Branch(branch) => {
                move_to = branch.generate(symt, func, bb, link_to, flow)?;
            }
        }

        if !insts.is_empty() {
            push_insts(func, bb, &insts);
        }

        Ok(move_to)
    }
}

impl<'input> Branch {
    fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        func: &mut FunctionData,
        bb: BasicBlock,
        link_to: BasicBlock,
        flow: &mut FlowGraph,
    ) -> Result<BasicBlock> {
        let mut insts = Vec::new();
        let cond = self.cond.generate(symt, func, &mut insts);
        let (true_bb, false_bb, end_bb) = new_branch(func);

        flow.insert(bb, Flow::Branch(cond, true_bb, false_bb));
        // the flows can be overwritten
        flow.insert(true_bb, Flow::Jump(end_bb));
        flow.insert(false_bb, Flow::Jump(end_bb));

        if bb != link_to {
            flow.insert(end_bb, Flow::Jump(link_to));
        }

        self.if_stmt.generate(symt, func, true_bb, end_bb, flow)?;
        if let Some(el_stmt) = &self.el_stmt {
            el_stmt.generate(symt, func, false_bb, end_bb, flow)?;
        }

        push_insts(func, bb, &insts);

        Ok(end_bb)
    }
}
