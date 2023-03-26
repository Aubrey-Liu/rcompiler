use koopa::ir::builder_traits::{LocalInstBuilder, ValueBuilder};

use super::*;

pub fn logical_check(program: &mut Program, recorder: &ProgramRecorder, opr: Value) -> Value {
    let zero = recorder.new_value(program).integer(0);
    let checked = recorder
        .new_value(program)
        .binary(IrBinaryOp::NotEq, opr, zero);
    recorder.func().push_inst(program, checked);

    checked
}

pub fn logical_and(
    program: &mut Program,
    recorder: &ProgramRecorder,
    lhs: Value,
    rhs: Value,
) -> Value {
    let lhs = logical_check(program, recorder, lhs);
    let rhs = logical_check(program, recorder, rhs);
    let and = recorder
        .new_value(program)
        .binary(IrBinaryOp::And, lhs, rhs);
    recorder.func().push_inst(program, and);

    and
}

pub fn logical_or(
    program: &mut Program,
    recorder: &ProgramRecorder,
    lhs: Value,
    rhs: Value,
) -> Value {
    let lhs = logical_check(program, recorder, lhs);
    let rhs = logical_check(program, recorder, rhs);
    let or = recorder.new_value(program).binary(IrBinaryOp::Or, lhs, rhs);
    recorder.func().push_inst(program, or);

    or
}

pub fn load_var(
    program: &mut Program,
    recorder: &ProgramRecorder,
    val: Value,
    init: bool,
) -> Value {
    if !init {
        panic!("variable used but it isn't initialized",)
    }
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
        .binary(IrBinaryOp::Eq, opr, zero);
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
