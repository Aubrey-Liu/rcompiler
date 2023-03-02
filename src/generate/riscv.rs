use crate::generate::ir;
use koopa::ir::values::Return;
use koopa::ir::ValueKind;
use koopa::ir::{FunctionData, Program};
use std::fs::File;
use std::io::Write;

pub trait GenerateAsm {
    fn generate(&self, f: &mut File);
}

impl GenerateAsm for Program {
    fn generate(&self, f: &mut File) {
        writeln!(f, "  .text").unwrap();
        for &func in self.func_layout() {
            self.func(func).generate(f);
        }
    }
}

impl GenerateAsm for FunctionData {
    fn generate(&self, f: &mut File) {
        writeln!(f, "  .globl {}", &self.name()[1..]).unwrap();
        writeln!(f, "{}:", &self.name()[1..]).unwrap();
        
        for (&_bb, node) in self.layout().bbs() {
            for &inst in node.insts().keys() {
                let value_data = self.dfg().value(inst);
                match value_data.kind() {
                    ValueKind::Return(i) => {
                        i.generate(f, &self);
                    }
                    _ => {}
                }
            }
        }
    }
}

trait InstGenerateAsm {
    fn generate(&self, f: &mut File, fib_data: &FunctionData);
}

impl InstGenerateAsm for Return {
    fn generate(&self, f: &mut File, fib_data: &FunctionData) {
        let ret_value = fib_data.dfg().value(self.value().unwrap());
        if let ValueKind::Integer(i) = ret_value.kind() {
            writeln!(f, "  li a0, {}", i.value()).unwrap();
        }
        writeln!(f, "  ret").unwrap();
    }
}

pub fn ir_to_riscv(ipath: &str, opath: &str) {
    let program = ir::into_mem_ir(&ipath);
    let mut f = File::create(&opath).unwrap();
    program.generate(&mut f);
}
