mod generate;
pub mod riscv;
mod utils;

pub use riscv::generate_code;

use anyhow::Result;
use generate::GenerateAsm;
use koopa::ir::values::*;
use koopa::ir::*;
use std::fs::File;
use utils::*;
