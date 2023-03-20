pub mod generate;
pub mod ir;
pub mod symt;
mod utils;

pub use ir::{generate_ir, generate_mem_ir};
pub use symt::*;

use generate::GenerateValue;
use utils::*;
