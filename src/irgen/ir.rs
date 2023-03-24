use std::fs::read_to_string;

use anyhow::Result;
use koopa::back::KoopaGenerator;

use super::{gen::GenerateIR, *};
use crate::sysy;

pub fn generate_mem_ir(ipath: &str) -> Result<Program> {
    let input = read_to_string(ipath)?;
    let mut errors = vec![];
    let ast = sysy::CompUnitParser::new()
        .parse(&mut errors, &input)
        .unwrap();
    let mut program = Program::new();
    let mut recorder = ProgramRecorder::new();
    ast.generate_ir(&mut program, &mut recorder)?;

    Ok(program)
}

pub fn generate_ir(ipath: &str, opath: &str) -> Result<()> {
    let program = generate_mem_ir(ipath)?;
    let mut gen = KoopaGenerator::from_path(opath)?;
    gen.generate_on(&program)?;

    Ok(())
}
