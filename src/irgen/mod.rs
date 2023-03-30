pub(crate) mod record;

mod gen;
mod utils;

pub(crate) use record::*;

use std::cell::RefCell;
use std::fs::read_to_string;
use std::rc::Rc;

use anyhow::*;
use koopa::back::KoopaGenerator;
use koopa::ir::BinaryOp as IrBinaryOp;
use koopa::ir::Type as IrType;
use koopa::ir::{BasicBlock, Function, FunctionData, Program, Value};

use crate::sysy;
use gen::*;
use utils::*;

pub fn generate_mem_ir(ipath: &str) -> Result<Program> {
    use crate::sema::analyze::Analyzer;
    use crate::sema::name::*;
    use crate::sema::symbol::SymbolTable;

    let input = read_to_string(ipath)?;
    let mut errors = vec![];
    let mut ast = sysy::CompUnitParser::new()
        .parse(&mut errors, &input)
        .unwrap();

    let mut manager = NameManager::new();
    ast.rename(&mut manager);

    dbg!(&ast);
    let symbols = Rc::new(RefCell::new(SymbolTable::new()));
    ast.analyze(&mut symbols.borrow_mut())?;

    let mut program = Program::new();
    let mut recorder = ProgramRecorder::new(&symbols);

    ast.generate_ir(&mut program, &mut recorder)?;

    Ok(program)
}

pub fn generate_ir(ipath: &str, opath: &str) -> Result<()> {
    let program = generate_mem_ir(ipath)?;
    let mut gen = KoopaGenerator::from_path(opath)?;
    gen.generate_on(&program)?;

    Ok(())
}
