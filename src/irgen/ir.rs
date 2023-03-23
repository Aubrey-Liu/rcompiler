use std::fs::read_to_string;

use anyhow::Result;
use koopa::back::KoopaGenerator;

use super::*;
use crate::sysy;

pub fn generate_mem_ir(ipath: &str) -> Result<Program> {
    let input = read_to_string(ipath)?;
    let mut global_symt = SymbolTable::new();
    let mut errors = Vec::new();
    let ast = sysy::CompUnitParser::new()
        .parse(&mut errors, &input)
        .unwrap();

    dbg!(&ast);

    ast.generate(&mut global_symt)
}

pub fn generate_ir(ipath: &str, opath: &str) -> Result<()> {
    let program = generate_mem_ir(ipath)?;
    let mut gen = KoopaGenerator::from_path(opath)?;
    gen.generate_on(&program)?;

    Ok(())
}
