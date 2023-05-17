use std::collections::HashSet;

use koopa::ir::{BasicBlock, FunctionData};
use smallvec::SmallVec;

use super::*;

pub struct RemoveUnreachable;

impl FunctionPass for RemoveUnreachable {
    fn run_on(&mut self, f: &mut FunctionData) {
        self.remove_unreachable_bb(f);
    }
}

impl RemoveUnreachable {
    fn remove_unreachable_bb(&mut self, f: &mut FunctionData) {
        loop {
            let mut changed = false;
            let mut removed_bbs = SmallVec::<[BasicBlock; 4]>::new();
            for bb in f.dfg().bbs().keys() {
                if f.layout().entry_bb().unwrap() != *bb && f.dfg().bb(*bb).used_by().is_empty() {
                    removed_bbs.push(*bb);
                    changed = true;
                }
            }
            for bb in removed_bbs {
                // remove a bb will not automatically remove the value attaching to it
                let mut removed_values = HashSet::new();
                for &v in f.layout().bbs().node(&bb).unwrap().insts().keys() {
                    removed_values.insert(v);
                }
                while !removed_values.is_empty() {
                    removed_values.retain(|&v| {
                        let flag = f.dfg().value(v).used_by().is_empty();
                        if flag {
                            f.dfg_mut().remove_value(v);
                        }

                        !flag
                    });
                }
                f.dfg_mut().remove_bb(bb);
                f.layout_mut().bbs_mut().remove(&bb);
            }
            if !changed {
                break;
            }
        }
    }
}
