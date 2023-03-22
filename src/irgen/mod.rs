pub mod generate;
pub mod ir;
pub mod symt;
mod utils;

pub use ir::{generate_ir, generate_mem_ir};
pub use symt::*;

use anyhow::*;
use generate::*;
use koopa::ir::BinaryOp as IR_BinaryOp;
use koopa::ir::{BasicBlock, Function, FunctionData, Program, Type, Value, ValueKind};
use utils::*;
