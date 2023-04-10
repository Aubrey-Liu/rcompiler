pub mod live;
pub mod pass;
pub mod ssa;
pub mod unreachable;

use koopa::ir::Program;
use pass::*;
use ssa::SsaBuilder;
use unreachable::RemoveUnreachable;

pub fn optimize(p: &mut Program) {
    let mut pass_runner = PassRunner::new();
    pass_runner.register_pass(Pass::FunctionPass(Box::new(RemoveUnreachable)));
    pass_runner.register_pass(Pass::FunctionPass(Box::new(SsaBuilder::new())));
    pass_runner.run_passes(p);
}
