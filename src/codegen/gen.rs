use std::io::Result;

use super::*;

pub trait GenerateAsm {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()>;
}

impl GenerateAsm for Program {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
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
        gen.enter_func(&self.name()[1..])?;

        let mut off = 0;
        self.dfg()
            .values()
            .iter()
            .filter(|(_, data)| data.kind().is_local_inst())
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

        for (bb, node) in self.layout().bbs() {
            gen.enter_bb(ctx.cur_func().get_bb_name(bb))?;
            if *bb == self.layout().entry_bb().unwrap() {
                prologue(gen, ctx)?;
            }
            node.insts()
                .keys()
                .try_for_each(|inst| inst.generate(gen, ctx))?;
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
                    gen.store("t1", "sp", ctx.cur_func().get_offset(self))
                } else {
                    Ok(())
                }
            }
            ValueKind::Load(v) => {
                v.generate(gen, ctx)?;
                gen.store("t1", "sp", ctx.cur_func().get_offset(self))
            }
            _ => Ok(()),
        }
    }
}

impl GenerateAsm for Load {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        load(gen, ctx, "t1", self.src())
    }
}

impl GenerateAsm for Store {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        load(gen, ctx, "t1", self.value())?;
        gen.store("t1", "sp", ctx.cur_func().get_offset(&self.dest()))
    }
}

impl GenerateAsm for Jump {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        gen.jump(ctx.cur_func().get_bb_name(&self.target()))
    }
}

impl GenerateAsm for Branch {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        load(gen, ctx, "t1", self.cond())?;
        branch(gen, ctx, "t1", &self.true_bb(), &self.false_bb())
    }
}

impl GenerateAsm for Binary {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        load(gen, ctx, "t1", self.lhs())?;
        load(gen, ctx, "t2", self.rhs())?;
        gen.binary(self.op(), "t1", "t2", "t1")
    }
}

impl GenerateAsm for Return {
    fn generate(&self, gen: &mut AsmGenerator, ctx: &mut Context) -> Result<()> {
        if self.value().is_some() {
            load(gen, ctx, "a0", self.value().unwrap())?;
        }
        epilogue(gen, ctx)
    }
}
