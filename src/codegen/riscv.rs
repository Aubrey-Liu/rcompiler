use anyhow::Result;
use std::fs::remove_file;

use super::*;
use crate::irgen::*;

pub fn generate_code(ipath: &str, opath: &str) -> Result<()> {
    let tmp_path = "tmp.koopa";
    generate_ir(ipath, tmp_path)?;
    let driver = koopa::front::Driver::from_path(tmp_path)?;
    let program = driver.generate_program();
    remove_file(tmp_path)?;

    let program = program.unwrap();
    let mut program_stat = ProgramStat::new_with_program(&program);
    let mut generator = AsmGenerator::from_path(opath)?;
    program.generate(&mut generator, &mut program_stat)?;

    Ok(())
}
