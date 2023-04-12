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
    let data = f.dfg().value(replace_by).clone();
    f.dfg_mut().replace_value_with(origin).raw(data);
}
