use std::env::args;

use anyhow::{anyhow, Result};
use lalrpop_util::lalrpop_mod;

use codegen::generate_code;
use irgen::generate_ir;

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
        "-koopa" => generate_ir(&ipath, &opath)?,
        "-riscv" => generate_code(&ipath, &opath)?,
        _ => return Err(anyhow!("invalid mode: {}", mode.as_str())),
    };

    Ok(())
}
