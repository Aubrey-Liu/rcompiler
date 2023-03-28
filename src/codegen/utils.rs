use std::io::{Result, Write};

use super::*;

pub struct AsmGenerator<'a> {
    f: File,
    // temporary registor name
    tmpr: &'a str,
}

impl<'a> AsmGenerator<'a> {
    pub fn addi(&mut self, dst: &str, opr: &str, imm: i32) -> Result<()> {
        if imm >= -2048 && imm <= 2047 {
            writeln!(self.f, "  addi {}, {}, {}", dst, opr, imm)
        } else {
            self.li(self.tmpr, imm)?;
            writeln!(self.f, "  add, {}, {}, {}", dst, opr, self.tmpr)
        }
    }

    pub fn li(&mut self, dst: &str, imm: i32) -> Result<()> {
        writeln!(self.f, "  li {}, {}", dst, imm)
    }

    pub fn lw(&mut self, dst: &str, src: &str, off: i32) -> Result<()> {
        if off >= -2048 && off <= 2047 {
            writeln!(self.f, "  lw {}, {}({})", dst, off, src)
        } else {
            self.addi(self.tmpr, src, off)?;
            writeln!(self.f, "  lw {}, 0({})", dst, self.tmpr)
        }
    }

    pub fn sw(&mut self, src: &str, dst: &str, off: i32) -> Result<()> {
        if off >= -2048 && off <= 2047 {
            writeln!(self.f, "  sw {}, {}({})", src, off, dst)
        } else {
            self.addi(self.tmpr, dst, off)?;
            writeln!(self.f, "  sw {}, 0({})", src, self.tmpr)
        }
    }

    pub fn j(&mut self, target: &String) -> Result<()> {
        writeln!(self.f, "  j {}", target)
    }

    pub fn bnez(&mut self, cond: &str, target: &str) -> Result<()> {
        writeln!(self.f, "  bnez {}, {}", cond, target)
    }

    pub fn binary(&mut self, op: BinaryOp, lhs: &str, rhs: &str, dst: &str) -> Result<()> {
        match op {
            BinaryOp::Add => writeln!(self.f, "  add {}, {}, {}", dst, lhs, rhs),
            BinaryOp::Sub => writeln!(self.f, "  sub {}, {}, {}", dst, lhs, rhs),
            BinaryOp::Mul => writeln!(self.f, "  mul {}, {}, {}", dst, lhs, rhs),
            BinaryOp::Div => writeln!(self.f, "  div {}, {}, {}", dst, lhs, rhs),
            BinaryOp::Mod => writeln!(self.f, "  rem {}, {}, {}", dst, lhs, rhs),
            BinaryOp::And => writeln!(self.f, "  and {}, {}, {}", dst, lhs, rhs),
            BinaryOp::Or => writeln!(self.f, "  or {}, {}, {}", dst, lhs, rhs),
            BinaryOp::Lt => writeln!(self.f, "  slt {}, {}, {}", dst, lhs, rhs),
            BinaryOp::Gt => writeln!(self.f, "  sgt {}, {}, {}", dst, lhs, rhs),
            BinaryOp::Eq => {
                writeln!(self.f, "  xor {}, {}, {}", lhs, lhs, rhs)?;
                writeln!(self.f, "  seqz {}, {}", dst, lhs)
            }
            BinaryOp::NotEq => {
                writeln!(self.f, "  xor {}, {}, {}", lhs, lhs, rhs)?;
                writeln!(self.f, "  snez {}, {}", dst, lhs)
            }
            BinaryOp::Le => {
                writeln!(self.f, "  sgt {}, {}, {}", lhs, lhs, rhs)?;
                writeln!(self.f, "  seqz {}, {}", dst, lhs)
            }
            BinaryOp::Ge => {
                writeln!(self.f, "  slt {}, {}, {}", lhs, lhs, rhs)?;
                writeln!(self.f, "  seqz {}, {}", dst, lhs)
            }
            _ => unreachable!(),
        }
    }

    pub fn ret(&mut self) -> Result<()> {
        writeln!(self.f, "  ret")
    }

    pub fn enter_bb(&mut self, name: &str) -> Result<()> {
        writeln!(self.f, "{}:", name)
    }

    pub fn prologue(&mut self, func_name: &str, ss: i32) -> Result<()> {
        writeln!(self.f, "  .text")?;
        writeln!(self.f, "  .globl {}", func_name)?;
        writeln!(self.f, "{}:", func_name)?;
        self.addi("sp", "sp", -ss)?;
        // todo: only protect ra if it's not a leaf function
        self.sw("ra", "sp", ss - 4)
    }

    pub fn epilogue(&mut self, ss: i32) -> Result<()> {
        self.lw("ra", "sp", ss - 4)?;
        self.addi("sp", "sp", ss)?;
        self.ret()?;
        writeln!(self.f)
    }

    pub fn from_path(path: &str, tmpr: &'a str) -> Self {
        Self {
            f: File::create(path).unwrap(),
            tmpr,
        }
    }
}

pub fn read_to(gen: &mut AsmGenerator, ctx: &Context, dst: &str, val: Value) -> Result<()> {
    if let ValueKind::Integer(imm) = ctx.value_kind(val) {
        gen.li(dst, imm.value())
    } else {
        gen.lw(dst, "sp", ctx.cur_func().get_offset(&val))
    }
}

pub fn branch(
    gen: &mut AsmGenerator,
    ctx: &Context,
    cond: &str,
    true_bb: &BasicBlock,
    false_bb: &BasicBlock,
) -> Result<()> {
    let true_bb = ctx.cur_func().get_bb_name(true_bb);
    let false_bb = ctx.cur_func().get_bb_name(false_bb);
    gen.bnez(cond, true_bb)?;
    gen.j(false_bb)
}
