mod alloca;
mod context;
mod gen;
mod program;
mod write;

use std::fs::File;

use crate::{irgen::generate_mem_ir, opt::live_range::LiveRange};
use anyhow::Result;
use koopa::ir::{values::*, *};

use context::*;
use gen::*;
use program::*;
use write::*;

use self::alloca::RegAllocator;

pub fn generate_code(input: &str, output: &str, opt: bool) -> Result<()> {
    let program = generate_mem_ir(input, opt)?;

    let mut ctx = Context::new(&program);
    let mut asm_program = AsmProgram::new();
    program.generate(&mut ctx, &mut asm_program);

    let mut writer = AsmWriter::from_path(output);
    writer.write_program(&asm_program)?;

    Ok(())
}
