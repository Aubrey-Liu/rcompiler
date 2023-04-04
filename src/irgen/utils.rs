use koopa::ir::builder_traits::{LocalInstBuilder, ValueBuilder};

use super::*;
use crate::ast::LVal;
use crate::sema::ty::Type;

pub fn local_alloc(recorder: &mut ProgramRecorder, ty: IrType, name: Option<String>) -> Value {
    let entry = recorder.func().get_entry_bb();
    let alloc = recorder.new_value().alloc(ty);
    if let Some(name) = name {
        recorder.set_value_name(name, alloc);
    }
    recorder.push_inst_to(entry, alloc);

    alloc
}

pub fn get_elem_ptr(recorder: &mut ProgramRecorder, src: Value, dims: &[Value]) -> Value {
    let mut dst_ptr = src;
    dims.iter().for_each(|idx| {
        let ptr = recorder.new_value().get_elem_ptr(dst_ptr, *idx);
        recorder.push_inst(ptr);
        dst_ptr = ptr;
    });
    dst_ptr
}

pub fn init_array(recorder: &mut ProgramRecorder, dst: Value, ty: &Type, init: &[i32]) {
    let mut dims = Vec::new();
    ty.get_dims(&mut dims);

    // let zero_init = recorder.new_value().zero_init(ty.get_ir_ty());
    // let store = recorder.new_value().store(zero_init, dst);
    // recorder.push_inst(store);

    fn inner(recorder: &mut ProgramRecorder, dst: Value, init: &[i32], dims: &[usize], pos: usize) {
        if dims.is_empty() {
            let value = recorder.new_value().integer(*init.get(pos).unwrap_or(&0));
            let store = recorder.new_value().store(value, dst);
            recorder.push_inst(store);
        } else {
            let stride: usize = dims.iter().skip(1).product();
            let this_dim = *dims.first().unwrap();
            for i in 0..this_dim {
                let next_pos = pos + i * stride;
                let index = recorder.new_value().integer(i as i32);
                let dst = get_elem_ptr(recorder, dst, &[index]);
                inner(recorder, dst, init, &dims[1..], next_pos);
            }
        }
    }

    inner(recorder, dst, init, &dims, 0);
}

pub fn init_global_array(recorder: &mut ProgramRecorder, ty: &Type, init: &[i32]) -> Value {
    let mut dims = Vec::new();
    ty.get_dims(&mut dims);

    fn inner(recorder: &mut ProgramRecorder, dims: &[usize], init: &[i32], pos: usize) -> Value {
        if pos >= init.len() {
            let ty = Type::infer_from_dims(dims).get_ir_ty();
            return recorder.new_global_value().zero_init(ty);
        }
        if dims.len() == 1 {
            let elems: Vec<_> = (0..dims[0])
                .map(|i| {
                    recorder
                        .new_global_value()
                        .integer(*init.get(pos + i).unwrap_or(&0))
                })
                .collect();
            recorder.new_global_value().aggregate(elems)
        } else {
            let len = dims[0];
            let stride: usize = dims.iter().skip(1).product();
            let elems: Vec<_> = (0..len)
                .map(|i| inner(recorder, &dims[1..], init, pos + i * stride))
                .collect();
            recorder.new_global_value().aggregate(elems)
        }
    }

    inner(recorder, &dims, init, 0)
}

pub fn get_lval_ptr<'i>(recorder: &mut ProgramRecorder<'i>, lval: &'i LVal) -> Value {
    let src = recorder.get_value(&lval.ident);
    let dims: Vec<_> = lval
        .dims
        .iter()
        .map(|e| e.generate_ir(recorder).unwrap())
        .collect();
    match recorder.get_symbol(&lval.ident) {
        Symbol::Pointer(_) => {
            let mut ptr = recorder.new_value().load(src);
            recorder.push_inst(ptr);
            if !dims.is_empty() {
                let get_ptr = recorder.new_value().get_ptr(ptr, dims[0]);
                recorder.push_inst(get_ptr);
                ptr = get_elem_ptr(recorder, get_ptr, &dims[1..]);
            }
            ptr
        }
        _ => get_elem_ptr(recorder, src, &dims),
    }
}

pub fn into_ptr(recorder: &mut ProgramRecorder, val: Value) -> Value {
    let index = recorder.new_value().integer(0);
    let ptr = recorder.new_value().get_elem_ptr(val, index);
    recorder.push_inst(ptr);

    ptr
}

pub fn load_lval<'i>(recorder: &mut ProgramRecorder<'i>, lval: &'i LVal) -> Value {
    let val = get_lval_ptr(recorder, lval);
    let dst = recorder.new_value().load(val);
    recorder.push_inst(dst);

    dst
}

pub fn binary(recorder: &mut ProgramRecorder, op: IrBinaryOp, lhs: Value, rhs: Value) -> Value {
    let by = recorder.new_value().binary(op, lhs, rhs);
    recorder.push_inst(by);

    by
}

pub fn negative(recorder: &mut ProgramRecorder, opr: Value) -> Value {
    let zero = recorder.new_value().integer(0);
    let neg = recorder.new_value().binary(IrBinaryOp::Sub, zero, opr);
    recorder.push_inst(neg);

    neg
}

pub fn logical_not(recorder: &mut ProgramRecorder, opr: Value) -> Value {
    let zero = recorder.new_value().integer(0);
    let not = recorder.new_value().binary(IrBinaryOp::Eq, opr, zero);
    recorder.push_inst(not);

    not
}
