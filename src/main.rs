use generate::riscv::GenerateAsm;
use lalrpop_util::lalrpop_mod;
use std::env::args;
use std::fs::File;
use std::io::Result;

pub mod ast;
pub mod generate;

// 引用 lalrpop 生成的解析器
// 因为我们刚刚创建了 sysy.lalrpop, 所以模块名是 sysy
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
            generate::ir::into_text_ir(&ipath, &opath);
        }
        "-riscv" => {
            let program = generate::ir::into_mem_ir(&ipath);
            let mut f = File::create(&opath).unwrap();
            program.generate(&mut f);
        }
        _ => panic!("Unexpected mode."),
    };

    Ok(())
}
