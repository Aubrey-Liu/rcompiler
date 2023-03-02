use crate::ast::*;
use crate::sysy;
use koopa::back::KoopaGenerator;
use koopa::ir::{builder_traits::BasicBlockBuilder, *};
use std::fs::read_to_string;

pub fn into_mem_ir(ipath: &str) -> Program {
    let input = read_to_string(ipath).unwrap();
    let ast = sysy::CompUnitParser::new().parse(&input).unwrap();
    ast.into_program()
}
pub fn into_text_ir(ipath: &str, opath: &str) {
    let program = into_mem_ir(ipath);

    let mut gen = KoopaGenerator::from_path(opath).unwrap();
    gen.generate_on(&program).unwrap();
}

impl CompUnit {
    pub fn into_program(&self) -> Program {
        let mut program = Program::new();
        let fib = self.func_def.new_func(&mut program);
        let fib_data = program.func_mut(fib);
        // Create the entry block
        let entry = self.func_def.block.new_bb(fib_data, "%entry");
        self.func_def.push_bb(fib_data, entry);
        self.func_def.block.parse_bb(fib_data, entry);

        program
    }
}

impl FuncDef {
    pub fn new_func(&self, program: &mut Program) -> Function {
        let name = "@".to_owned() + &self.ident;
        program.new_func(FunctionData::with_param_names(
            name,
            Vec::new(),
            Type::get_i32(),
        ))
    }

    pub fn push_bb(&self, fib_data: &mut FunctionData, bb: BasicBlock) {
        fib_data.layout_mut().bbs_mut().extend([bb]);
    }
}

impl Block {
    pub fn new_bb(&self, fib_data: &mut FunctionData, name: &str) -> BasicBlock {
        fib_data.dfg_mut().new_bb().basic_block(Some(name.into()))
    }

    pub fn parse_bb(&self, fib_data: &mut FunctionData, bb: BasicBlock) {
        let mut insts = Vec::new();
        let mut values = self.values.iter().peekable();

        while let Some(value) = values.next() {
            if let AstValue::Return(e) = value {
                let ret_value = e.parse(fib_data);
                insts.push(insts::ret(fib_data, ret_value));
            } else {
                // todo: At now, we can only understand the 'return' statement.
                break;
            }
        }
        fib_data.layout_mut().bb_mut(bb).insts_mut().extend(insts);
    }
}

impl Exp {
    pub fn parse(&self, fib_data: &mut FunctionData) -> Value {
        insts::integer(fib_data, exp::parse_exp(self))
    }
}

pub(super) mod exp {
    use crate::ast::*;

    pub fn parse_exp(exp: &Exp) -> i32 {
        match exp {
            Exp::Integer(i) => *i,
            Exp::Uxp(op) => match op {
                UnaryExp::Neg(e) => -parse_exp(e),
                UnaryExp::Not(e) => is_zero(parse_exp(e)),
            },
            Exp::Bxp(op) => match op {
                BinaryExp::Add(p) => parse_exp(&p.left) + parse_exp(&p.right),
                BinaryExp::Sub(p) => parse_exp(&p.left) - parse_exp(&p.right),
                BinaryExp::Mul(p) => parse_exp(&p.left) * parse_exp(&p.right),
                BinaryExp::Div(p) => parse_exp(&p.left) / parse_exp(&p.right),
                BinaryExp::Mod(p) => parse_exp(&p.left) % parse_exp(&p.right),
                BinaryExp::And(p) => lnd(parse_exp(&p.left), parse_exp(&p.right)),
                BinaryExp::Or(p) => lor(parse_exp(&p.left), parse_exp(&p.right)),
                BinaryExp::Eq(p) => is_zero(parse_exp(&p.left) - parse_exp(&p.right)),
                BinaryExp::Neq(p) => not_zero(parse_exp(&p.left) - parse_exp(&p.right)),
                BinaryExp::Lt(p) => positive(parse_exp(&p.right) - parse_exp(&p.left)),
                BinaryExp::Lte(p) => non_negative(parse_exp(&p.right) - parse_exp(&p.left)),
                BinaryExp::Gt(p) => positive(parse_exp(&p.left) - parse_exp(&p.right)),
                BinaryExp::Gte(p) => non_negative(parse_exp(&p.left) - parse_exp(&p.right)),
            },
        }
    }

    // logical and
    fn lnd(x: i32, y: i32) -> i32 {
        (x != 0 && y != 0) as i32
    }

    // logical or
    fn lor(x: i32, y: i32) -> i32 {
        (x != 0 || y != 0) as i32
    }

    fn is_zero(i: i32) -> i32 {
        (i == 0) as i32
    }

    fn not_zero(i: i32) -> i32 {
        (i != 0) as i32
    }

    fn positive(x: i32) -> i32 {
        x.is_positive() as i32
    }

    fn non_negative(x: i32) -> i32 {
        !x.is_negative() as i32
    }
}

pub(super) mod insts {
    use koopa::ir::{
        builder_traits::{LocalInstBuilder, ValueBuilder},
        FunctionData, Value,
    };

    pub fn integer(fib_data: &mut FunctionData, i: i32) -> Value {
        fib_data.dfg_mut().new_value().integer(i)
    }

    pub fn ret(fib_data: &mut FunctionData, v: Value) -> Value {
        fib_data.dfg_mut().new_value().ret(Some(v))
    }
}
