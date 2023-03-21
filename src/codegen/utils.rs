use super::*;

pub fn get_bb_name(func: &FunctionData, bb: BasicBlock) -> &str {
    &func.dfg().bb(bb).name().as_ref().unwrap()[1..]
}
