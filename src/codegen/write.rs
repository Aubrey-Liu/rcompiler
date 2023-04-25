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
        writeln!(self.f, "  li {}, {}", dst.into_name(), imm)
    }

    pub fn mem_access(&mut self, method: &str, rd1: RegID, rd2: RegID, offset: i32) -> Result<()> {
        writeln!(
            self.f,
            "  {} {}, {}({})",
            method,
            rd1.into_name(),
            offset,
            rd2.into_name()
        )
    }

    pub fn lw(&mut self, dst: RegID, src: RegID, offset: i32) -> Result<()> {
        if (-2048..=2047).contains(&offset) {
            self.mem_access("lw", dst, src, offset)
        } else {
            self.binary_with_imm("add", dst, src, offset)?;
            self.mem_access("lw", dst, dst, 0)
        }
    }

    pub fn la(&mut self, dst: RegID, name: &str) -> Result<()> {
        writeln!(self.f, "  la {}, {}", dst.into_name(), name)
    }

    pub fn sw(&mut self, src: RegID, dst: RegID, offset: i32) -> Result<()> {
        if (-2048..=2047).contains(&offset) {
            self.mem_access("sw", src, dst, offset)
        } else {
            self.binary_with_imm("add", "t0".into_id(), dst, offset)?;
            self.mem_access("sw", src, "t0".into_id(), 0)
        }
    }

    pub fn j(&mut self, target: &str) -> Result<()> {
        writeln!(self.f, "  j {}", target)
    }

    pub fn bnez(&mut self, cond: RegID, target: &str) -> Result<()> {
        writeln!(self.f, "  bnez {}, {}", cond.into_name(), target)
    }

    pub fn unary(&mut self, op: &str, dst: RegID, opr: RegID) -> Result<()> {
        writeln!(self.f, "  {} {}, {}", op, dst.into_name(), opr.into_name())
    }

    pub fn binary(&mut self, op: &str, dst: RegID, lhs: RegID, rhs: RegID) -> Result<()> {
        writeln!(
            self.f,
            "  {} {}, {}, {}",
            op,
            dst.into_name(),
            lhs.into_name(),
            rhs.into_name()
        )
    }

    pub fn binary_with_imm(&mut self, op: &str, dst: RegID, opr: RegID, imm: i32) -> Result<()> {
        if (-2048..=2047).contains(&imm) {
            writeln!(
                self.f,
                "  {}i {}, {}, {}",
                op,
                dst.into_name(),
                opr.into_name(),
                imm
            )
        } else {
            self.li("t0".into_id(), imm)?;
            self.binary(op, dst, opr, "t0".into_id())
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
            AsmValue::Unary(op, dst, opr) => self.unary(op.asm_name(), *dst, *opr),
            AsmValue::Binary(op, dst, lhs, rhs) => self.binary(op.asm_name(), *dst, *lhs, *rhs),
            AsmValue::BinaryImm(op, dst, lhs, imm) => {
                self.binary_with_imm(op.asm_name(), *dst, *lhs, *imm)
            }
            AsmValue::Directive(directive) => self.directive(directive),
            AsmValue::Call(label) => self.call(label),
            AsmValue::Branch(cond, target) => self.bnez(*cond, target),
            AsmValue::Jump(target) => self.j(target),
            AsmValue::LocalSymbol(label) => self.local_symbol(label),
            AsmValue::GlobalSymbol(label) => self.global_symbol(label),
            AsmValue::Return => self.ret(),
        })
    }
}
