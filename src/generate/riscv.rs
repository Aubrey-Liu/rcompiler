use crate::generate::ir;
use anyhow::{Ok, Result};
use koopa::ir::values::{Binary, Load, Return, Store};
use koopa::ir::{BinaryOp, FunctionData, Program};
use koopa::ir::{Value, ValueKind};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

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

        let mut store = HashMap::<Value, i32>::new();
        let mut asm = String::new();
        let mut local_alloc = 0;
        for (&_bb, node) in self.layout().bbs() {
            for &inst in node.insts().keys() {
                let value_data = self.dfg().value(inst);

                if !value_data.ty().is_unit() {
                    store.insert(inst, local_alloc);
                    local_alloc += 4;
                }

                inst.generate(&mut asm, self, &store)?;
            }
        }
        asm = format!("  addi sp, sp, -{}\n", local_alloc).to_owned() + asm.as_str();
        asm += format!("  addi sp, sp, {}\n", local_alloc).as_str();
        asm += "  ret\n";

        write!(f, "{}", asm.as_str())?;

        Ok(())
    }
}

trait ValueToAsm {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &HashMap<Value, i32>,
    ) -> Result<()>;
}

trait UnitInstToAsm {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &HashMap<Value, i32>,
    ) -> Result<()>;
}

trait NonUnitInstToAsm {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &HashMap<Value, i32>,
        save: Value,
    ) -> Result<()>;
}

impl ValueToAsm for Value {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &HashMap<Value, i32>,
    ) -> Result<()> {
        let value_data = func.dfg().value(*self);
        match value_data.kind() {
            ValueKind::Return(r) => r.generate(asm, func, store)?,
            ValueKind::Store(s) => s.generate(asm, func, store)?,
            ValueKind::Load(l) => l.generate(asm, func, store, *self)?,
            ValueKind::Binary(b) => b.generate(asm, func, store, *self)?,
            _ => {}
        }

        Ok(())
    }
}

impl NonUnitInstToAsm for Load {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &HashMap<Value, i32>,
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
        store: &HashMap<Value, i32>,
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
            BinaryOp::And => {
                asm.push_str("  snez t1, t1\n");
                asm.push_str("  snez t2, t2\n");
                asm.push_str("  and t1, t1, t2\n");
            }
            BinaryOp::Or => {
                asm.push_str("  snez t1, t1\n");
                asm.push_str("  snez t2, t2\n");
                asm.push_str("  or t1, t1, t2\n");
            }
            BinaryOp::Eq => {
                asm.push_str("  sub t1, t1, t2\n");
                asm.push_str("  seqz t1, t1\n");
            }
            BinaryOp::NotEq => {
                asm.push_str("  sub t1, t1, t2\n");
                asm.push_str("  snez t1, t1\n");
            }
            BinaryOp::Lt => asm.push_str("  slt t1, t1, t2\n"),
            BinaryOp::Le => {
                asm.push_str("  sgt t1, t1, t2\n");
                asm.push_str("  snez t1, t1\n");
            }
            BinaryOp::Gt => asm.push_str("  sgt t1, t1, t2\n"),
            BinaryOp::Ge => {
                asm.push_str("  slt t1, t1, t2\n");
                asm.push_str("  snez t1, t1\n");
            }
            _ => {}
        }
        asm.push_str(format!("  sw t1, {}(sp)\n", save).as_str());

        Ok(())
    }
}

impl UnitInstToAsm for Store {
    fn generate(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &HashMap<Value, i32>,
    ) -> Result<()> {
        let dst = store.get(&self.dest()).unwrap();
        self.value().load(asm, func, store, "t1")?;
        asm.push_str(format!("  sw t1, {}(sp)\n", dst).as_str());

        Ok(())
    }
}

impl UnitInstToAsm for Return {
    fn generate(&self, asm: &mut String, func: &FunctionData, store: &LocalStore) -> Result<()> {
        if self.value().is_none() {
            asm.push_str("  li a0, 0\n");
            return Ok(());
        }
        self.value().unwrap().load(asm, func, store, "a0")?;

        Ok(())
    }
}

trait LoadValue {
    fn load(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &HashMap<Value, i32>,
        dst: &str,
    ) -> Result<()>;
}

impl LoadValue for Value {
    fn load(
        &self,
        asm: &mut String,
        func: &FunctionData,
        store: &HashMap<Value, i32>,
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

pub fn ir_to_riscv(input: &str, opath: &str) -> Result<()> {
    let program = ir::into_mem_ir(input)?;
    let mut f = File::create(&opath)?;
    program.generate(&mut f)?;

    Ok(())
}
