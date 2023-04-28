use std::cmp::max;

use lazy_static_include::lazy_static::lazy_static;

use super::*;

lazy_static! {
    static ref T1: RegID = "t1".into_id();
    static ref T2: RegID = "t2".into_id();
}

pub trait GenerateAsm {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram);
}

impl GenerateAsm for Program {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        Type::set_ptr_size(4);

        self.inst_layout().iter().for_each(|&g| {
            let name = ctx.global_value_data(g).name().as_ref().unwrap()[1..].to_string();
            ctx.register_global_var(g, name);
            p.push(AsmValue::Directive(Directive::Data));
            g.generate(ctx, p);
        });

        self.funcs()
            .iter()
            .filter(|(_, data)| data.layout().entry_bb().is_some())
            .for_each(|(&f, data)| {
                ctx.new_func();
                ctx.set_func(f);
                data.generate(ctx, p);
            });
    }
}

impl GenerateAsm for FunctionData {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        let max_arg_num = self
            .dfg()
            .values()
            .values()
            .filter_map(|data| match data.kind() {
                ValueKind::Call(c) => Some(c.args().len()),
                ValueKind::Store(s)
                    if matches!(self.dfg().value(s.value()).kind(), ValueKind::ZeroInit(_)) =>
                {
                    Some(3)
                }
                _ => None,
            })
            .max();

        let is_leaf = max_arg_num.is_none();
        let protect_space = if is_leaf { 0 } else { 4 };
        ctx.cur_func_mut().set_is_leaf(is_leaf);

        let spilled_arg_size = max(max_arg_num.unwrap_or(0) as i32 - 8, 0) * 4;
        let saved_reg_range = ctx.cur_func().saved_regs();
        let base_offset = spilled_arg_size + (saved_reg_range.1 - saved_reg_range.0);
        ctx.cur_func_mut().set_base_offset(base_offset);

        let mut off = ctx.cur_func().spilled_size();
        self.layout().bbs().nodes().for_each(|node| {
            for &val in node.insts().keys() {
                let data = self.dfg().value(val);
                if let ValueKind::Alloc(_) = data.kind() {
                    let size = match data.ty().kind() {
                        TypeKind::Pointer(base_ty) => base_ty.size() as i32,
                        _ => unreachable!(),
                    };
                    if size > 4 {
                        ctx.cur_func_mut().spill_to_mem(val, off);
                        off += size;
                    }
                }
            }
        });

        // give a name to each basic block
        self.layout()
            .bbs()
            .keys()
            .for_each(|&bb| ctx.register_bb(bb));

        // align stack size to 16
        ctx.cur_func_mut().set_ss(off + protect_space);

        p.prologue(&self.name()[1..], ctx, ctx.cur_func().saved_regs(), is_leaf);

        let ss = ctx.cur_func().ss();
        self.params().iter().enumerate().for_each(|(i, param)| {
            if i < 8 {
                p.write_back(ctx, format!("a{}", i).into_id(), *param);
            } else {
                p.load(*T1, "sp".into_id(), ss + (i as i32 - 8) * 4);
                p.write_back(ctx, *T1, *param);
            }
        });

        self.layout().bbs().iter().for_each(|(bb, node)| {
            p.local_symbol(ctx.cur_func().get_bb_name(bb));
            node.insts().keys().for_each(|inst| inst.generate(ctx, p))
        })
    }
}

impl GenerateAsm for Value {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        match ctx.value_kind(*self) {
            ValueKind::Branch(v) => v.generate(ctx, p),
            ValueKind::Jump(v) => v.generate(ctx, p),
            ValueKind::Return(v) => v.generate(ctx, p),
            ValueKind::Store(v) => v.generate(ctx, p),
            ValueKind::Aggregate(v) => v.generate(ctx, p),
            ValueKind::Integer(v) => v.generate(ctx, p),
            ValueKind::ZeroInit(v) => v.generate(ctx, p, *self),
            ValueKind::Binary(v) => v.generate(ctx, p, *self),
            ValueKind::Load(v) => v.generate(ctx, p, *self),
            ValueKind::Call(v) => v.generate(ctx, p, *self),
            ValueKind::GetElemPtr(v) => v.generate(ctx, p, *self),
            ValueKind::GetPtr(v) => v.generate(ctx, p, *self),
            ValueKind::GlobalAlloc(v) => v.generate(ctx, p, *self),
            ValueKind::Alloc(v) => v.generate(ctx, p, *self),
            _ => {}
        }
    }
}

