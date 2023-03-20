use super::*;
use crate::irgen::*;

pub fn generate_code(ipath: &str, opath: &str) -> Result<()> {
    let program = generate_mem_ir(ipath)?;
    let mut f = File::create(opath)?;
    program.generate(&mut f)?;

    Ok(())
}
