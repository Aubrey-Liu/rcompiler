use koopa::ir::builder_traits::{LocalInstBuilder, ValueBuilder};

use super::*;
use crate::ast::LVal;
use crate::sema::ty::Type;

pub fn local_alloc(
    recorder: &ProgramRecorder,
    program: &mut Program,
    ty: IrType,
    name: Option<String>,
) -> Value {
    let entry = recorder.func().entry_bb();
    let alloc = recorder.new_value(program).alloc(ty);
    if let Some(name) = name {
        recorder.func().set_value_name(program, name, alloc);
    }
    recorder.func().push_inst_to(program, entry, alloc);

    alloc
}

pub fn get_elem_ptr(
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
    dst: Value,
    ty: &Type,
    init: &[i32],
) {
    let mut dims = Vec::new();
    ty.get_dims(&mut dims);

    let zero_init = recorder.new_value(program).zero_init(ty.get_ir_ty());
    let store = recorder.new_value(program).store(zero_init, dst);
    recorder.func().push_inst(program, store);

    fn inner(
        program: &mut Program,
        recorder: &ProgramRecorder,
        dst: Value,
        init: &[i32],
        dims: &[usize],
        pos: usize,
    ) {
        if dims.is_empty() {
            let value = recorder
                .new_value(program)
                .integer(*init.get(pos).unwrap_or(&0));
            let store = recorder.new_value(program).store(value, dst);
            recorder.func().push_inst(program, store);
        } else {
            let stride: usize = dims.iter().skip(1).product();
            let this_dim = *dims.first().unwrap();
            for i in 0..this_dim {
                let next_pos = pos + i * stride;
                if next_pos >= init.len() {
                    break;
                }
                let index = recorder.new_value(program).integer(i as i32);
                let dst = get_elem_ptr(program, recorder, dst, &[index]);
                inner(program, recorder, dst, init, &dims[1..], next_pos);
            }
        }
    }

    inner(program, recorder, dst, init, &dims, 0);
}

pub fn init_global_array(program: &mut Program, ty: &Type, init: &[i32]) -> Value {
    let mut dims = Vec::new();
    ty.get_dims(&mut dims);

    fn inner(program: &mut Program, dims: &[usize], init: &[i32], pos: usize) -> Vec<Value> {
        if dims.len() == 1 {
            (0..dims[0])
                .map(|i| {
                    program
                        .new_value()
                        .integer(*init.get(pos + i).unwrap_or(&0))
                })
                .collect()
        } else {
            let len = dims[0];
            let stride: usize = dims.iter().skip(1).product();
            (0..len)
                .map(|i| {
                    let elems = inner(program, &dims[1..], init, pos + i * stride);
                    program.new_value().aggregate(elems)
                })
                .collect()
        }
    }

    let elems = inner(program, &dims, init, 0);
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
    match recorder.get_symbol(&lval.ident) {
        Symbol::Pointer(_) => {
            let mut ptr = recorder.new_value(program).load(src);
            recorder.func().push_inst(program, ptr);
            if !dims.is_empty() {
                let get_ptr = recorder.new_value(program).get_ptr(ptr, dims[0]);
                recorder.func().push_inst(program, get_ptr);
                ptr = get_elem_ptr(program, recorder, get_ptr, &dims[1..]);
            }
            ptr
        }
        _ => get_elem_ptr(program, recorder, src, &dims),
    }
}

pub fn into_ptr(program: &mut Program, recorder: &ProgramRecorder, val: Value) -> Value {
    let index = recorder.new_value(program).integer(0);
    let ptr = recorder.new_value(program).get_elem_ptr(val, index);
    recorder.func().push_inst(program, ptr);

    ptr
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
