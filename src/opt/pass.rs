use koopa::ir::*;

pub trait ProgramPass {
    fn run_on(&mut self, p: &mut Program);
}

pub trait FunctionPass {
    fn run_on(&mut self, f: &mut FunctionData);
}

pub struct PassRunner {
    passes: Vec<Pass>,
}

pub struct Pass(pub Box<dyn FunctionPass>);

impl PassRunner {
    pub fn run_passes(&mut self, program: &mut Program) {
        for pass in &mut self.passes {
            for data in program.funcs_mut().values_mut() {
                if data.layout().entry_bb().is_some() {
                    pass.0.run_on(data);
                }
            }
        }
    }

    pub fn register_pass(&mut self, pass: Pass) {
        self.passes.push(pass);
    }

    pub fn new() -> Self {
        PassRunner { passes: Vec::new() }
    }
}
