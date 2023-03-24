pub(crate) mod ir;

pub(self) mod gen;
pub(self) mod stat;

mod control;
mod utils;

pub(crate) use ir::generate_ir;
pub(crate) use stat::*;

use anyhow::*;
use control::*;
use koopa::ir::BinaryOp as IR_BinaryOp;
use koopa::ir::{BasicBlock, Function, FunctionData, Program, Type, Value, ValueKind};
use utils::*;
