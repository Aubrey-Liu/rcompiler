use std::cmp::max;
use std::io::Result;

use super::*;

pub trait GenerateAsm {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()>;
}

impl GenerateAsm for Program {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        Type::set_ptr_size(4);

        self.inst_layout().iter().try_for_each(|&g| {
            ctx.register_global_var(g);
            gen.data_seg()?;
            g.generate(gen, ctx)?;
            gen.blank_line()
        })?;

        self.funcs()
            .iter()
            .filter(|(_, data)| data.layout().entry_bb().is_some())
            .try_for_each(|(&f, data)| {
                ctx.new_func(f);
                ctx.set_func(f);
                data.generate(gen, ctx)
            })?;

        gen.flush()
    }
}

impl GenerateAsm for FunctionData {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        let max_arg_num = self
            .dfg()
            .values()
            .values()
            .filter_map(|data| {
                if let ValueKind::Call(c) = data.kind() {
                    Some(c.args().len())
                } else {
                    None
                }
            })
            .max();

        let is_leaf = max_arg_num.is_none();
        let protect_space = if is_leaf { 0 } else { 4 };
        ctx.cur_func_mut().set_is_leaf(is_leaf);
        ctx.cur_func_mut().set_params(self.params());

        let mut off = if let Some(n) = max_arg_num {
            max(n as i32 - 8, 0) * 4
        } else {
            0
        };

        self.dfg()
            .values()
            .iter()
            .filter(|(_, data)| data.kind().is_local_inst() && !data.used_by().is_empty())
            .for_each(|(&val, data)| {
                if let ValueKind::Alloc(_) = data.kind() {
                    ctx.cur_func_mut().register_var(val, off, false);
                    off += match data.ty().kind() {
                        TypeKind::Pointer(base_ty) => base_ty.size() as i32,
                        _ => unreachable!(),
                    };
                } else {
                    let is_ptr = matches!(data.ty().kind(), TypeKind::Pointer(_));
                    ctx.cur_func_mut().register_var(val, off as i32, is_ptr);
                    off += data.ty().size() as i32;
                }
            });
        self.dfg()
            .bbs()
            .iter()
            .for_each(|(&bb, _)| ctx.register_bb(bb));

        // align stack size to 16
        let ss = (off + protect_space + 15) / 16 * 16;
        ctx.cur_func_mut().set_ss(ss);

        gen.prologue(&self.name()[1..], ss, is_leaf)?;
        self.layout().bbs().iter().try_for_each(|(bb, node)| {
            gen.enter_bb(ctx.cur_func().get_bb_name(bb))?;
            node.insts()
                .keys()
                .try_for_each(|inst| inst.generate(gen, ctx))
        })
    }
}

impl GenerateAsm for Value {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        match ctx.value_kind(*self) {
            ValueKind::Branch(v) => v.generate(gen, ctx),
            ValueKind::Jump(v) => v.generate(gen, ctx),
            ValueKind::Return(v) => v.generate(gen, ctx),
            ValueKind::Store(v) => v.generate(gen, ctx),
            ValueKind::Aggregate(v) => v.generate(gen, ctx),
            ValueKind::Integer(v) => v.generate(gen, ctx),
            ValueKind::ZeroInit(v) => v.generate(gen, ctx, *self),
            ValueKind::Binary(v) => v.generate(gen, ctx, *self),
            ValueKind::Load(v) => v.generate(gen, ctx, *self),
            ValueKind::Call(v) => v.generate(gen, ctx, *self),
            ValueKind::GetElemPtr(v) => v.generate(gen, ctx, *self),
            ValueKind::GetPtr(v) => v.generate(gen, ctx, *self),
            ValueKind::GlobalAlloc(v) => v.generate(gen, ctx, *self),
            _ => Ok(()),
        }
    }
}

impl GenerateAsm for Store {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        read_to(gen, ctx, "t1", self.value())?;
        if ctx.is_pointer(self.dest()) {
            read_to(gen, ctx, "t2", self.dest())?;
            gen.sw("t1", "t2", 0)
        } else {
            write_back(gen, ctx, "t1", self.dest())
        }
    }
}

impl GenerateAsm for Branch {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        read_to(gen, ctx, "t1", self.cond())?;
        branch(gen, ctx, "t1", &self.true_bb(), &self.false_bb())
    }
}

impl GenerateAsm for Jump {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        gen.j(ctx.cur_func().get_bb_name(&self.target()))
    }
}

impl GenerateAsm for Return {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        if self.value().is_some() {
            read_to(gen, ctx, "a0", self.value().unwrap())?;
        }
        gen.epilogue(ctx.cur_func().ss(), ctx.cur_func().is_leaf())
    }
}

impl GenerateAsm for Integer {
    fn generate(&self, gen: &mut AsmGenerator, _ctx: &mut Context) -> Result<()> {
        gen.global_word(self.value())
    }
}

impl GenerateAsm for Aggregate {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        self.elems().iter().try_for_each(|e| e.generate(gen, ctx))
    }
}

