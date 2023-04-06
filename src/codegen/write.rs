use std::io::BufWriter;
use std::io::{Result, Write};

use super::*;

pub struct AsmGenerator<'a> {
    f: BufWriter<File>,
    temp_reg: &'a str,
}

impl<'a> AsmGenerator<'a> {
    pub fn flush(&mut self) -> Result<()> {
        self.f.flush()
    }

    pub fn blank_line(&mut self) -> Result<()> {
        writeln!(self.f)
    }

    pub fn data_seg(&mut self) -> Result<()> {
        writeln!(self.f, "  .data")
    }

    pub fn addi(&mut self, dst: &str, opr: &str, imm: i32) -> Result<()> {
        if (-2048..=2047).contains(&imm) {
            writeln!(self.f, "  addi {}, {}, {}", dst, opr, imm)
        } else {
            self.li(self.temp_reg, imm)?;
            writeln!(self.f, "  add {}, {}, {}", dst, opr, self.temp_reg)
        }
    }

    pub fn li(&mut self, dst: &str, imm: i32) -> Result<()> {
        writeln!(self.f, "  li {}, {}", dst, imm)
    }

    pub fn lw(&mut self, dst: &str, src: &str, off: i32) -> Result<()> {
        if (-2048..=2047).contains(&off) {
            writeln!(self.f, "  lw {}, {}({})", dst, off, src)
        } else {
            self.addi(self.temp_reg, src, off)?;
            writeln!(self.f, "  lw {}, 0({})", dst, self.temp_reg)
        }
    }

    pub fn la(&mut self, dst: &str, src: &str) -> Result<()> {
        writeln!(self.f, "  la {}, {}", dst, src)
    }

    pub fn mv(&mut self, dst: &str, src: &str) -> Result<()> {
        writeln!(self.f, "  mv {}, {}", dst, src)
    }

    pub fn sw(&mut self, src: &str, dst: &str, off: i32) -> Result<()> {
        if (-2048..=2047).contains(&off) {
            writeln!(self.f, "  sw {}, {}({})", src, off, dst)
        } else {
            self.addi(self.temp_reg, dst, off)?;
            writeln!(self.f, "  sw {}, 0({})", src, self.temp_reg)
        }
    }

    pub fn j(&mut self, target: &String) -> Result<()> {
        writeln!(self.f, "  j {}", target)
    }

    pub fn bnez(&mut self, cond: &str, target: &str) -> Result<()> {
        writeln!(self.f, "  bnez {}, {}", cond, target)
    }

    pub fn call(&mut self, callee: &str) -> Result<()> {
        writeln!(self.f, "  call {}", callee)
    }

    pub fn binary(&mut self, op: &str, dst: &str, lhs: &str, rhs: &str) -> Result<()> {
        writeln!(self.f, "  {} {}, {}, {}", op, dst, lhs, rhs)
    }

    pub fn muli(&mut self, dst: &str, lhs: &str, imm: i32) -> Result<()> {
        if imm == 0 {
            self.mv(dst, "x0")
        } else if imm > 0 && (imm & (imm - 1)) == 0 {
            let mut shift = 0;
            let mut imm = imm >> 1;
            while imm != 0 {
                shift += 1;
                imm >>= 1;
            }
            self.slli(dst, lhs, shift)
        } else {
            self.li(self.temp_reg, imm)?;
            self.binary("mul", dst, lhs, self.temp_reg)
        }
    }

    pub fn slli(&mut self, dst: &str, lhs: &str, imm: i32) -> Result<()> {
        writeln!(self.f, "  slli {}, {}, {}", dst, lhs, imm)
    }

    pub fn unary(&mut self, op: &str, dst: &str, opr: &str) -> Result<()> {
        writeln!(self.f, "  {} {}, {}", op, dst, opr)
    }

    pub fn ret(&mut self) -> Result<()> {
        writeln!(self.f, "  ret")
    }

    pub fn global_alloc(&mut self, id: usize) -> Result<()> {
        writeln!(self.f, "  .globl var{}", id)?;
        writeln!(self.f, "var{}:", id)
    }

    pub fn global_zero_init(&mut self, size: usize) -> Result<()> {
        writeln!(self.f, "  .zero {}", size)
    }

    pub fn global_word(&mut self, val: i32) -> Result<()> {
        writeln!(self.f, "  .word {}", val)
    }

    pub fn enter_bb(&mut self, name: &str) -> Result<()> {
        writeln!(self.f, "{}:", name)
    }

    pub fn prologue(&mut self, func_name: &str, ss: i32, is_leaf: bool) -> Result<()> {
        writeln!(self.f, "  .text")?;
        writeln!(self.f, "  .globl {}", func_name)?;
        writeln!(self.f, "{}:", func_name)?;
        self.addi("sp", "sp", -ss)?;
        if !is_leaf {
            self.sw("ra", "sp", ss - 4)
        } else {
            Ok(())
        }
    }

    pub fn epilogue(&mut self, ss: i32, is_leaf: bool) -> Result<()> {
        if !is_leaf {
            self.lw("ra", "sp", ss - 4)?;
        }
        self.addi("sp", "sp", ss)?;
        self.ret()?;
        writeln!(self.f)
    }

    pub fn from_path(path: &str, temp_reg: &'a str) -> Self {
        Self {
            f: BufWriter::new(File::create(path).unwrap()),
            temp_reg,
        }
    }
}

pub fn write_back(gen: &mut AsmGenerator, ctx: &Context, src: &str, val: Value) -> Result<()> {
    if ctx.is_global(val) {
        let id = ctx.get_global_var(&val);
        let name = format!("var{}", id);
        gen.la("t0", &name)?;
        gen.sw(src, "t0", 0)
    } else {
        gen.sw(src, "sp", ctx.cur_func().get_offset(&val))
    }
}

pub fn read_to(gen: &mut AsmGenerator, ctx: &Context, dst: &str, val: Value) -> Result<()> {
    if ctx.is_global(val) {
        let id = ctx.get_global_var(&val);
        let name = format!("var{}", id);
        gen.la("t0", &name)?;
        return gen.lw(dst, "t0", 0);
    }
    match ctx.cur_func().params().get(&val) {
        Some(id) => {
            if *id < 8 {
                let src = format!("a{}", id);
                gen.mv(dst, &src)
            } else {
                let off = ctx.cur_func().ss() + (id - 8) * 4;
                gen.lw(dst, "sp", off)
            }
        }
        None => {
            if let ValueKind::Integer(imm) = ctx.value_kind(val) {
                gen.li(dst, imm.value())
            } else {
                gen.lw(dst, "sp", ctx.cur_func().get_offset(&val))
            }
        }
    }
}

pub fn read_addr_to(gen: &mut AsmGenerator, ctx: &Context, dst: &str, val: Value) -> Result<()> {
    if ctx.is_global(val) {
        let id = ctx.get_global_var(&val);
        let name = format!("var{}", id);
        gen.la(dst, &name)
    } else {
        gen.addi(dst, "sp", ctx.cur_func().get_offset(&val))
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