impl GenerateAsm for Store {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        let (t1, t2) = (*T1, *T2);
        if let ValueKind::ZeroInit(_) = ctx.value_kind(self.value()) {
            let a0 = "a0".into_id();
            let begin = p.read_value(ctx, a0, self.dest());
            if begin != a0 {
                p.mv(a0, begin);
            }
            p.load_imm("a1".into_id(), 0);
            p.load_imm(
                "a2".into_id(),
                ctx.value_data(self.value()).ty().size() as i32,
            );
            p.call("memset@plt");
            return;
        }
        let dst = p.read_value_addr(ctx, t1, self.dest());
        let val = p.read_value(ctx, t2, self.value());
        p.store(val, dst, 0);
    }
}

impl GenerateAsm for Branch {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        for (i, &arg) in self.true_args().iter().enumerate() {
            let param = ctx.cur_func_data().dfg().bb(self.true_bb()).params()[i];
            p.move_local_value(ctx, param, arg);
        }

        for (i, &arg) in self.false_args().iter().enumerate() {
            let param = ctx.cur_func_data().dfg().bb(self.false_bb()).params()[i];
            p.move_local_value(ctx, param, arg);
        }

        let cond = p.read_value(ctx, *T1, self.cond());

        let true_bb = ctx.cur_func().get_bb_name(&self.true_bb());
        let false_bb = ctx.cur_func().get_bb_name(&self.false_bb());
        p.branch(cond, true_bb);
        p.jump(false_bb);
    }
}

impl GenerateAsm for Jump {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        let bb_params = ctx.cur_func_data().dfg().bb(self.target()).params();
        for (&arg, &param) in self.args().iter().zip(bb_params) {
            p.move_local_value(ctx, param, arg);
        }
        p.jump(ctx.cur_func().get_bb_name(&self.target()));
    }
}

impl GenerateAsm for Return {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        if self.value().is_some() {
            let a0 = "a0".into_id();
            let ret = p.read_value(ctx, a0, self.value().unwrap());
            if ret != a0 {
                p.mv(a0, ret);
            }
        }
        p.epilogue(ctx, ctx.cur_func().saved_regs(), ctx.cur_func().is_leaf());
    }
}

impl GenerateAsm for Integer {
    #[allow(unused_variables)]
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        p.directive(Directive::Word(self.value()));
    }
}

impl GenerateAsm for Aggregate {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        self.elems().iter().for_each(|e| e.generate(ctx, p))
    }
}

pub trait NonUnitGenerateAsm {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram, val: Value);
}

impl NonUnitGenerateAsm for Load {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram, val: Value) {
        let t1 = *T1;
        let src = p.read_value_addr(ctx, t1, self.src());
        p.load(t1, src, 0);
        p.write_back(ctx, t1, val);
    }
}

impl NonUnitGenerateAsm for Binary {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram, val: Value) {
        let (t1, t2) = (*T1, *T2);
        let lhs = p.read_value(ctx, t1, self.lhs());
        let rhs = p.read_value(ctx, t2, self.rhs());
        match ctx.get_local_place(val) {
            Place::Reg(dst) => p.ir_binary(self.op(), dst, lhs, rhs),
            Place::Mem(off) => {
                p.ir_binary(self.op(), t1, lhs, rhs);
                p.store(t1, "sp".into_id(), off);
            }
        }
    }
}

impl NonUnitGenerateAsm for GlobalAlloc {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram, val: Value) {
        let id = ctx.get_global_var(&val);
        p.global_symbol(id);
        self.init().generate(ctx, p)
    }
}

impl NonUnitGenerateAsm for ZeroInit {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram, val: Value) {
        let size = ctx.global_value_data(val).ty().size();
        p.directive(Directive::Zero(size));
    }
}

