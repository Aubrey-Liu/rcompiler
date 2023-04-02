mod context;
mod gen;
mod utils;

use std::fs::File;

use anyhow::Result;
use koopa::ir::{values::*, *};

use context::*;
use gen::*;
use utils::*;

pub fn generate_code(input: &str, output: &str) -> Result<()> {
    use crate::irgen::generate_mem_ir;

    let program = generate_mem_ir(input)?;
    let mut ctx = Context::new_with_program(&program);
    let mut generator = AsmGenerator::from_path(output, "t0");
    program.generate(&mut generator, &mut ctx)?;

    Ok(())
}
