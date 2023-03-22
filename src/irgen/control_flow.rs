use std::collections::HashMap;

use super::*;
use crate::ast::*;

#[derive(Debug, Clone, Copy)]
pub(in crate::irgen) enum Flow {
    Branch(Value, BasicBlock, BasicBlock),
    Jump(BasicBlock),
}

pub(in crate::irgen) type FlowGraph = HashMap<BasicBlock, Flow>;

impl<'input> Branch {
    pub(in crate::irgen) fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        flow: &mut FlowGraph,
        func: &mut FunctionData,
        bb: BasicBlock,
        link_to: BasicBlock,
    ) -> Result<BasicBlock> {
        let (true_bb, false_bb, end_bb) = new_branch(func);

        self.shortcut(symt, flow, func, bb, true_bb, false_bb, &self.cond)?;

        // the flows can be overwritten
        flow.insert(true_bb, Flow::Jump(end_bb));
        flow.insert(false_bb, Flow::Jump(end_bb));

        if bb != link_to {
            flow.insert(end_bb, Flow::Jump(link_to));
        }

        // let cond = self.cond.generate(symt, func, &mut insts);

        self.if_stmt.generate(symt, flow, func, true_bb, end_bb)?;
        if let Some(el_stmt) = &self.el_stmt {
            el_stmt.generate(symt, flow, func, false_bb, end_bb)?;
        }

        Ok(end_bb)
    }

    fn shortcut(
        &'input self,
        symt: &mut SymbolTable<'input>,
        flow: &mut FlowGraph,
        func: &mut FunctionData,
        bb: BasicBlock,
        true_bb: BasicBlock,
        false_bb: BasicBlock,
        cond: &Box<Exp>,
    ) -> Result<()> {
        let mut insts = Vec::new();

        if !cond.is_logical() {
            let cond_val = cond.generate(symt, func, &mut insts);
            flow.insert(bb, Flow::Branch(cond_val, true_bb, false_bb));

            push_insts(func, bb, &insts);

            return Ok(());
        }

        match cond.get_binary_op().unwrap() {
            BinaryOp::And => {
                let bxp = cond.get_bxp().unwrap();
                let check_right = new_bb(func, "%check");
                self.shortcut(symt, flow, func, bb, check_right, false_bb, &bxp.lhs)?;
                self.shortcut(symt, flow, func, check_right, true_bb, false_bb, &bxp.rhs)?;
            }
            BinaryOp::Or => {
                let bxp = cond.get_bxp().unwrap();
                let check_right = new_bb(func, "%check");
                self.shortcut(symt, flow, func, bb, true_bb, check_right, &bxp.lhs)?;
                self.shortcut(symt, flow, func, check_right, true_bb, false_bb, &bxp.rhs)?;
            }
            _ => unreachable!(),
        }
        push_insts(func, bb, &insts);

        Ok(())
    }
}
