pub(crate) mod riscv;

mod gen;
mod stat;
mod tests;
mod utils;

pub(crate) use riscv::generate_code;

use gen::GenerateAsm;
use koopa::ir::values::*;
use koopa::ir::*;
use stat::*;
use std::fs::File;
use utils::*;
