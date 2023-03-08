use crate::generate::ir;
use anyhow::{Ok, Result};
use koopa::ir::values::Return;
use koopa::ir::ValueKind;
use koopa::ir::{FunctionData, Program};
use std::fs::File;
use std::io::Write;

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

        for (&_bb, node) in self.layout().bbs() {
            for &inst in node.insts().keys() {
                let value_data = self.dfg().value(inst);
                match value_data.kind() {
                    ValueKind::Return(i) => {
                        i.generate(f, &self)?;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

trait InstGenerateAsm {
    fn generate(&self, f: &mut File, fib_data: &FunctionData) -> Result<()>;
}

impl InstGenerateAsm for Return {
    fn generate(&self, f: &mut File, fib_data: &FunctionData) -> Result<()> {
        let ret_value = fib_data.dfg().value(self.value().unwrap());
        if let ValueKind::Integer(i) = ret_value.kind() {
            writeln!(f, "  li a0, {}", i.value())?;
        }
        writeln!(f, "  ret")?;

        Ok(())
    }
}

pub fn ir_to_riscv(input: &str, opath: &str) -> Result<()> {
    let program = ir::into_mem_ir(input)?;
    let mut f = File::create(&opath)?;
    program.generate(&mut f)?;

    Ok(())
}
