use koopa::ir::builder_traits::{LocalInstBuilder, ValueBuilder};
use smallvec::SmallVec;

use super::*;
use crate::ast::{InitVal, LVal};
use crate::sema::ty::{DimTy, Type, TypeKind};

pub fn eval_array(init: &InitVal, ty: &Type) -> Vec<i32> {
    let mut elems = Vec::new();
    let mut dims: DimTy = SmallVec::new();
    ty.get_dims(&mut dims);

    let mut acc = 1;
    let boundaries: DimTy = dims
        .iter()
        .rev()
        .map(|d| {
            acc *= d;
            acc
        })
        .collect();

    fn fill_array(init: &[InitVal], bds: &[usize], pos: usize, elems: &mut Vec<i32>) -> usize {
        let (idx, stride) = bds
            .iter()
            .rev()
            .enumerate()
            .find(|&(_, d)| pos % d == 0)
            .expect("invalid initializer");
        let mut pos = pos;
        let next_pos = pos + stride;
        let next_bd = bds.len() - idx - 1;

        for e in init {
            match e {
                InitVal::Expr(e) => {
                    if pos > elems.len() {
                        elems.resize_with(pos, Default::default);
                    }
                    elems.push(e.get_i32());
                    pos += 1;
                }
                InitVal::List(list) => {
                    pos = fill_array(list, &bds[0..next_bd], pos, elems);
                }
            };
        }

        next_pos
    }

    if let InitVal::List(list) = init {
        fill_array(list, &boundaries, 0, &mut elems);
    } else {
        panic!("incompatible initializer type")
    }

    elems
}

pub fn init_array(recorder: &mut ProgramRecorder, dst: Value, ty: &Type, init: &[i32]) {
    fn init_array_recur(
        recorder: &mut ProgramRecorder,
        dst: Value,
        ty: &Type,
        init: &[i32],
        pos: usize,
    ) {
        match ty.kind() {
            TypeKind::Integer => {
                let value = recorder.new_value().integer(*init.get(pos).unwrap_or(&0));
                let store = recorder.new_value().store(value, dst);
                recorder.push_inst(store);
            }
            TypeKind::Array(base_ty, len) => {
                let stride: usize = base_ty.size();
                for i in 0..*len {
                    let next_pos = pos + i * stride;
                    let index = recorder.new_value().integer(i as i32);
                    let dst = get_elem_ptr(recorder, dst, &[index]);
                    init_array_recur(recorder, dst, base_ty, init, next_pos);
                }
            }
            _ => unreachable!(),
        }
    }

    init_array_recur(recorder, dst, ty, init, 0);
}

pub fn init_global_array(recorder: &mut ProgramRecorder, ty: &Type, init: &[i32]) -> Value {
    fn init_global_array_recur(
        recorder: &mut ProgramRecorder,
        ty: &Type,
        init: &[i32],
        pos: usize,
    ) -> Value {
        if pos >= init.len() {
            return recorder.new_global_value().zero_init(ty.get_ir_ty());
        }
        if let TypeKind::Array(base_ty, len) = ty.kind() {
            match base_ty.kind() {
                TypeKind::Integer => {
                    let elems: Vec<_> = (0..*len)
                        .map(|i| {
                            recorder
                                .new_global_value()
                                .integer(*init.get(pos + i).unwrap_or(&0))
                        })
                        .collect();
                    recorder.new_global_value().aggregate(elems)
                }
                TypeKind::Array(_, _) => {
                    let elems: Vec<_> = (0..*len)
                        .map(|i| {
                            init_global_array_recur(
                                recorder,
                                base_ty,
                                init,
                                pos + i * base_ty.size(),
                            )
                        })
                        .collect();
                    recorder.new_global_value().aggregate(elems)
                }
                _ => unreachable!(),
            }
        } else {
            unreachable!()
        }
    }

    init_global_array_recur(recorder, ty, init, 0)
}

pub fn into_ptr(recorder: &mut ProgramRecorder, val: Value) -> Value {
    let index = recorder.new_value().integer(0);
    let ptr = recorder.new_value().get_elem_ptr(val, index);
    recorder.push_inst(ptr);

    ptr
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

pub fn get_lval_ptr<'i>(recorder: &mut ProgramRecorder<'i>, lval: &'i LVal) -> Value {
    let src = recorder.get_value(&lval.ident);
    let dims: SmallVec<[Value; 4]> = lval
        .dims
        .iter()
        .map(|e| e.generate_ir(recorder).unwrap())
        .collect();
    match recorder.get_ty(&lval.ident).kind() {
        TypeKind::Integer => src,
        TypeKind::Array(_, _) => get_elem_ptr(recorder, src, &dims),
        TypeKind::Pointer(_) => {
            let mut ptr = recorder.new_value().load(src);
            recorder.push_inst(ptr);
            if !dims.is_empty() {
                let get_ptr = recorder.new_value().get_ptr(ptr, dims[0]);
                recorder.push_inst(get_ptr);
                ptr = get_elem_ptr(recorder, get_ptr, &dims[1..]);
            }
            ptr
        }
        _ => unreachable!(),
    }
}

pub fn load_lval<'i>(recorder: &mut ProgramRecorder<'i>, lval: &'i LVal) -> Value {
    let val = get_lval_ptr(recorder, lval);
    let dst = recorder.new_value().load(val);
    recorder.push_inst(dst);

    dst
}

pub fn local_alloc(recorder: &mut ProgramRecorder, ty: IrType, name: Option<String>) -> Value {
    let entry = recorder.func().get_entry_bb();
    let alloc = recorder.new_value().alloc(ty);
    if let Some(name) = name {
        recorder.set_value_name(name, alloc);
    }
    recorder.push_inst_to(entry, alloc);

    alloc
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
