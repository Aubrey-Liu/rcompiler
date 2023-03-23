use std::io::Result;

use super::*;

pub trait GenerateAsm {
    fn generate(&self, gen: &mut AsmGenerator, program: &mut ProgramStat) -> Result<()>;
}

impl GenerateAsm for Program {
    fn generate(&self, gen: &mut AsmGenerator, program: &mut ProgramStat) -> Result<()> {
        gen.text()?;
        for &func in self.func_layout() {
            program.set_func(func);
            self.func(func).generate(gen, program)?;
        }

        Ok(())
    }
}

impl GenerateAsm for FunctionData {
    fn generate(&self, gen: &mut AsmGenerator, program: &mut ProgramStat) -> Result<()> {
        gen.enter_func(&self.name()[1..])?;

        let mut off = 0;
        for (&val, data) in self.dfg().values() {
            if !data.ty().is_unit() && !matches!(data.kind(), ValueKind::Load(_)) {
                program.curr_func_mut().register_inst(val, off);
                off += 4;
            }
        }

        // align to 16
        let alloc = (off + 15) / 16 * 16;
        program.curr_func_mut().set_ss(alloc);

        for (&bb, node) in self.layout().bbs() {
            if bb == self.layout().entry_bb().unwrap() {
                gen.prologue(program)?;
            } else {
                gen.enter_bb(program, bb)?;
            }
            for &inst in node.insts().keys() {
                inst.generate(gen, program)?;
            }
        }

        Ok(())
    }
}

impl GenerateAsm for Value {
    fn generate(&self, gen: &mut AsmGenerator, program: &mut ProgramStat) -> Result<()> {
        let value_data = program.func_data().dfg().value(*self);
        match value_data.kind().clone() {
            ValueKind::Binary(b) => {
                b.generate(gen, program)?;
                gen.store(program, "t1", *self)
            }
            ValueKind::Branch(b) => b.generate(gen, program),
            ValueKind::Jump(j) => j.generate(gen, program),
            ValueKind::Return(r) => r.generate(gen, program),
            ValueKind::Store(s) => s.generate(gen, program),
            _ => Ok(()),
        }
    }
}

impl GenerateAsm for Store {
    fn generate(&self, gen: &mut AsmGenerator, program: &mut ProgramStat) -> Result<()> {
        gen.load(program, "t1", self.value())?;
        gen.store(program, "t1", self.dest())
    }
}

impl GenerateAsm for Jump {
    fn generate(&self, gen: &mut AsmGenerator, program: &mut ProgramStat) -> Result<()> {
        gen.jump(get_bb_name(program, self.target()))
    }
}

impl GenerateAsm for Branch {
    fn generate(&self, gen: &mut AsmGenerator, program: &mut ProgramStat) -> Result<()> {
        gen.load(program, "t1", self.cond())?;
        gen.branch(program, "t1", self.true_bb(), self.false_bb())
    }
}

impl GenerateAsm for Binary {
    fn generate(&self, gen: &mut AsmGenerator, program: &mut ProgramStat) -> Result<()> {
        gen.load(program, "t1", self.lhs())?;
        gen.load(program, "t2", self.rhs())?;
        gen.binary(self.op(), "t1", "t2", "t1")
    }
}

impl GenerateAsm for Return {
    fn generate(&self, gen: &mut AsmGenerator, program: &mut ProgramStat) -> Result<()> {
        if self.value().is_none() {
            gen.loadi("a0", 0)?;
        } else {
            gen.load(program, "a0", self.value().unwrap())?;
        }
        gen.epilogue(program)
    }
}
