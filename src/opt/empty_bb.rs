use koopa::ir::{builder_traits::ValueBuilder, BasicBlock, FunctionData, Value, ValueKind};
use smallvec::SmallVec;

use super::*;

pub struct RemoveEmptyBB;

impl FunctionPass for RemoveEmptyBB {
    fn run_on(&mut self, f: &mut FunctionData) {
        if f.layout().entry_bb().is_some() {
            while self.remove_empty_bb(f) {}
            self.try_coalesce_entry(f);
        }
    }
}

impl RemoveEmptyBB {
    fn remove_empty_bb(&self, f: &mut FunctionData) -> bool {
        let mut changed = false;
        let mut empty_bbs: SmallVec<[(BasicBlock, Value); 4]> = SmallVec::new();
        'outer: for (bb, node) in f.layout().bbs() {
            if f.layout().entry_bb().unwrap() == *bb {
                continue;
            }
            let val = *node.insts().front_key().unwrap();
            let params = f.dfg().bb(*bb).params();
            if let ValueKind::Jump(j) = f.dfg().value(val).kind() {
                if params.is_empty() {
                    empty_bbs.push((*bb, val));
                    changed = true;
                    continue;
                }
                if params.len() != j.args().len() {
                    continue;
                }
                for (x, y) in params.iter().zip(j.args()) {
                    if x != y {
                        continue 'outer;
                    }
                }
                empty_bbs.push((*bb, val));
                changed = true;
            }
        }

        for &(bb, val) in &empty_bbs {
            if let ValueKind::Jump(j) = f.dfg().value(val).kind().clone() {
                let extra_args = if f.dfg().bb(bb).params().is_empty() {
                    j.args()
                } else {
                    &[]
                };
                self.replace_empty_bb(f, bb, j.target(), extra_args);
                f.dfg_mut().remove_value(val);
                f.dfg_mut().remove_bb(bb);
                f.layout_mut().bbs_mut().remove(&bb);
            }
        }

        changed
    }

    fn try_coalesce_entry(&self, f: &mut FunctionData) {
        let entry_bb = f.layout().entry_bb().unwrap();
        let node = f.layout().bbs().node(&entry_bb).unwrap();
        let val = *node.insts().front_key().unwrap();
        if let ValueKind::Jump(j) = value_kind(f, val).clone() {
            if !j.args().is_empty() {
                return;
            }
            let target = j.target();
            replace_bb_with(f, target, entry_bb);

            f.layout_mut().bb_mut(entry_bb).insts_mut().remove(&val);

            f.dfg_mut().remove_value(val);
            f.dfg_mut().remove_bb(target);
            let (_, node) = f.layout_mut().bbs_mut().remove(&target).unwrap();

            for val in node.insts().keys() {
                f.layout_mut()
                    .bb_mut(entry_bb)
                    .insts_mut()
                    .push_key_back(*val)
                    .unwrap();
            }
        }
    }

    fn replace_empty_bb(
        &self,
        f: &mut FunctionData,
        bb: BasicBlock,
        next_bb: BasicBlock,
        args: &[Value],
    ) {
        for user in f.dfg().bb(bb).used_by().clone() {
            let mut data = f.dfg().value(user).clone();
            let pred_args = match data.kind_mut() {
                ValueKind::Jump(j) => {
                    *j.target_mut() = next_bb;
                    j.args_mut()
                }
                ValueKind::Branch(br) => {
                    if br.true_bb() == bb {
                        *br.true_bb_mut() = next_bb;
                        br.true_args_mut()
                    } else {
                        *br.false_bb_mut() = next_bb;
                        br.false_args_mut()
                    }
                }
                _ => unreachable!(),
            };
            pred_args.extend(args);
            f.dfg_mut().replace_value_with(user).raw(data);
        }
    }
}
