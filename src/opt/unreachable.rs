use koopa::ir::{BasicBlock, FunctionData, Value};
use smallvec::SmallVec;

use super::*;

pub struct RemoveUnreachable;

impl FunctionPass for RemoveUnreachable {
    fn run_on(&mut self, f: &mut FunctionData) {
        if f.layout().entry_bb().is_some() {
            self.remove_unreachable_bb(f);
        }
    }
}

impl RemoveUnreachable {
    fn remove_unreachable_bb(&mut self, func: &mut FunctionData) {
        loop {
            let mut changed = false;
            let mut removed_bbs = SmallVec::<[BasicBlock; 4]>::new();
            for bb in func.dfg().bbs().keys() {
                if func.layout().entry_bb().unwrap() != *bb
                    && func.dfg().bb(*bb).used_by().is_empty()
                {
                    removed_bbs.push(*bb);
                    changed = true;
                }
            }
            for bb in removed_bbs {
                // remove a bb will not automatically remove the value attaching to it
                let mut removed_values = SmallVec::<[Value; 6]>::new();
                for &v in func.layout().bbs().node(&bb).unwrap().insts().keys() {
                    removed_values.push(v);
                }
                for &v in &removed_values {
                    func.dfg_mut().remove_value(v);
                }
                func.dfg_mut().remove_bb(bb);
                func.layout_mut().bbs_mut().remove(&bb);
            }
            if !changed {
                break;
            }
        }
    }
}
