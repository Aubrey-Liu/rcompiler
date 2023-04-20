use std::collections::HashSet;

use koopa::ir::{
    builder_traits::{LocalInstBuilder, ValueBuilder},
    *,
};

pub fn value_kind(f: &FunctionData, val: Value) -> &ValueKind {
    f.dfg().value(val).kind()
}

pub fn replace_bb_with(f: &mut FunctionData, bb: BasicBlock, new_bb: BasicBlock) {
    for user in f.dfg().bb(bb).used_by().clone() {
        let mut data = f.dfg().value(user).clone();
        match data.kind_mut() {
            ValueKind::Jump(j) => {
                *j.target_mut() = new_bb;
                f.dfg_mut().replace_value_with(user).raw(data);
            }
            ValueKind::Branch(br) => {
                if br.true_bb() == bb {
                    *br.true_bb_mut() = new_bb;
                }
                if br.false_bb() == bb {
                    *br.false_bb_mut() = new_bb;
                }
                let mut naive = br.true_bb() == br.false_bb();
                for (&true_arg, &false_arg) in br.true_args().iter().zip(br.false_args()) {
                    if !f.dfg().value_eq(true_arg, false_arg) {
                        naive = false;
                        break;
                    }
                }
                if naive {
                    f.dfg_mut()
                        .replace_value_with(user)
                        .jump_with_args(new_bb, br.true_args().to_vec());
                } else {
                    f.dfg_mut().replace_value_with(user).raw(data);
                }
            }
            _ => unreachable!(),
        }
    }
}

pub fn replace_variable(f: &mut FunctionData, origin: Value, replace_by: Value) {
    for user in f.dfg().value(origin).used_by().clone() {
        let used_by = f.dfg().value(user).used_by().clone();
        let mut data = f.dfg().value(user).clone();
        match data.kind_mut() {
            ValueKind::Return(ret) => *ret.value_mut() = Some(replace_by),
            ValueKind::Store(s) => *s.value_mut() = replace_by,
            ValueKind::GetElemPtr(g) => *g.index_mut() = replace_by,
            ValueKind::GetPtr(g) => *g.index_mut() = replace_by,
            ValueKind::Binary(b) => {
                if origin == b.lhs() {
                    *b.lhs_mut() = replace_by;
                } else {
                    *b.rhs_mut() = replace_by;
                }
            }
            ValueKind::Call(call) => call.args_mut().iter_mut().for_each(|arg| {
                if *arg == origin {
                    *arg = replace_by;
                }
            }),
            ValueKind::Jump(j) => j.args_mut().iter_mut().for_each(|arg| {
                if *arg == origin {
                    *arg = replace_by;
                }
            }),
            ValueKind::Branch(br) => {
                if br.cond() == origin {
                    *br.cond_mut() = replace_by;
                }
                br.true_args_mut().iter_mut().for_each(|arg| {
                    if *arg == origin {
                        *arg = replace_by;
                    }
                });
                br.false_args_mut().iter_mut().for_each(|arg| {
                    if *arg == origin {
                        *arg = replace_by;
                    }
                });
            }
            _ => {}
        }
        f.dfg_mut().replace_value_with(user).raw(data);
        fix_used_by(f, &used_by);
    }
}

pub fn fix_used_by(f: &mut FunctionData, used_by: &HashSet<Value>) {
    for &user in used_by {
        let deeper_used_by = f.dfg().value(user).used_by().clone();
        let data = f.dfg().value(user).clone();
        f.dfg_mut().replace_value_with(user).raw(data);
        fix_used_by(f, &deeper_used_by);
    }
}

pub fn last_inst_of_bb(f: &FunctionData, bb: BasicBlock) -> Value {
    f.layout()
        .bbs()
        .node(&bb)
        .map(|n| *n.insts().back_key().unwrap())
        .unwrap()
}

pub fn fix_bb_param_idx(f: &mut FunctionData, bb: BasicBlock) {
    for (i, &param) in f.dfg().bb(bb).params().to_owned().iter().enumerate() {
        let mut data = f.dfg().value(param).clone();
        if let ValueKind::BlockArgRef(arg) = data.kind_mut() {
            *arg.index_mut() = i;
        }
        f.dfg_mut().replace_value_with(param).raw(data);
    }
}
