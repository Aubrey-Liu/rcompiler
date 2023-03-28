use std::cmp::max;
use std::io::Result;

use super::*;

pub trait GenerateAsm {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()>;
}

impl GenerateAsm for Program {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        self.inst_layout().iter().for_each(|&g| {
            if self.borrow_value(g).kind().is_global_alloc() {
                ctx.register_global_var(g);
                ctx.global_alloc(gen, g);
            }
        });
        self.funcs()
            .iter()
            .filter(|(_, data)| data.layout().entry_bb().is_some())
            .try_for_each(|(&f, data)| {
                ctx.new_func(f);
                ctx.set_func(f);
                data.generate(gen, ctx)
            })
    }
}

impl GenerateAsm for FunctionData {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        let max_arg_num = self
            .dfg()
            .values()
            .iter()
            .filter_map(|(_, data)| {
                if let ValueKind::Call(c) = data.kind() {
                    Some(c.args().len())
                } else {
                    None
                }
            })
            .max();
        let is_leaf = max_arg_num.is_none();
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
            .for_each(|(&val, _)| {
                ctx.cur_func_mut().register_var(val, off);
                off += 4;
            });
        self.dfg()
            .bbs()
            .iter()
            .for_each(|(&bb, _)| ctx.register_bb(bb));

        // align stack size to 16
        let ss = (off + 4 + 15) / 16 * 16;
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
        let data = ctx.cur_func_data().dfg().value(*self);
        match data.kind().clone() {
            ValueKind::Branch(v) => v.generate(gen, ctx),
            ValueKind::Jump(v) => v.generate(gen, ctx),
            ValueKind::Return(v) => v.generate(gen, ctx),
            ValueKind::Store(v) => v.generate(gen, ctx),
            ValueKind::Binary(v) => v.generate(gen, ctx, *self),
            ValueKind::Load(v) => v.generate(gen, ctx, *self),
            ValueKind::Call(v) => v.generate(gen, ctx, *self),
            _ => Ok(()),
        }
    }
}

impl NonUnitGenerateAsm for GlobalAlloc {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()> {
        let init = ctx.global_init(self.init());
        let id = ctx.get_global_var(&val);
        gen.global_alloc(init, &id)
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
                write_to(gen, ctx, "a0", val)?;
            }
        }
        Ok(())
    }
}

impl GenerateAsm for Store {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        read_to(gen, ctx, "t1", self.value())?;
        write_to(gen, ctx, "t1", self.dest())
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

pub trait NonUnitGenerateAsm {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()>;
}

impl NonUnitGenerateAsm for Load {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()> {
        if !ctx.is_used(val) {
            return Ok(());
        }
        read_to(gen, ctx, "t1", self.src())?;
        write_to(gen, ctx, "t1", val)
    }
}

impl NonUnitGenerateAsm for Binary {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context, val: Value) -> Result<()> {
        if !ctx.is_used(val) {
            return Ok(());
        }

        read_to(gen, ctx, "t1", self.lhs())?;
        read_to(gen, ctx, "t2", self.rhs())?;
        gen.binary(self.op(), "t1", "t2", "t1")?;
        write_to(gen, ctx, "t1", val)
    }
}