impl NonUnitGenerateAsm for Call {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram, val: Value) {
        self.args().iter().enumerate().for_each(|(i, &arg)| {
            if i < 8 {
                let dst = format!("a{}", i).into_id();
                let reg = p.read_value(ctx, dst, arg);
                if reg != dst {
                    p.mv(dst, reg);
                }
            } else {
                let reg = p.read_value(ctx, *T1, arg);
                p.store(reg, "sp".into_id(), (i as i32 - 8) * 4);
            }
        });

        let callee = &ctx.get_func_name(self.callee())[1..];
        p.call(callee);

        // write the return value to pre-allocated space
        if let TypeKind::Function(_, ret_ty) = ctx.func_data(self.callee()).ty().kind() {
            if ret_ty.is_i32() {
                p.write_back(ctx, "a0".into_id(), val);
            }
        }
    }
}

impl NonUnitGenerateAsm for GetElemPtr {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram, val: Value) {
        let ty = ctx.value_ty(val);
        let stride = if let TypeKind::Pointer(base_ty) = ty.kind() {
            base_ty.size() as i32
        } else {
            unreachable!()
        };

        let (t1, t2) = (*T1, *T2);
        if let ValueKind::Integer(i) = ctx.value_kind(self.index()) {
            let offset = i.value() * stride;
            let src = p.read_value_addr(ctx, t1, self.src());
            match ctx.get_local_place(val) {
                Place::Reg(dst) => p.binary_with_imm(AsmBinaryOp::Add, dst, src, offset),
                Place::Mem(dst_off) => {
                    p.binary_with_imm(AsmBinaryOp::Add, t1, src, offset);
                    p.store(t1, "sp".into_id(), dst_off);
                }
            }
        } else {
            let src = p.read_value_addr(ctx, t1, self.src());
            let index = p.read_value(ctx, t2, self.index());
            p.muli(t2, index, stride);
            match ctx.get_local_place(val) {
                Place::Reg(dst) => p.binary(AsmBinaryOp::Add, dst, src, t2),
                Place::Mem(dst_off) => {
                    p.binary(AsmBinaryOp::Add, t1, src, t2);
                    p.store(t1, "sp".into_id(), dst_off);
                }
            }
        }
    }
}

impl NonUnitGenerateAsm for GetPtr {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram, val: Value) {
        let ty = ctx.value_ty(val);
        let stride = if let TypeKind::Pointer(base_ty) = ty.kind() {
            base_ty.size() as i32
        } else {
            unreachable!()
        };

        let (t1, t2) = (*T1, *T2);
        if let ValueKind::Integer(i) = ctx.value_kind(self.index()) {
            let offset = i.value() * stride;
            let src = p.read_value_addr(ctx, t1, self.src());
            match ctx.get_local_place(val) {
                Place::Reg(dst) => p.binary_with_imm(AsmBinaryOp::Add, dst, src, offset),
                Place::Mem(dst_off) => {
                    p.binary_with_imm(AsmBinaryOp::Add, t1, src, offset);
                    p.store(t1, "sp".into_id(), dst_off);
                }
            }
        } else {
            let src = p.read_value_addr(ctx, t1, self.src());
            let index = p.read_value(ctx, t2, self.index());
            p.muli(t2, index, stride);
            match ctx.get_local_place(val) {
                Place::Reg(dst) => p.binary(AsmBinaryOp::Add, dst, src, t2),
                Place::Mem(dst_off) => {
                    p.binary(AsmBinaryOp::Add, t1, src, t2);
                    p.store(t1, "sp".into_id(), dst_off);
                }
            }
        }
    }
}

impl NonUnitGenerateAsm for Alloc {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram, val: Value) {
        let begin_at = ctx.cur_func().get_local_array(val);
        match ctx.get_local_place(val) {
            Place::Reg(reg) => p.binary_with_imm(AsmBinaryOp::Add, reg, "sp".into_id(), begin_at),
            Place::Mem(off) => {
                p.binary_with_imm(AsmBinaryOp::Add, *T1, "sp".into_id(), begin_at);
                p.store(*T1, "sp".into_id(), off);
            }
        }
    }
}
