use std::collections::HashMap;
use std::io::Write;

use super::*;

type LocalStore = HashMap<Value, i32>;

pub trait GenerateAsm {
    fn generate(&self, f: &mut File) -> Result<()>;
}

impl GenerateAsm for Program {
    fn generate(&self, f: &mut File) -> Result<()> {
        writeln!(f, "  .text")?;
        for &func in self.func_layout() {
            self.func(func).generate(f)?;
        }

        Ok(())
    }
}

impl GenerateAsm for FunctionData {
    fn generate(&self, f: &mut File) -> Result<()> {
        writeln!(f, "  .globl {}", &self.name()[1..])?;
        writeln!(f, "{}:", &self.name()[1..])?;

        let mut store = LocalStore::new();
        let mut asm = String::new();
        let mut alloc = 0;
        for (&bb, node) in self.layout().bbs() {
            asm.push_str(format!("{}:\n", get_bb_name(self, bb)).as_str());
            for &inst in node.insts().keys() {
                let value_data = self.dfg().value(inst);

                if !value_data.ty().is_unit() {
                    store.insert(inst, alloc);
                    alloc += 4;
                }

                inst.generate(&mut asm, self, &store, alloc)?;
            }
        }

        asm = format!("  addi sp, sp, -{}\n", alloc) + asm.as_str();
        write!(f, "{}", asm.as_str())?;

        Ok(())
    }
}

trait ValueToAsm {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &LocalStore,
        alloc: i32,
    ) -> Result<()>;
}

impl ValueToAsm for Value {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &LocalStore,
        alloc: i32,
    ) -> Result<()> {
        let value_data = func.dfg().value(*self);
        match value_data.kind() {
            ValueKind::Return(r) => r.generate(asm, func, store, alloc)?,
            ValueKind::Store(s) => s.generate(asm, func, store)?,
            ValueKind::Load(l) => l.generate(asm, func, store, *self)?,
            ValueKind::Binary(b) => b.generate(asm, func, store, *self)?,
            ValueKind::Jump(j) => j.generate(asm, func, store)?,
            ValueKind::Branch(b) => b.generate(asm, func, store)?,
            _ => {}
        }

        Ok(())
    }
}

trait UnitInstToAsm {
    fn generate(&self, asm: &mut String, func: &FunctionData, store: &LocalStore) -> Result<()>;
}

impl UnitInstToAsm for Store {
    fn generate(&self, asm: &mut String, func: &FunctionData, store: &LocalStore) -> Result<()> {
        let dst = store.get(&self.dest()).unwrap();
        self.value().load(asm, func, store, "t1")?;
        asm.push_str(format!("  sw t1, {}(sp)\n", dst).as_str());

        Ok(())
    }
}

trait ReturnToAsm {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &LocalStore,
        prologue: i32,
    ) -> Result<()>;
}

impl ReturnToAsm for Return {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &LocalStore,
        prologue: i32,
    ) -> Result<()> {
        if self.value().is_none() {
            asm.push_str("  li a0, 0\n");
        } else {
            self.value().unwrap().load(asm, func, store, "a0")?;
        }
        asm.push_str(format!("  addi sp, sp, {}\n", prologue).as_str());
        asm.push_str("  ret\n");

        Ok(())
    }
}

impl UnitInstToAsm for Jump {
    fn generate(&self, asm: &mut String, func: &FunctionData, _store: &LocalStore) -> Result<()> {
        asm.push_str(format!("  j {}\n", get_bb_name(func, self.target())).as_str());

        Ok(())
    }
}

impl UnitInstToAsm for Branch {
    fn generate(&self, asm: &mut String, func: &FunctionData, store: &LocalStore) -> Result<()> {
        self.cond().load(asm, func, store, "t1")?;
        asm.push_str(format!("  bnez t1, {}\n", get_bb_name(func, self.true_bb())).as_str());
        asm.push_str(format!("  j {}\n", get_bb_name(func, self.false_bb())).as_str());

        Ok(())
    }
}

trait NonUnitInstToAsm {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &LocalStore,
        save: Value,
    ) -> Result<()>;
}

impl NonUnitInstToAsm for Load {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &LocalStore,
        save: Value,
    ) -> Result<()> {
        let save = store.get(&save).unwrap();
        self.src().load(asm, func, store, "t1")?;
        asm.push_str(format!("  sw t1, {}(sp)\n", save).as_str());

        Ok(())
    }
}

impl NonUnitInstToAsm for Binary {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &LocalStore,
        save: Value,
    ) -> Result<()> {
        self.lhs().load(asm, func, store, "t1")?;
        self.rhs().load(asm, func, store, "t2")?;
        let save = store.get(&save).unwrap();

        match self.op() {
            BinaryOp::Add => asm.push_str("  add t1, t1, t2\n"),
            BinaryOp::Sub => asm.push_str("  sub t1, t1, t2\n"),
            BinaryOp::Mul => asm.push_str("  mul t1, t1, t2\n"),
            BinaryOp::Div => asm.push_str("  div t1, t1, t2\n"),
            BinaryOp::Mod => asm.push_str("  rem t1, t1, t2\n"),
            BinaryOp::And => asm.push_str("  and t1, t1, t2\n"),
            BinaryOp::Or => asm.push_str("  or t1, t1, t2\n"),
            BinaryOp::Lt => asm.push_str("  slt t1, t1, t2\n"),
            BinaryOp::Gt => asm.push_str("  sgt t1, t1, t2\n"),
            BinaryOp::Eq => {
                asm.push_str("  sub t1, t1, t2\n");
                asm.push_str("  seqz t1, t1\n");
            }
            BinaryOp::NotEq => {
                asm.push_str("  sub t1, t1, t2\n");
                asm.push_str("  snez t1, t1\n");
            }
            BinaryOp::Le => {
                asm.push_str("  sgt t1, t1, t2\n");
                asm.push_str("  snez t1, t1\n");
            }
            BinaryOp::Ge => {
                asm.push_str("  slt t1, t1, t2\n");
                asm.push_str("  snez t1, t1\n");
            }
            _ => unreachable!(),
        }
        asm.push_str(format!("  sw t1, {}(sp)\n", save).as_str());

        Ok(())
    }
}

trait LoadValue {
    fn load(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &LocalStore,
        dst: &str,
    ) -> Result<()>;
}

impl LoadValue for Value {
    fn load(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &LocalStore,
        dst: &str,
    ) -> Result<()> {
        let val = func.dfg().value(*self);
        if let ValueKind::Integer(i) = val.kind() {
            asm.push_str(format!("  li {}, {}\n", dst, i.value()).as_str());
        } else {
            let src = store.get(self).unwrap();
            asm.push_str(format!("  lw {}, {}(sp)\n", dst, src).as_str());
        }

        Ok(())
    }
}
