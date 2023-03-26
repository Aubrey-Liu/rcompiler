use std::io::Result;

use super::*;

pub trait GenerateAsm {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()>;
}

impl GenerateAsm for Program {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        for &func in self.func_layout() {
            ctx.set_func(func);
            self.func(func).generate(gen, ctx)?;
        }

        Ok(())
    }
}

impl GenerateAsm for FunctionData {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        gen.enter_func(&self.name()[1..])?;

        let mut off = 0;
        for (&val, data) in self.dfg().values() {
            if data.kind().is_local_inst() && !data.used_by().is_empty() {
                ctx.cur_func_mut().register_var(val, off);
                off += 4;
            }
        }

        for (&bb, _) in self.dfg().bbs() {
            ctx.register_bb(bb);
        }

        // align stack size to 16
        let ss = (off + 15) / 16 * 16;
        ctx.cur_func_mut().set_ss(ss);

        for (&bb, node) in self.layout().bbs() {
            gen.enter_bb(ctx, bb)?;
            if bb == self.layout().entry_bb().unwrap() {
                gen.prologue(ctx)?;
            }
            for &inst in node.insts().keys() {
                inst.generate(gen, ctx)?;
            }
        }

        Ok(())
    }
}

impl GenerateAsm for Value {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        let value_data = ctx.func_data().dfg().value(*self);
        let used_by = !ctx.func_data().dfg().value(*self).used_by().is_empty();
        match value_data.kind().clone() {
            ValueKind::Branch(v) => v.generate(gen, ctx),
            ValueKind::Jump(v) => v.generate(gen, ctx),
            ValueKind::Return(v) => v.generate(gen, ctx),
            ValueKind::Store(v) => v.generate(gen, ctx),
            ValueKind::Binary(v) => {
                v.generate(gen, ctx)?;
                if used_by {
                    gen.store(ctx, "t1", *self)
                } else {
                    Ok(())
                }
            }
            ValueKind::Load(v) => {
                v.generate(gen, ctx)?;
                gen.store(ctx, "t1", *self)
            }
            _ => Ok(()),
        }
    }
}

impl GenerateAsm for Load {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        gen.load(ctx, "t1", self.src())
    }
}

impl GenerateAsm for Store {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        gen.load(ctx, "t1", self.value())?;
        gen.store(ctx, "t1", self.dest())
    }
}

impl GenerateAsm for Jump {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        gen.jump(ctx.cur_func().get_bb_name(self.target()))
    }
}

impl GenerateAsm for Branch {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        gen.load(ctx, "t1", self.cond())?;
        gen.branch(ctx, "t1", self.true_bb(), self.false_bb())
    }
}

impl GenerateAsm for Binary {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        gen.load(ctx, "t1", self.lhs())?;
        gen.load(ctx, "t2", self.rhs())?;
        gen.binary(self.op(), "t1", "t2", "t1")
    }
}

impl GenerateAsm for Return {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        if self.value().is_some() {
            gen.load(ctx, "a0", self.value().unwrap())?;
        }
        gen.epilogue(ctx)
    }
}
