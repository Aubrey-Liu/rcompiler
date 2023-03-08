use anyhow::{anyhow, Result};
use generate::riscv::GenerateAsm;
use lalrpop_util::lalrpop_mod;
use std::env::args;
use std::fs::File;

pub mod ast;
pub mod generate;

lalrpop_mod!(sysy);

fn main() -> Result<()> {
    // 解析命令行参数
    let mut args = args();
    args.next();
    let mode = args.next().unwrap();
    let ipath = args.next().unwrap();
    args.next();
    let opath = args.next().unwrap();

    match mode.as_str() {
        "-koopa" => {
            generate::ir::into_text_ir(&ipath, &opath)?;
        }
        "-riscv" => {
            let program = generate::ir::into_mem_ir(&ipath)?;
            let mut f = File::create(&opath)?;
            program.generate(&mut f)?;
        }
        _ => return Err(anyhow!(format!("invalid mode: {}", mode.as_str()))),
    };

    Ok(())
}
