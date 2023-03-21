use koopa::ir::builder_traits::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};
use koopa::ir::{BasicBlock, BinaryOp, Function, FunctionData, Program, Type, Value, ValueKind};

pub fn generate_var_name(name: &str) -> String {
    "@".to_owned() + name
}

pub fn new_func(program: &mut Program, ident: &str) -> Function {
    let name = "@".to_owned() + ident;
    program.new_func(FunctionData::with_param_names(
        name,
        Vec::new(),
        Type::get_i32(),
    ))
}

pub fn new_bb(func: &mut FunctionData, name: &str) -> BasicBlock {
    let bb = func.dfg_mut().new_bb().basic_block(Some(name.into()));
    push_bb(func, bb);

    bb
}

pub fn new_branch(func: &mut FunctionData) -> (BasicBlock, BasicBlock, BasicBlock) {
    let true_bb = new_bb(func, "%then");
    let false_bb = new_bb(func, "%else");
    let end_bb = new_bb(func, "%end");

    push_bb(func, true_bb);
    push_bb(func, false_bb);
    push_bb(func, end_bb);

    (true_bb, false_bb, end_bb)
}

pub fn push_bb(func: &mut FunctionData, bb: BasicBlock) {
    func.layout_mut().bbs_mut().extend([bb]);
}

pub fn push_one_inst(func: &mut FunctionData, bb: BasicBlock, inst: Value) {
    func.layout_mut().bb_mut(bb).insts_mut().extend([inst]);
}

pub fn push_insts(func: &mut FunctionData, bb: BasicBlock, insts: &Vec<Value>) {
    func.layout_mut()
        .bb_mut(bb)
        .insts_mut()
        .extend(insts.clone());
}

pub fn alloc(func: &mut FunctionData) -> Value {
    // allocate a pointer for an integer
    func.dfg_mut().new_value().alloc(Type::get_i32())
}

pub fn binary(func: &mut FunctionData, op: BinaryOp, lhs: Value, rhs: Value) -> Value {
    func.dfg_mut().new_value().binary(op, lhs, rhs)
}

pub fn branch(
    func: &mut FunctionData,
    cond: Value,
    true_bb: BasicBlock,
    false_bb: BasicBlock,
) -> Value {
    func.dfg_mut().new_value().branch(cond, true_bb, false_bb)
}

pub fn branch_from(
    func: &mut FunctionData,
    cond: Value,
    src: BasicBlock,
    true_bb: BasicBlock,
    false_bb: BasicBlock,
) {
    let br = branch(func, cond, true_bb, false_bb);
    push_one_inst(func, src, br);
}

pub fn integer(func: &mut FunctionData, i: i32) -> Value {
    func.dfg_mut().new_value().integer(i)
}

pub fn load(func: &mut FunctionData, src: Value) -> Value {
    func.dfg_mut().new_value().load(src)
}

pub fn jump(func: &mut FunctionData, target: BasicBlock) -> Value {
    func.dfg_mut().new_value().jump(target)
}

pub fn jump_to(func: &mut FunctionData, from: BasicBlock, to: BasicBlock) {
    let jump = jump(func, to);
    push_one_inst(func, from, jump);
}

pub fn check_and_jump(func: &mut FunctionData, src: BasicBlock, target: BasicBlock) {
    if func.layout_mut().bb_mut(src).insts().is_empty() {
        jump_to(func, src, target);
        return;
    }
    let last_inst = *func.layout_mut().bb_mut(src).insts().back_key().unwrap();
    let last_inst = func.dfg().value(last_inst).kind();
    match last_inst {
        ValueKind::Branch(_) | ValueKind::Return(_) | ValueKind::Jump(_) => {}
        _ => jump_to(func, src, target),
    }
}

pub fn ret(func: &mut FunctionData, v: Value) -> Value {
    func.dfg_mut().new_value().ret(Some(v))
}

pub fn store(func: &mut FunctionData, val: Value, dst: Value) -> Value {
    func.dfg_mut().new_value().store(val, dst)
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
