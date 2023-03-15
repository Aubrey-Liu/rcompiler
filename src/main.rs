use std::env::args;
use std::fs::File;

use anyhow::{anyhow, Result};
use lalrpop_util::lalrpop_mod;

use codegen::GenerateAsm;

pub mod ast;
pub mod codegen;
pub mod irgen;

lalrpop_mod!(sysy);

fn main() -> Result<()> {
    let mut args = args();
    args.next();
    let mode = args.next().unwrap();
    let ipath = args.next().unwrap();
    args.next();
    let opath = args.next().unwrap();

    match mode.as_str() {
        "-koopa" => {
            irgen::into_text_ir(&ipath, &opath)?;
        }
        "-riscv" => {
            let program = irgen::into_mem_ir(&ipath)?;
            let mut f = File::create(&opath)?;
            program.generate(&mut f)?;
        }
        _ => return Err(anyhow!(format!("invalid mode: {}", mode.as_str()))),
    };

    Ok(())
}
