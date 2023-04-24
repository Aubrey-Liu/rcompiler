mod context;
mod gen;
mod program;
mod write;

use std::fs::File;

use crate::irgen::{generate_mem_ir, generate_mem_ir_opt};
use anyhow::Result;
use koopa::ir::{values::*, *};

use context::*;
use gen::*;
use program::*;
use write::*;

pub fn generate_code(input: &str, output: &str) -> Result<()> {
    let program = generate_mem_ir(input)?;
    let mut ctx = Context::new_with_program(&program);
    let mut asm_program = AsmProgram::new();
    program.generate(&mut ctx, &mut asm_program);
    let mut writer = AsmWriter::from_path(output);
    writer.write_program(&asm_program)?;

    Ok(())
}

pub fn generate_code_opt(input: &str, output: &str) -> Result<()> {
    let program = generate_mem_ir_opt(input)?;
    let mut ctx = Context::new_with_program(&program);
    let mut asm_program = AsmProgram::new();
    program.generate(&mut ctx, &mut asm_program);
    let mut writer = AsmWriter::from_path(output);
    writer.write_program(&asm_program)?;

    Ok(())
}
