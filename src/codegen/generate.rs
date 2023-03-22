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
        let mut alloc = 0;

        for (&_bb, node) in self.layout().bbs() {
            for &inst in node.insts().keys() {
                let value_data = self.dfg().value(inst);
                if !value_data.ty().is_unit() {
                    store.insert(inst, alloc);
                    alloc += 4;
                }
            }
        }

        for (&bb, node) in self.layout().bbs() {
            let bb_name = get_bb_name(self, bb);
            writeln!(f, "{}:", bb_name)?;
            if bb_name == "entry" {
                writeln!(f, "  addi sp, sp, -{}", alloc)?;
            }
            for &inst in node.insts().keys() {
                inst.generate(f, self, &store, alloc)?;
            }
        }

        Ok(())
    }
}

trait ValueToAsm {
    fn generate<W: Write>(
        &self,
        asm: &mut W,
        func: &FunctionData,
        store: &LocalStore,
        alloc: i32,
    ) -> Result<()>;
}

impl ValueToAsm for Value {
    fn generate<W: Write>(
        &self,
        w: &mut W,
        func: &FunctionData,
        store: &LocalStore,
        alloc: i32,
    ) -> Result<()> {
        let value_data = func.dfg().value(*self);
        match value_data.kind() {
            ValueKind::Return(r) => r.generate(w, func, store, alloc)?,
            ValueKind::Store(s) => s.generate(w, func, store)?,
            ValueKind::Load(l) => l.generate(w, func, store, *self)?,
            ValueKind::Binary(b) => b.generate(w, func, store, *self)?,
            ValueKind::Jump(j) => j.generate(w, func, store)?,
            ValueKind::Branch(b) => b.generate(w, func, store)?,
            _ => {}
        }

        Ok(())
    }
}

trait UnitInstToAsm {
    fn generate<W: Write>(&self, w: &mut W, func: &FunctionData, store: &LocalStore) -> Result<()>;
}

impl UnitInstToAsm for Store {
    fn generate<W: Write>(&self, w: &mut W, func: &FunctionData, store: &LocalStore) -> Result<()> {
        let dst = store.get(&self.dest()).unwrap();
        self.value().load(w, func, store, "t1")?;
        writeln!(w, "  sw t1, {}(sp)", dst)?;

        Ok(())
    }
}

trait ReturnToAsm {
    fn generate<W: Write>(
        &self,
        w: &mut W,
        func: &FunctionData,
        store: &LocalStore,
        prologue: i32,
    ) -> Result<()>;
}

impl ReturnToAsm for Return {
    fn generate<W: Write>(
        &self,
        w: &mut W,
        func: &FunctionData,
        store: &LocalStore,
        prologue: i32,
    ) -> Result<()> {
        if self.value().is_none() {
            writeln!(w, "  li a0, 0")?;
        } else {
            self.value().unwrap().load(w, func, store, "a0")?;
        }
        writeln!(w, "  addi sp, sp, {}", prologue)?;
        writeln!(w, "  ret")?;

        Ok(())
    }
}

impl UnitInstToAsm for Jump {
    fn generate<W: Write>(
        &self,
        w: &mut W,
        func: &FunctionData,
        _store: &LocalStore,
    ) -> Result<()> {
        writeln!(w, "  j {}", get_bb_name(func, self.target()))?;

        Ok(())
    }
}

impl UnitInstToAsm for Branch {
    fn generate<W: Write>(&self, w: &mut W, func: &FunctionData, store: &LocalStore) -> Result<()> {
        self.cond().load(w, func, store, "t1")?;
        writeln!(w, "  bnez t1, {}", get_bb_name(func, self.true_bb()))?;
        writeln!(w, "  j {}", get_bb_name(func, self.false_bb()))?;

        Ok(())
    }
}

trait NonUnitInstToAsm {
    fn generate<W: Write>(
        &self,
        w: &mut W,
        func: &FunctionData,
        store: &LocalStore,
        save: Value,
    ) -> Result<()>;
}

impl NonUnitInstToAsm for Load {
    fn generate<W: Write>(
        &self,
        w: &mut W,
        func: &FunctionData,
        store: &LocalStore,
        save: Value,
    ) -> Result<()> {
        let save = store.get(&save).unwrap();
        self.src().load(w, func, store, "t1")?;
        writeln!(w, "  sw t1, {}(sp)", save)?;

        Ok(())
    }
}

impl NonUnitInstToAsm for Binary {
    fn generate<W: Write>(
        &self,
        w: &mut W,
        func: &FunctionData,
        store: &LocalStore,
        save: Value,
    ) -> Result<()> {
        self.lhs().load(w, func, store, "t1")?;
        self.rhs().load(w, func, store, "t2")?;
        let save = store.get(&save).unwrap();

        match self.op() {
            BinaryOp::Add => writeln!(w, "  add t1, t1, t2")?,
            BinaryOp::Sub => writeln!(w, "  sub t1, t1, t2")?,
            BinaryOp::Mul => writeln!(w, "  mul t1, t1, t2")?,
            BinaryOp::Div => writeln!(w, "  div t1, t1, t2")?,
            BinaryOp::Mod => writeln!(w, "  rem t1, t1, t2")?,
            BinaryOp::And => writeln!(w, "  and t1, t1, t2")?,
            BinaryOp::Or => writeln!(w, "  or t1, t1, t2")?,
            BinaryOp::Lt => writeln!(w, "  slt t1, t1, t2")?,
            BinaryOp::Gt => writeln!(w, "  sgt t1, t1, t2")?,
            BinaryOp::Eq => {
                writeln!(w, "  sub t1, t1, t2")?;
                writeln!(w, "  seqz t1, t1")?;
            }
            BinaryOp::NotEq => {
                writeln!(w, "  sub t1, t1, t2")?;
                writeln!(w, "  snez t1, t1")?;
            }
            BinaryOp::Le => {
                writeln!(w, "  sgt t1, t1, t2")?;
                writeln!(w, "  snez t1, t1")?;
            }
            BinaryOp::Ge => {
                writeln!(w, "  slt t1, t1, t2")?;
                writeln!(w, "  snez t1, t1")?;
            }
            _ => unreachable!(),
        }
        writeln!(w, "  sw t1, {}(sp)", save)?;

        Ok(())
    }
}

trait LoadValue {
    fn load<W: Write>(
        &self,
        asm: &mut W,
        func: &FunctionData,
        store: &LocalStore,
        dst: &str,
    ) -> Result<()>;
}

impl LoadValue for Value {
    fn load<W: Write>(
        &self,
        w: &mut W,
        func: &FunctionData,
        store: &LocalStore,
        dst: &str,
    ) -> Result<()> {
        let val = func.dfg().value(*self);
        if let ValueKind::Integer(i) = val.kind() {
            writeln!(w, "  li {}, {}", dst, i.value())?;
        } else {
            let src = store.get(self).unwrap();
            writeln!(w, "  lw {}, {}(sp)", dst, src)?;
        }

        Ok(())
    }
}
