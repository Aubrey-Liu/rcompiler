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
    dims: &[Value],
) -> Value {
    let mut dst_ptr = src;
    dims.iter().for_each(|idx| {
        let ptr = recorder.new_value(program).get_elem_ptr(dst_ptr, *idx);
        recorder.func().push_inst(program, ptr);
        dst_ptr = ptr;
    });
    dst_ptr
}

pub fn init_array(
    program: &mut Program,
    recorder: &ProgramRecorder,
    src: Value,
    ty: &Type,
    init: &Vec<i32>,
) {
    let mut dims = Vec::new();
    ty.get_dims(&mut dims);

    (0..init.len()).for_each(|index| {
        let mut k = index;
        let mut depth = 0;
        let mut unfolded_index = vec![recorder.new_value(program).integer(0); dims.len()];
        while k > 0 {
            unfolded_index[depth] = recorder
                .new_value(program)
                .integer((k % dims[depth]) as i32);
            k /= dims[depth];
            depth += 1;
        }
        let ptr = get_ptr(program, recorder, src, &unfolded_index);
        let init_val = recorder.new_value(program).integer(init[index]);
        let store = recorder.new_value(program).store(init_val, ptr);
        recorder.func().push_inst(program, store);
    });
}

pub fn init_global_array(
    program: &mut Program,
    recorder: &ProgramRecorder,
    ty: &Type,
    init: &Vec<i32>,
) -> Value {
    let mut dims = Vec::new();
    ty.get_dims(&mut dims);

    fn init_elems(
        program: &mut Program,
        recorder: &ProgramRecorder,
        dims: &[usize],
        init: &[i32],
        pos: usize,
    ) -> Vec<Value> {
        if dims.len() == 1 {
            let len = dims[0];
            (0..len)
                .map(|i| program.new_value().integer(init[pos + i]))
                .collect()
        } else {
            let len = dims[0];
            let stride = dims.iter().skip(1).fold(1, |acc, &x| acc * x);
            (0..len)
                .map(|i| {
                    let elems = init_elems(program, recorder, &dims[1..], init, pos + i * stride);
                    program.new_value().aggregate(elems)
                })
                .collect()
        }
    }

    let elems = init_elems(program, recorder, &dims, init, 0);
    program.new_value().aggregate(elems)
}

pub fn get_lval_ptr<'i>(
    program: &mut Program,
    recorder: &mut ProgramRecorder<'i>,
    lval: &'i LVal,
) -> Value {
    let src = recorder.get_value(&lval.ident);
    let dims: Vec<_> = lval
        .dims
        .iter()
        .map(|e| e.generate_ir(program, recorder).unwrap())
        .collect();
    get_ptr(program, recorder, src, &dims)
}

pub fn load_lval<'i>(
    program: &mut Program,
    recorder: &mut ProgramRecorder<'i>,
    lval: &'i LVal,
) -> Value {
    let val = get_lval_ptr(program, recorder, lval);
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
