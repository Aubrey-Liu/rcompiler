use koopa::ir::{builder_traits::ValueBuilder, BasicBlock, FunctionData, Value, ValueKind};

use super::*;

pub struct RemoveTrivialArgs;

impl FunctionPass for RemoveTrivialArgs {
    fn run_on(&mut self, f: &mut FunctionData) {
        while self.try_remove_unused_args(f) || self.try_remove_trivial_args(f) {}
    }
}

impl RemoveTrivialArgs {
    fn try_remove_unused_args(&self, f: &mut FunctionData) -> bool {
        let mut unused_args = Vec::new();
        for (&bb, data) in f.dfg().bbs() {
            for (i, &p) in data.params().iter().enumerate() {
                if f.dfg().value(p).used_by().is_empty() {
                    unused_args.push((bb, i))
                }
            }
        }
        unused_args.sort_by(|a, b| b.1.cmp(&a.1));

        for &(bb, idx) in &unused_args {
            self.remove_arg(f, bb, idx);
        }
        for &(bb, _) in &unused_args {
            fix_bb_param_idx(f, bb);
        }

        !unused_args.is_empty()
    }

    fn try_remove_trivial_args(&self, f: &mut FunctionData) -> bool {
        let mut trivial_args = Vec::new();
        'outer: for (&bb, data) in f.dfg().bbs() {
            for i in 0..data.params().len() {
                let same = match self.is_trivial(f, bb, i) {
                    Some(same) => same,
                    None => continue,
                };
                // replace the argument only when it's not related to any other possible replacements
                trivial_args.push((same, bb, i));
                break 'outer;
            }
        }

        for &(same, bb, idx) in &trivial_args {
            self.replace_trivial_arg(f, bb, idx, same);
            fix_bb_param_idx(f, bb);
        }

        !trivial_args.is_empty()
    }

    fn remove_arg(&self, f: &mut FunctionData, bb: BasicBlock, param_idx: usize) -> Value {
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

        f.dfg_mut().bb_mut(bb).params_mut().remove(param_idx)
    }

    fn replace_trivial_arg(
        &self,
        f: &mut FunctionData,
        bb: BasicBlock,
        param_idx: usize,
        same: Value,
    ) {
        let param = self.remove_arg(f, bb, param_idx);
        replace_variable(f, param, same);
        f.dfg_mut().remove_value(param);
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
            if let Some(s) = same {
                if value_eq(f, s, arg) {
                    continue;
                } else {
                    return None;
                }
            }

            same = Some(arg);
        }

        same
    }
}
