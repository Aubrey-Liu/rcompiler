mod gen;
pub mod riscv;
mod stat;
mod utils;

pub use riscv::generate_code;

use gen::GenerateAsm;
use koopa::ir::values::*;
use koopa::ir::*;
use stat::*;
use std::fs::File;
use utils::*;
