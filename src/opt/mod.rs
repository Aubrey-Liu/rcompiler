mod empty_bb;
mod live;
pub mod pass;
mod sccp;
mod ssa;
mod unreachable;
mod utils;

use empty_bb::*;
use koopa::ir::Program;
use pass::*;
use ssa::SsaBuilder;
use unreachable::RemoveUnreachable;
use utils::*;

pub fn optimize(p: &mut Program) {
    let mut pass_runner = PassRunner::new();
    // remove unreachable bbs before constructing ssa form can yield less bb arguments
    pass_runner.register_pass(Pass::FunctionPass(Box::new(RemoveUnreachable)));
    pass_runner.register_pass(Pass::FunctionPass(Box::new(SsaBuilder::new())));
    pass_runner.register_pass(Pass::FunctionPass(Box::new(RemoveEmptyBB)));
    pass_runner.run_passes(p);
}
