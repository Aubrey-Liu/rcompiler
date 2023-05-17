use std::env::args;

use anyhow::{bail, Result};
use lalrpop_util::lalrpop_mod;

use codegen::generate_code;
use irgen::generate_ir;

mod ast;
mod codegen;
mod irgen;
mod opt;
mod sema;

lalrpop_mod!(sysy);

fn main() -> Result<()> {
    let mut args = args();
    let mode = args.nth(1).unwrap();
    let input = args.next().unwrap();
    let output = args.nth(1).unwrap();

    match mode.as_str() {
        "-koopa" => generate_ir(&input, &output, true)?,
        "-riscv" => generate_code(&input, &output, false)?,
        "-perf" => generate_code(&input, &output, true)?,
        _ => bail!("invalid mode: {}", mode.as_str()),
    };

    Ok(())
}
