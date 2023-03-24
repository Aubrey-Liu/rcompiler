use std::io::{Result, Write};

use super::*;

pub struct AsmGenerator<'a> {
    f: File,
    tmpr: &'a str,
}

impl<'a> AsmGenerator<'a> {
    pub fn addi(&mut self, dst: &str, opr: &str, imm: i32) -> Result<()> {
        if imm >= -2048 && imm <= 2047 {
            writeln!(self.f, "  addi {}, {}, {}", dst, opr, imm)
        } else {
            writeln!(self.f, "  li {}, {}", self.tmpr, imm)?;
            writeln!(self.f, "  add, {}, {}, {}", dst, opr, self.tmpr)
        }
    }

    pub fn loadi(&mut self, dst: &str, imm: i32) -> Result<()> {
        writeln!(self.f, "  li {}, {}", dst, imm)
    }

    pub fn prologue(&mut self, ctx: &Context) -> Result<()> {
        writeln!(self.f, "entry:")?;
        self.addi("sp", "sp", -ctx.curr_func().ss())
    }

    pub fn enter_bb(&mut self, ctx: &Context, bb: BasicBlock) -> Result<()> {
        writeln!(self.f, "{}:", ctx.get_bb_name(bb))
    }

    pub fn enter_func(&mut self, func_name: &str) -> Result<()> {
        writeln!(self.f, "  .globl {}", func_name)?;
        writeln!(self.f, "{}:", func_name)
    }

    pub fn epilogue(&mut self, ctx: &Context) -> Result<()> {
        self.addi("sp", "sp", ctx.curr_func().ss())?;
        writeln!(self.f, "  ret")
    }

    pub fn jump(&mut self, target: &String) -> Result<()> {
        writeln!(self.f, "  j {}", target)
    }

    pub fn load(&mut self, ctx: &Context, dst: &str, val: Value) -> Result<()> {
        if let ValueKind::Integer(imm) = ctx.value_kind(val) {
            return self.loadi(dst, imm.value());
        }
        let off = ctx.curr_func().get_offset(&val);
        if off >= -2048 && off <= 2047 {
            writeln!(self.f, "  lw {}, {}(sp)", dst, off)
        } else {
            writeln!(self.f, "  li {}, {}", self.tmpr, off)?;
            writeln!(self.f, "  add {}, {}, sp", self.tmpr, self.tmpr)?;
            writeln!(self.f, "  lw {}, 0({})", dst, self.tmpr)
        }
    }

    pub fn store(&mut self, ctx: &Context, src: &str, val: Value) -> Result<()> {
        let off = ctx.curr_func().get_offset(&val);

        if off >= -2048 && off <= 2047 {
            writeln!(self.f, "  sw {}, {}(sp)", src, off)
        } else {
            writeln!(self.f, "  li {}, {}", self.tmpr, off)?;
            writeln!(self.f, "  add {}, {}, sp", self.tmpr, self.tmpr)?;
            writeln!(self.f, "  sw {}, 0({})", src, self.tmpr)
        }
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

    pub fn branch(
        &mut self,
        ctx: &Context,
        cond: &str,
        true_bb: BasicBlock,
        false_bb: BasicBlock,
    ) -> Result<()> {
        writeln!(self.f, "  bnez {}, {}", cond, ctx.get_bb_name(true_bb))?;
        writeln!(self.f, "  j {}", ctx.get_bb_name(false_bb))
    }

    pub fn text(&mut self) -> Result<()> {
        writeln!(self.f, "  .text")
    }

    pub fn from_path(path: &str, tmpr: &'a str) -> Result<Self> {
        Ok(Self {
            f: File::create(path)?,
            tmpr,
        })
    }
}
