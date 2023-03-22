use std::fs::remove_file;

use super::*;
use crate::irgen::*;

pub fn generate_code(ipath: &str, opath: &str) -> Result<()> {
    let mut f = File::create(opath)?;
    let tmp_path = "tmp.koopa";
    generate_ir(ipath, tmp_path)?;
    let driver = koopa::front::Driver::from_path(tmp_path)?;
    let program = driver.generate_program();
    remove_file(tmp_path)?;
    program.unwrap().generate(&mut f)?;

    Ok(())
}
