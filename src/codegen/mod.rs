mod generate;
pub mod riscv;

pub use riscv::generate_code;

use anyhow::Result;
use std::fs::File;

use generate::GenerateAsm;
