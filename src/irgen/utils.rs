use koopa::ir::builder_traits::{LocalInstBuilder, ValueBuilder};

use super::*;

pub fn alloc(
    recorder: &ProgramRecorder,
    program: &mut Program,
    ty: IrType,
    name: Option<String>,
) -> Value {
    let entry = recorder.func().entry_bb();
    let val = recorder.new_value(program).alloc(ty);
    if let Some(name) = name {
        recorder.func().set_value_name(program, name, val);
    }
    recorder.func().push_inst_to(program, entry, val);

    val
}

pub fn load_var(program: &mut Program, recorder: &ProgramRecorder, val: Value) -> Value {
    let dst = recorder.new_value(program).load(val);
    recorder.func().push_inst(program, dst);

    dst
}

pub fn binary(
    program: &mut Program,
    recorder: &ProgramRecorder,
    op: IrBinaryOp,
    lhs: Value,
    rhs: Value,
) -> Value {
    let by = recorder.new_value(program).binary(op, lhs, rhs);
    recorder.func().push_inst(program, by);

    by
}

pub fn negative(program: &mut Program, recorder: &ProgramRecorder, opr: Value) -> Value {
    let zero = recorder.new_value(program).integer(0);
    let neg = recorder
        .new_value(program)
        .binary(IrBinaryOp::Sub, zero, opr);
    recorder.func().push_inst(program, neg);

    neg
}

pub fn logical_not(program: &mut Program, recorder: &ProgramRecorder, opr: Value) -> Value {
    let zero = recorder.new_value(program).integer(0);
    let not = recorder
        .new_value(program)
        .binary(IrBinaryOp::Eq, opr, zero);
    recorder.func().push_inst(program, not);

    not
}
