use std::collections::HashMap;

use super::*;
use crate::ast::*;

#[derive(Debug, Clone, Copy)]
pub enum Flow {
    Branch(Value, BasicBlock, BasicBlock),
    Jump(BasicBlock),
}

pub type FlowGraph = HashMap<BasicBlock, Flow>;

impl<'i> While {
    pub fn generate(
        &'i self,
        symt: &mut SymbolTable<'i>,
        flow: &mut FlowGraph,
        func: &mut FunctionData,
        bb: BasicBlock,
        link_to: BasicBlock,
    ) -> Result<BasicBlock> {
        let (entry, body, end) = new_loop(func);

        flow.insert(bb, Flow::Jump(entry));

        let info = BranchInfo(entry, body, end);
        shortcut(symt, flow, func, info, &self.cond)?;

        flow.insert(body, Flow::Jump(entry));
        if bb != link_to {
            flow.insert(end, Flow::Jump(link_to));
        }

        self.stmt.generate(symt, flow, func, body, entry)?;

        Ok(end)
    }
}

impl<'i> Branch {
    pub fn generate(
        &'i self,
        symt: &mut SymbolTable<'i>,
        flow: &mut FlowGraph,
        func: &mut FunctionData,
        bb: BasicBlock,
        link_to: BasicBlock,
    ) -> Result<BasicBlock> {
        let (true_bb, false_bb, end_bb) = new_branch(func);

        let br = BranchInfo(bb, true_bb, false_bb);
        shortcut(symt, flow, func, br, &self.cond)?;

        // the flows can be overwritten
        flow.insert(true_bb, Flow::Jump(end_bb));
        flow.insert(false_bb, Flow::Jump(end_bb));

        if bb != link_to {
            flow.insert(end_bb, Flow::Jump(link_to));
        }

        self.if_stmt.generate(symt, flow, func, true_bb, end_bb)?;
        if let Some(el_stmt) = &self.el_stmt {
            el_stmt.generate(symt, flow, func, false_bb, end_bb)?;
        }

        Ok(end_bb)
    }
}

struct BranchInfo(BasicBlock, BasicBlock, BasicBlock);

fn shortcut<'i>(
    symt: &mut SymbolTable<'i>,
    flow: &mut FlowGraph,
    func: &mut FunctionData,
    info: BranchInfo,
    cond: &Box<Exp>,
) -> Result<()> {
    let BranchInfo(bb, true_bb, false_bb) = info;

    if !cond.is_logical() {
        let mut insts = Vec::new();

        let cond_val = cond.generate(symt, func, &mut insts);
        flow.insert(bb, Flow::Branch(cond_val, true_bb, false_bb));

        push_insts(func, bb, &insts);

        return Ok(());
    }

    match cond.get_binary_op().unwrap() {
        BinaryOp::And => {
            let bxp = cond.get_bxp().unwrap();
            let check_right = new_bb(func, "%check");

            let path_1 = BranchInfo(bb, check_right, false_bb);
            let path_2 = BranchInfo(check_right, true_bb, false_bb);
            shortcut(symt, flow, func, path_1, &bxp.lhs)?;
            shortcut(symt, flow, func, path_2, &bxp.rhs)
        }
        BinaryOp::Or => {
            let bxp = cond.get_bxp().unwrap();
            let check_right = new_bb(func, "%check");

            let path_1 = BranchInfo(bb, true_bb, check_right);
            let path_2 = BranchInfo(check_right, true_bb, false_bb);

            shortcut(symt, flow, func, path_1, &bxp.lhs)?;
            shortcut(symt, flow, func, path_2, &bxp.rhs)
        }
        _ => unreachable!(),
    }
}
