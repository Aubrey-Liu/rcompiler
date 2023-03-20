use koopa::ir::builder_traits::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};
use koopa::ir::{BasicBlock, BinaryOp, Function, FunctionData, Program, Type, Value};

pub fn new_func(program: &mut Program, ident: &str) -> Function {
    let name = "@".to_owned() + ident;
    program.new_func(FunctionData::with_param_names(
        name,
        Vec::new(),
        Type::get_i32(),
    ))
}

pub fn new_bb(func: &mut FunctionData, name: &str) -> BasicBlock {
    func.dfg_mut().new_bb().basic_block(Some(name.into()))
}

pub fn push_bb(func: &mut FunctionData, bb: BasicBlock) {
    func.layout_mut().bbs_mut().extend([bb]);
}

pub fn push_insts(func: &mut FunctionData, bb: BasicBlock, insts: Vec<Value>) {
    func.layout_mut().bb_mut(bb).insts_mut().extend(insts);
}

pub fn integer(func: &mut FunctionData, i: i32) -> Value {
    func.dfg_mut().new_value().integer(i)
}

pub fn ret(func: &mut FunctionData, v: Value) -> Value {
    func.dfg_mut().new_value().ret(Some(v))
}

pub fn alloc(func: &mut FunctionData) -> Value {
    // allocate a pointer for an integer
    func.dfg_mut().new_value().alloc(Type::get_i32())
}

pub fn store(func: &mut FunctionData, val: Value, dst: Value) -> Value {
    func.dfg_mut().new_value().store(val, dst)
}

pub fn load(func: &mut FunctionData, src: Value) -> Value {
    func.dfg_mut().new_value().load(src)
}

pub fn binary(func: &mut FunctionData, op: BinaryOp, lhs: Value, rhs: Value) -> Value {
    func.dfg_mut().new_value().binary(op, lhs, rhs)
}

pub fn neg(func: &mut FunctionData, val: Value) -> Value {
    let zero = zero(func);
    func.dfg_mut().new_value().binary(BinaryOp::Sub, zero, val)
}

pub fn not(func: &mut FunctionData, val: Value) -> Value {
    let zero = zero(func);
    func.dfg_mut().new_value().binary(BinaryOp::Eq, zero, val)
}

fn zero(func: &mut FunctionData) -> Value {
    func.dfg_mut().new_value().integer(0)
}
