use crate::ast::*;
use crate::sysy;
use anyhow::Result;
use koopa::back::KoopaGenerator;
use koopa::ir::{builder_traits::BasicBlockBuilder, *};
use std::fs::read_to_string;

pub fn into_mem_ir(ipath: &str) -> Result<Program> {
    let input = read_to_string(ipath)?;
    let ast = sysy::CompUnitParser::new().parse(&input).unwrap();

    Ok(ast.into_program())
}

pub fn into_text_ir(ipath: &str, opath: &str) -> Result<()> {
    let program = into_mem_ir(ipath)?;

    let mut gen = KoopaGenerator::from_path(opath)?;
    gen.generate_on(&program)?;

    Ok(())
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
            Exp::Uxp(uxp) => match uxp {
                UnaryExp::Neg(e) => -parse_exp(e),
                UnaryExp::Not(e) => is_zero(parse_exp(e)),
            },
            Exp::Bxp(bxp) => match bxp {
                BinaryExp::Add(x) => parse_exp(&x.left) + parse_exp(&x.right),
                BinaryExp::Sub(x) => parse_exp(&x.left) - parse_exp(&x.right),
                BinaryExp::Mul(x) => parse_exp(&x.left) * parse_exp(&x.right),
                BinaryExp::Div(x) => parse_exp(&x.left) / parse_exp(&x.right),
                BinaryExp::Mod(x) => parse_exp(&x.left) % parse_exp(&x.right),
                BinaryExp::And(x) => lnd(parse_exp(&x.left), parse_exp(&x.right)),
                BinaryExp::Or(x) => lor(parse_exp(&x.left), parse_exp(&x.right)),
                BinaryExp::Eq(x) => is_zero(parse_exp(&x.left) - parse_exp(&x.right)),
                BinaryExp::Neq(x) => not_zero(parse_exp(&x.left) - parse_exp(&x.right)),
                BinaryExp::Lt(x) => positive(parse_exp(&x.right) - parse_exp(&x.left)),
                BinaryExp::Lte(x) => non_negative(parse_exp(&x.right) - parse_exp(&x.left)),
                BinaryExp::Gt(x) => positive(parse_exp(&x.left) - parse_exp(&x.right)),
                BinaryExp::Gte(x) => non_negative(parse_exp(&x.left) - parse_exp(&x.right)),
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
