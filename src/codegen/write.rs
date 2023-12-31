use std::io::BufWriter;
use std::io::{Result, Write};

use super::program::{AsmProgram, AsmValue};
use super::*;

pub struct AsmWriter {
    f: BufWriter<File>,
}

impl AsmWriter {
    pub fn segment(&mut self, name: &str) -> Result<()> {
        writeln!(self.f, "  .{}", name)
    }

    pub fn li(&mut self, dst: RegID, imm: i32) -> Result<()> {
        writeln!(self.f, "  li {}, {}", dst, imm)
    }

    pub fn mem_access(&mut self, method: &str, rd1: RegID, rd2: RegID, offset: i32) -> Result<()> {
        writeln!(self.f, "  {} {}, {}({})", method, rd1, offset, rd2)
    }

    pub fn lw(&mut self, dst: RegID, src: RegID, offset: i32) -> Result<()> {
        if (-2048..=2047).contains(&offset) {
            self.mem_access("lw", dst, src, offset)
        } else {
            self.binary_with_imm(&AsmBinaryOp::Addi, dst, src, offset)?;
            self.mem_access("lw", dst, dst, 0)
        }
    }

    pub fn la(&mut self, dst: RegID, name: &str) -> Result<()> {
        writeln!(self.f, "  la {}, {}", dst, name)
    }

    pub fn sw(&mut self, src: RegID, dst: RegID, offset: i32) -> Result<()> {
        if (-2048..=2047).contains(&offset) {
            self.mem_access("sw", src, dst, offset)
        } else {
            let tmp = "t0".into_id();
            self.binary_with_imm(&AsmBinaryOp::Addi, tmp, dst, offset)?;
            self.mem_access("sw", src, tmp, 0)
        }
    }

    pub fn j(&mut self, target: &str) -> Result<()> {
        writeln!(self.f, "  j {}", target)
    }

    pub fn branch(&mut self, op: &BranchOp, lhs: RegID, rhs: RegID, target: &str) -> Result<()> {
        match op {
            BranchOp::Bnez => writeln!(self.f, "  bnez {}, {}", lhs, target),
            BranchOp::Beqz => writeln!(self.f, "  beqz {}, {}", lhs, target),
            _ => {
                writeln!(self.f, "  {} {}, {}, {}", op, lhs, rhs, target)
            }
        }
    }

    pub fn unary(&mut self, op: &AsmUnaryOp, dst: RegID, opr: RegID) -> Result<()> {
        writeln!(self.f, "  {} {}, {}", op, dst, opr)
    }

    pub fn binary(&mut self, op: &AsmBinaryOp, dst: RegID, lhs: RegID, rhs: RegID) -> Result<()> {
        writeln!(self.f, "  {} {}, {}, {}", op, dst, lhs, rhs)
    }

    pub fn binary_with_imm(
        &mut self,
        op: &AsmBinaryOp,
        dst: RegID,
        opr: RegID,
        imm: i32,
    ) -> Result<()> {
        if (-2048..=2047).contains(&imm) {
            writeln!(self.f, "  {} {}, {}, {}", op, dst, opr, imm)
        } else {
            let tmp = if dst == "sp".into_id() {
                "t0".into_id()
            } else {
                dst
            };
            self.li(tmp, imm)?;
            self.binary(&op.into_reg_type(), dst, opr, tmp)
        }
    }

    pub fn call(&mut self, callee: &str) -> Result<()> {
        writeln!(self.f, "  call {}", callee)
    }

    pub fn ret(&mut self) -> Result<()> {
        writeln!(self.f, "  ret")
    }

    pub fn local_symbol(&mut self, name: &str) -> Result<()> {
        writeln!(self.f, "{}:", name)
    }

    pub fn global_symbol(&mut self, name: &str) -> Result<()> {
        writeln!(self.f, "  .globl {}", name)?;
        writeln!(self.f, "{}:", name)
    }

    pub fn zero(&mut self, size: usize) -> Result<()> {
        writeln!(self.f, "  .zero {}", size)
    }

    pub fn word(&mut self, val: i32) -> Result<()> {
        writeln!(self.f, "  .word {}", val)
    }

    pub fn from_path(path: &str) -> Self {
        Self {
            f: BufWriter::new(File::create(path).unwrap()),
        }
    }

    fn directive(&mut self, directive: &Directive) -> Result<()> {
        match directive {
            Directive::Data => self.segment("data"),
            Directive::Text => self.segment("text"),
            Directive::Word(val) => self.word(*val),
            Directive::Zero(size) => self.zero(*size),
        }
    }

    pub fn write_program(&mut self, program: &AsmProgram) -> Result<()> {
        program.values.iter().try_for_each(|value| match value {
            AsmValue::LoadAddress(dst, lable) => self.la(*dst, lable),
            AsmValue::LoadImm(dst, imm) => self.li(*dst, *imm),
            AsmValue::Load(dst, src, offset) => self.lw(*dst, *src, *offset),
            AsmValue::Store(src, dst, offset) => self.sw(*src, *dst, *offset),
            AsmValue::Unary(op, dst, opr) => self.unary(op, *dst, *opr),
            AsmValue::Binary(op, dst, lhs, rhs) => self.binary(op, *dst, *lhs, *rhs),
            AsmValue::BinaryImm(op, dst, lhs, imm) => self.binary_with_imm(op, *dst, *lhs, *imm),
            AsmValue::Directive(directive) => self.directive(directive),
            AsmValue::Call(label) => self.call(label),
            AsmValue::Branch(op, lhs, rhs, target) => self.branch(op, *lhs, *rhs, target),
            AsmValue::Jump(target) => self.j(target),
            AsmValue::LocalSymbol(label) => self.local_symbol(label),
            AsmValue::GlobalSymbol(label) => self.global_symbol(label),
            AsmValue::Return => self.ret(),
        })
    }
}
