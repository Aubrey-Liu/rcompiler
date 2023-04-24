pub(crate) mod record;

mod gen;
mod utils;

pub(crate) use record::*;

use std::fs::read_to_string;
use std::fs::File;
use std::io::BufWriter;

use anyhow::*;
use koopa::back::KoopaGenerator;
use koopa::ir::BinaryOp as IrBinaryOp;
use koopa::ir::Type as IrType;
use koopa::ir::{BasicBlock, Function, FunctionData, Program, Value};

use crate::opt::optimize;
use crate::sema::*;
use crate::sysy;
use gen::*;
use utils::*;

pub fn generate_mem_ir_opt(input: &str) -> Result<Program> {
    let mut p = generate_mem_ir(input)?;
    optimize(&mut p);

    Ok(p)
}

pub fn generate_mem_ir(input: &str) -> Result<Program> {
    let input = read_to_string(input)?;
    let mut errors = vec![];
    let mut ast = sysy::CompUnitParser::new()
        .parse(&mut errors, &input)
        .unwrap();

    let mut name_manager = NameManager::new();
    ast.accept(&mut name_manager);

    let mut evaluator = Evaluator::new();
    ast.accept(&mut evaluator);

    let mut symbols = SymbolTable::new();
    ast.accept(&mut symbols);

    let mut program = Program::new();
    let mut recorder = ProgramRecorder::new(&mut program, &symbols);
    ast.generate_ir(&mut recorder)?;

    Ok(program)
}

pub fn generate_ir(input: &str, output: &str) -> Result<()> {
    let program = generate_mem_ir_opt(input)?;
    let output = File::create(output).unwrap();
    let mut gen = KoopaGenerator::new(BufWriter::new(output));
    gen.generate_on(&program)?;

    Ok(())
}
