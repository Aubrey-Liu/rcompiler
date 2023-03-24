use anyhow::Result;

use super::*;
use crate::irgen::generate_mem_ir;

pub fn generate_code(ipath: &str, opath: &str) -> Result<()> {
    let program = generate_mem_ir(ipath)?;
    let mut ctx = Context::new_with_program(&program);
    let mut generator = AsmGenerator::from_path(opath, "t0")?;
    program.generate(&mut generator, &mut ctx)?;

    Ok(())
}
