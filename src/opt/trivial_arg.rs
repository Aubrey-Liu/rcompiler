use std::collections::HashSet;

use koopa::ir::{builder_traits::ValueBuilder, BasicBlock, FunctionData, Value, ValueKind};

use super::*;

pub struct RemoveTrivialArgs;

impl FunctionPass for RemoveTrivialArgs {
    fn run_on(&mut self, f: &mut FunctionData) {
        if f.layout().entry_bb().is_some() {
            while self.try_remove_trivial_args(f) {}
        }
    }
}

impl RemoveTrivialArgs {
    fn try_remove_trivial_args(&self, f: &mut FunctionData) -> bool {
        let mut used_values = HashSet::new();
        let mut trivial_args = Vec::new();
        for (&bb, data) in f.dfg().bbs() {
            for (i, arg) in data.params().iter().enumerate() {
                let same = match self.is_trivial(f, bb, i) {
                    Some(same) => same,
                    None => continue,
                };
                // replace the argument only when it's not related to any other possible replacements
                if !used_values.contains(arg) {
                    used_values.insert(same);
                    trivial_args.push((same, bb, i));
                }
            }
        }

        trivial_args.sort_by(|a, b| b.2.cmp(&a.2));

        for &(same, bb, idx) in &trivial_args {
            self.remove_trivial_arg(f, bb, idx, same);
        }

        !trivial_args.is_empty()
    }

    fn remove_trivial_arg(
        &self,
        f: &mut FunctionData,
        bb: BasicBlock,
        param_idx: usize,
        same: Value,
    ) {
        for user in f.dfg().bb(bb).used_by().clone() {
            let mut data = f.dfg().value(user).clone();
            match data.kind_mut() {
                ValueKind::Jump(j) => {
                    j.args_mut().remove(param_idx);
                }
                ValueKind::Branch(br) => {
                    if br.true_bb() == bb {
                        br.true_args_mut().remove(param_idx);
                    }
                    if br.false_bb() == bb {
                        br.false_args_mut().remove(param_idx);
                    }
                }
                _ => unreachable!(),
            }
            f.dfg_mut().replace_value_with(user).raw(data);
        }
        let param = f.dfg_mut().bb_mut(bb).params_mut().remove(param_idx);
        replace_variable(f, param, same);
        fix_bb_param_idx(f, bb);
    }

    fn is_trivial(&self, f: &FunctionData, bb: BasicBlock, idx: usize) -> Option<Value> {
        let param = f.dfg().bb(bb).params()[idx];
        let mut same = None;
        for &user in f.dfg().bb(bb).used_by() {
            let arg = match value_kind(f, user) {
                ValueKind::Jump(j) => j.args()[idx],
                ValueKind::Branch(br) => {
                    if br.true_bb() == bb {
                        br.true_args()[idx]
                    } else {
                        br.false_args()[idx]
                    }
                }
                _ => unreachable!(),
            };
            if arg == param {
                continue;
            }
            if same.is_some() && f.dfg().value_eq(same.unwrap(), arg) {
                continue;
            }
            if same.is_some() {
                return None;
            }

            same = Some(arg);
        }

        same
    }
}
