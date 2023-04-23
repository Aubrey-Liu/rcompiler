mod context;
mod gen;
mod program;
mod write;

use std::fs::File;

use crate::irgen::generate_mem_ir;
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
    let mut generator = AsmWriter::from_path(output);
    generator.write_program(&asm_program)?;

    Ok(())
}
