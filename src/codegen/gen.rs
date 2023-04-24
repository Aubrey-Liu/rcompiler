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
            p.push(AsmValue::Blank);
        });

        self.funcs()
            .iter()
            .filter(|(_, data)| data.layout().entry_bb().is_some())
            .for_each(|(&f, data)| {
                ctx.new_func(f);
                ctx.set_func(f);
                data.generate(ctx, p);
            });

        p.memset_def();
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
                    Some(2)
                }
                _ => None,
            })
            .max();

        let is_leaf = max_arg_num.is_none();
        let protect_space = if is_leaf { 0 } else { 4 };
        ctx.cur_func_mut().set_is_leaf(is_leaf);

        let mut off = max(max_arg_num.unwrap_or(0) as i32 - 8, 0) * 4;
        self.params().iter().for_each(|p| {
            ctx.cur_func_mut().spill_to_mem(*p, off, false);
            off += 4;
        });

        for data in self.dfg().bbs().values() {
            for p in data.params() {
                ctx.cur_func_mut().spill_to_mem(*p, off, false);
                off += 4;
            }
        }

        self.layout().bbs().nodes().for_each(|node| {
            for &val in node.insts().keys() {
                let data = self.dfg().value(val);
                if data.ty().is_unit() {
                    continue;
                }
                if let ValueKind::Alloc(_) = data.kind() {
                    ctx.cur_func_mut().spill_to_mem(val, off, false);
                    off += match data.ty().kind() {
                        TypeKind::Pointer(base_ty) => base_ty.size() as i32,
                        _ => unreachable!(),
                    };
                } else {
                    let is_ptr = matches!(data.ty().kind(), TypeKind::Pointer(_));
                    ctx.cur_func_mut().spill_to_mem(val, off, is_ptr);
                    off += data.ty().size() as i32;
                }
            }
        });

        // give a name to each basic block
        self.layout()
            .bbs()
            .keys()
            .for_each(|&bb| ctx.register_bb(bb));

        // align stack size to 16
        let ss = (off + protect_space + 15) / 16 * 16;
        ctx.cur_func_mut().set_ss(ss);

        p.prologue(&self.name()[1..], ss, is_leaf);

        self.params().iter().enumerate().for_each(|(i, param)| {
            match ctx.cur_func().get_local_place(*param) {
                Place::Mem(offset) => {
                    if i < 8 {
                        p.store(format!("a{}", i).into_id(), "sp".into_id(), offset);
                    } else {
                        p.load(*T1, "sp".into_id(), ss + (i as i32 - 8) * 4);
                        p.store(*T1, "sp".into_id(), offset);
                    }
                }
                _ => unreachable!(),
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
            _ => {}
        }
    }
}

impl GenerateAsm for Store {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        let (t1, t2) = (*T1, *T2);
        if let ValueKind::ZeroInit(_) = ctx.value_kind(self.value()) {
            p.read_addr_to(ctx, "a0".into_id(), self.dest());
            p.load_imm(
                "a1".into_id(),
                ctx.value_data(self.value()).ty().size() as i32,
            );
            p.call("zmemset");
        } else {
            p.read_to(ctx, t1, self.value());
            if ctx.is_pointer(self.dest()) {
                p.read_to(ctx, t2, self.dest());
                p.store(t1, t2, 0);
            } else {
                p.write_back(ctx, t1, self.dest());
            }
        }
    }
}

impl GenerateAsm for Branch {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        for (i, &arg) in self.true_args().iter().enumerate() {
            let param = ctx.cur_func_data().dfg().bb(self.true_bb()).params()[i];
            p.read_to(ctx, *T1, arg);
            p.write_back(ctx, *T1, param);
        }

        for (i, &arg) in self.false_args().iter().enumerate() {
            let param = ctx.cur_func_data().dfg().bb(self.false_bb()).params()[i];
            p.read_to(ctx, *T1, arg);
            p.write_back(ctx, *T1, param);
        }

        p.read_to(ctx, *T1, self.cond());

        let true_bb = ctx.cur_func().get_bb_name(&self.true_bb());
        let false_bb = ctx.cur_func().get_bb_name(&self.false_bb());
        p.branch(*T1, true_bb);
        p.jump(false_bb);
    }
}

impl GenerateAsm for Jump {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        for (i, &arg) in self.args().iter().enumerate() {
            let param = ctx.cur_func_data().dfg().bb(self.target()).params()[i];
            p.read_to(ctx, *T1, arg);
            p.write_back(ctx, *T1, param);
        }
        p.jump(ctx.cur_func().get_bb_name(&self.target()));
    }
}

impl GenerateAsm for Return {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram) {
        if self.value().is_some() {
            p.read_to(ctx, "a0".into_id(), self.value().unwrap());
        }
        p.epilogue(ctx.cur_func().ss(), ctx.cur_func().is_leaf());
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
        p.read_to(ctx, t1, self.src());
        if ctx.is_pointer(self.src()) {
            p.load(t1, t1, 0);
        }
        p.write_back(ctx, t1, val);
    }
}

impl NonUnitGenerateAsm for Binary {
    fn generate(&self, ctx: &mut Context, p: &mut AsmProgram, val: Value) {
        let (t1, t2) = (*T1, *T2);
        p.read_to(ctx, t1, self.lhs());
        p.read_to(ctx, t2, self.rhs());
        p.ir_binary(self.op(), t1, t1, t2);
        p.write_back(ctx, t1, val);
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
                let dst = format!("a{}", i);
                p.read_to(ctx, dst.into_id(), arg);
            } else {
                p.read_to(ctx, "t0".into_id(), arg);
                p.store("t0".into_id(), "sp".into_id(), (i as i32 - 8) * 4);
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

        let src = self.src();
        let (t1, t2) = (*T1, *T2);
        if let ValueKind::Integer(i) = ctx.value_kind(self.index()) {
            let offset = i.value() * stride;
            if ctx.is_pointer(src) {
                p.read_to(ctx, t1, src);
                p.binary_with_imm(AsmBinaryOp::Add, t1, t1, offset);
            } else {
                p.read_addr_to(ctx, t1, src);
                p.binary_with_imm(AsmBinaryOp::Add, t1, t1, offset);
            }
        } else {
            if ctx.is_pointer(src) {
                p.read_to(ctx, t1, src);
            } else {
                p.read_addr_to(ctx, t1, src);
            }
            p.read_to(ctx, t2, self.index());
            p.muli(t2, t2, stride);
            p.binary(AsmBinaryOp::Add, t1, t1, t2);
        }

        p.write_back(ctx, t1, val);
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

        let src = self.src();
        let (t1, t2) = (*T1, *T2);
        if let ValueKind::Integer(i) = ctx.value_kind(self.index()) {
            let offset = i.value() * stride;
            if ctx.is_pointer(src) {
                p.read_to(ctx, t1, src);
                p.binary_with_imm(AsmBinaryOp::Add, t1, t1, offset);
            } else {
                p.read_addr_to(ctx, t1, src);
                p.binary_with_imm(AsmBinaryOp::Add, t1, t1, offset);
            }
        } else {
            if ctx.is_pointer(src) {
                p.read_to(ctx, t1, src);
            } else {
                p.read_addr_to(ctx, t1, src);
            }
            p.read_to(ctx, t2, self.index());
            p.muli(t2, t2, stride);
            p.binary(AsmBinaryOp::Add, t1, t1, t2);
        }

        p.write_back(ctx, t1, val);
    }
}
