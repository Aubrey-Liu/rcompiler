use koopa::ir::*;

pub trait Pass<'p> {
    fn run_on(&mut self, p: &'p mut Program);
}
