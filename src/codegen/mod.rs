pub(crate) mod riscv;

mod context;
mod gen;
mod utils;

mod tests;

pub(crate) use riscv::generate_code;

use context::*;
use gen::GenerateAsm;
use koopa::ir::values::*;
use koopa::ir::*;
use std::fs::File;
use utils::*;
