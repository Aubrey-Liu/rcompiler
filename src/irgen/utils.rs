use koopa::ir::builder_traits::{LocalInstBuilder, ValueBuilder};

use super::*;
use crate::ast::LVal;
use crate::sema::ty::Type;

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

pub fn get_ptr(
    program: &mut Program,
    recorder: &ProgramRecorder,
    src: Value,
    dims: &[usize],
) -> Value {
    let mut dst = src;
    dims.iter().for_each(|d| {
        let idx = recorder.new_value(program).integer(*d as i32);
        let ptr = recorder.new_value(program).get_elem_ptr(dst, idx);
        recorder.func().push_inst(program, ptr);
        dst = ptr;
    });
    dst
}

pub fn init_array(
    program: &mut Program,
    recorder: &ProgramRecorder,
    src: Value,
    ty: &Type,
    init: &Vec<i32>,
) {
    if let Type::Array(_, len) = ty {
        (0..*len).for_each(|i| {
            let ptr = get_ptr(program, recorder, src, &[i]);
            let init_val = recorder.new_value(program).integer(init[i]);
            let store = recorder.new_value(program).store(init_val, ptr);
            recorder.func().push_inst(program, store);
        })
    } else {
        unreachable!()
    }
}

pub fn load_var(program: &mut Program, recorder: &ProgramRecorder, lval: &LVal) -> Value {
    let src = recorder.get_value(&lval.ident);
    let dims: Vec<_> = lval.dims.iter().map(|d| d.get_i32() as usize).collect();
    let val = get_ptr(program, recorder, src, &dims);
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