impl NonUnitGenerateAsm for ZeroInit {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()> {
        let size = ctx.global_value_data(val).ty().size();
        gen.global_zero_init(size)
    }
}

pub trait NonUnitGenerateAsm {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()>;
}

impl NonUnitGenerateAsm for Load {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()> {
        if !ctx.is_used(val) {
            return Ok(());
        }
        read_to(gen, ctx, "t1", self.src())?;
        if ctx.is_pointer(self.src()) {
            gen.lw("t1", "t1", 0)?;
        }
        write_back(gen, ctx, "t1", val)
    }
}

impl NonUnitGenerateAsm for Binary {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()> {
        if !ctx.is_used(val) {
            return Ok(());
        }

        read_to(gen, ctx, "t1", self.lhs())?;
        read_to(gen, ctx, "t2", self.rhs())?;
        match self.op() {
            BinaryOp::Add => gen.binary("add", "t1", "t2", "t1")?,
            BinaryOp::Sub => gen.binary("sub", "t1", "t2", "t1")?,
            BinaryOp::Mul => gen.binary("mul", "t1", "t2", "t1")?,
            BinaryOp::Div => gen.binary("div", "t1", "t2", "t1")?,
            BinaryOp::Mod => gen.binary("rem", "t1", "t2", "t1")?,
            BinaryOp::And => gen.binary("and", "t1", "t2", "t1")?,
            BinaryOp::Or => gen.binary("or", "t1", "t2", "t1")?,
            BinaryOp::Lt => gen.binary("slt", "t1", "t2", "t1")?,
            BinaryOp::Gt => gen.binary("sgt", "t1", "t2", "t1")?,
            BinaryOp::Eq => {
                gen.binary("xor", "t1", "t2", "t1")?;
                gen.unary("seqz", "t1", "t1")?;
            }
            BinaryOp::NotEq => {
                gen.binary("xor", "t1", "t2", "t1")?;
                gen.unary("snez", "t1", "t1")?;
            }
            BinaryOp::Le => {
                gen.binary("sgt", "t1", "t2", "t1")?;
                gen.unary("seqz", "t1", "t1")?;
            }
            BinaryOp::Ge => {
                gen.binary("slt", "t1", "t2", "t1")?;
                gen.unary("seqz", "t1", "t1")?;
            }
            _ => unreachable!(),
        }
        // gen.binary(self.op(), "t1", "t2", "t1")?;
        write_back(gen, ctx, "t1", val)
    }
}

impl NonUnitGenerateAsm for GlobalAlloc {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()> {
        let id = ctx.get_global_var(&val);
        gen.global_alloc(id)?;
        self.init().generate(gen, ctx)
    }
}

impl NonUnitGenerateAsm for Call {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()> {
        self.args().iter().enumerate().try_for_each(|(i, &arg)| {
            if i < 8 {
                let dst = format!("a{}", i);
                read_to(gen, ctx, &dst, arg)
            } else {
                read_to(gen, ctx, "t0", arg)?;
                gen.sw("t0", "sp", (i as i32 - 8) * 4)
            }
        })?;
        let callee = &ctx.get_func_name(self.callee())[1..];
        gen.call(callee)?;
        // write the return value to pre-allocated space
        if let TypeKind::Function(_, ret_ty) = ctx.func_data(self.callee()).ty().kind() {
            if ret_ty.is_i32() && ctx.is_used(val) {
                write_back(gen, ctx, "a0", val)?;
            }
        }
        Ok(())
    }
}

impl NonUnitGenerateAsm for GetElemPtr {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()> {
        read_to(gen, ctx, "t1", self.index())?;

        let ty = ctx.value_ty(val);
        let stride = if let TypeKind::Pointer(base_ty) = ty.kind() {
            base_ty.size() as i32
        } else {
            unreachable!()
        };

        gen.li("t2", stride)?;
        gen.binary("mul", "t1", "t2", "t1")?;

        if ctx.is_pointer(self.src()) {
            read_to(gen, ctx, "t2", self.src())?;
        } else {
            read_addr_to(gen, ctx, "t2", self.src())?;
        }
        gen.binary("add", "t1", "t2", "t1")?;

        write_back(gen, ctx, "t1", val)
    }
}

impl NonUnitGenerateAsm for GetPtr {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()> {
        read_to(gen, ctx, "t1", self.index())?;

        let ty = ctx.value_ty(val);
        let stride = if let TypeKind::Pointer(base_ty) = ty.kind() {
            base_ty.size() as i32
        } else {
            unreachable!()
        };

        gen.li("t2", stride)?;
        gen.binary("mul", "t1", "t2", "t1")?;

        if ctx.is_pointer(self.src()) {
            read_to(gen, ctx, "t2", self.src())?;
        } else {
            read_addr_to(gen, ctx, "t2", self.src())?;
        }
        gen.binary("add", "t1", "t2", "t1")?;

        write_back(gen, ctx, "t1", val)
    }
}
