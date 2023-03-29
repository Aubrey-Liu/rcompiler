pub(crate) mod record;

mod gen;
mod utils;

pub(crate) use record::*;

use std::fs::read_to_string;

use anyhow::*;
use koopa::back::KoopaGenerator;
use koopa::ir::BinaryOp as IrBinaryOp;
use koopa::ir::Type as IrType;
use koopa::ir::{BasicBlock, Function, FunctionData, Program, Value};

use crate::sysy;
use gen::*;
use utils::*;

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
