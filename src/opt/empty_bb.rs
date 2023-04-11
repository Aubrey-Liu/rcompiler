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
        for (bb, node) in f.layout().bbs() {
            if f.layout().entry_bb().unwrap() == *bb {
                continue;
            }
            let val = *node.insts().front_key().unwrap();
            if let ValueKind::Jump(_) = f.dfg().value(val).kind() {
                empty_bbs.push((*bb, val));
                changed = true;
            }
        }

        for &(bb, val) in &empty_bbs {
            if let ValueKind::Jump(j) = f.dfg().value(val).kind().clone() {
                let params = f.dfg().bb(bb).params().to_owned();
                let target_params_num = f.dfg().bb(j.target()).params().len();
                let args = j.args();

                if params.len() < args.len() {
                    self.append_args_to_pred(f, bb, &j.args()[params.len()..]);
                }
                replace_bb_with(f, bb, j.target());
                if params.len() > target_params_num {
                    self.append_params_to_succ(f, j.target(), &params[target_params_num..]);
                    f.dfg_mut().bb_mut(bb).params_mut().clear();
                }
            } else {
                unreachable!()
            }
            f.dfg_mut().remove_value(val);
            f.dfg_mut().remove_bb(bb);
            f.layout_mut().bbs_mut().remove(&bb);
        }

        changed
    }

    fn try_coalesce_entry(&self, f: &mut FunctionData) {
        let entry_bb = f.layout().entry_bb().unwrap();
        let node = f.layout().bbs().node(&entry_bb).unwrap();
        let val = node.insts().front_key().unwrap();
        if let ValueKind::Jump(j) = value_kind(f, *val).clone() {
            if !j.args().is_empty() {
                return;
            }

            replace_bb_with(f, j.target(), entry_bb);

            f.layout_mut()
                .bbs_mut()
                .node_mut(&entry_bb)
                .unwrap()
                .insts_mut()
                .clear();

            let (_, node) = f.layout_mut().bbs_mut().remove(&j.target()).unwrap();
            for val in node.insts().keys() {
                f.layout_mut()
                    .bb_mut(entry_bb)
                    .insts_mut()
                    .push_key_back(*val)
                    .unwrap();
            }
        }
    }

    fn append_params_to_succ(&self, f: &mut FunctionData, target_bb: BasicBlock, params: &[Value]) {
        f.dfg_mut().bb_mut(target_bb).params_mut().extend(params);
    }

    fn append_args_to_pred(&self, f: &mut FunctionData, origin_bb: BasicBlock, args: &[Value]) {
        for val in f.dfg().bb(origin_bb).used_by().clone() {
            let mut data = f.dfg().value(val).clone();
            match data.kind_mut() {
                ValueKind::Jump(j) => {
                    j.args_mut().extend(args);
                }
                ValueKind::Branch(br) => {
                    if br.true_bb() == origin_bb {
                        br.true_args_mut().extend(args);
                    } else {
                        br.false_args_mut().extend(args);
                    }
                }
                _ => unreachable!(),
            }
            f.dfg_mut().replace_value_with(val).raw(data);
        }
    }
}
