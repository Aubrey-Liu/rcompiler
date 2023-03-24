pub(crate) mod ir;

pub(self) mod gen;
pub mod record;

mod utils;

pub(crate) use ir::generate_ir;
pub(crate) use record::*;

use anyhow::*;
use koopa::ir::BinaryOp as IR_BinaryOp;
use koopa::ir::{BasicBlock, Function, FunctionData, Program, Type, Value, ValueKind};
use utils::*;
