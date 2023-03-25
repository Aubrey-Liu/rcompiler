pub(crate) mod ir;
pub(crate) mod record;

mod gen;

mod utils;

pub(crate) use ir::*;
pub(crate) use record::*;

use anyhow::*;
use koopa::ir::BinaryOp as IR_BinaryOp;
use koopa::ir::{BasicBlock, Function, FunctionData, Program, Type, Value, ValueKind};
use utils::*;
