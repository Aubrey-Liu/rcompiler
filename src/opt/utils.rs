use koopa::ir::{
    builder_traits::{LocalInstBuilder, ValueBuilder},
    *,
};

pub fn value_kind(func: &FunctionData, val: Value) -> &ValueKind {
    func.dfg().value(val).kind()
}

pub fn replace_bb_with(func: &mut FunctionData, bb: BasicBlock, new_bb: BasicBlock) {
    for user in func.dfg().bb(bb).used_by().clone() {
        let mut data = func.dfg().value(user).clone();
        match data.kind_mut() {
            ValueKind::Jump(j) => {
                *j.target_mut() = new_bb;
                func.dfg_mut().replace_value_with(user).raw(data);
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
                    if !func.dfg().value_eq(true_arg, false_arg) {
                        naive = false;
                        break;
                    }
                }
                if naive {
                    func.dfg_mut()
                        .replace_value_with(user)
                        .jump_with_args(new_bb, br.true_args().to_vec());
                } else {
                    func.dfg_mut().replace_value_with(user).raw(data);
                }
            }
            _ => unreachable!(),
        }
    }
}

pub fn replace_variable(func: &mut FunctionData, origin: Value, replace_by: Value) {
    for user in func.dfg().value(origin).used_by().clone() {
        let mut data = func.dfg().value(user).clone();
        match data.kind_mut() {
            ValueKind::Branch(br) => *br.cond_mut() = replace_by,
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
            ValueKind::Call(call) => {
                for arg in call.args_mut() {
                    if *arg == origin {
                        *arg = replace_by;
                    }
                }
            }
            _ => {}
        }
        func.dfg_mut().replace_value_with(user).raw(data);
    }
}
