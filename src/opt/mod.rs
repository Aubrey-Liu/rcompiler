mod common_expr;
mod empty_bb;
pub mod pass;
mod sccp;
mod ssa;
mod trivial_arg;
mod unreachable;
mod utils;

use common_expr::*;
use empty_bb::*;
use koopa::ir::Program;
use pass::*;
use sccp::*;
use ssa::SsaBuilder;
use trivial_arg::*;
use unreachable::RemoveUnreachable;
use utils::*;

pub fn optimize(p: &mut Program) {
    let mut pass_runner = PassRunner::new();
    pass_runner.register_pass(Pass(Box::new(RemoveUnreachable)));
    pass_runner.register_pass(Pass(Box::new(SsaBuilder::new())));
    pass_runner.register_pass(Pass(Box::new(Sccp::new())));
    pass_runner.register_pass(Pass(Box::new(RemoveUnreachable)));
    pass_runner.register_pass(Pass(Box::new(RemoveEmptyBB)));
    pass_runner.register_pass(Pass(Box::new(RemoveCommonExpression)));
    pass_runner.register_pass(Pass(Box::new(RemoveTrivialArgs)));
    pass_runner.register_pass(Pass(Box::new(RemoveEmptyBB)));
    pass_runner.run_passes(p);
}
