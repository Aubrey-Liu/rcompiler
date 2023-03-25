use std::env::args;

use anyhow::{bail, Result};
use lalrpop_util::lalrpop_mod;

use codegen::generate_code;
use irgen::generate_ir;

mod ast;
mod codegen;
mod irgen;

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
        _ => bail!("invalid mode: {}", mode.as_str()),
    };

    Ok(())
}
