#![allow(unused)]
use std::collections::{hash_map, HashMap};

use super::pass::Pass;
use koopa::ir::{
    builder_traits::{LocalInstBuilder, ValueBuilder},
    dfg::DataFlowGraph,
    entities::{BasicBlockData, ValueData},
    layout::BasicBlockNode,
    *,
};

#[derive(Debug, Default)]
pub struct SsaBuilder {
    // mapping from a local variable to its recent definition
    defs: HashMap<Value, HashMap<BasicBlock, Value>>,
    replace_with: Vec<(BasicBlock, Value, Value)>,
    preds: HashMap<BasicBlock, Vec<BasicBlock>>,
}

impl<'p> Pass<'p> for SsaBuilder {
    fn run_on(&mut self, p: &'p mut Program) {
        p.funcs_mut()
            .values_mut()
            .filter(|f| f.layout().entry_bb().is_some())
            .for_each(|f| self.visit_func(f));
    }
}

impl SsaBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    fn visit_func(&mut self, func: &mut FunctionData) {
        // go from back to the front
        for (&bb, node) in func.layout().bbs().iter() {
            for &val in node.insts().keys() {
                match value_kind(func, val) {
                    ValueKind::Alloc(_) => {
                        self.defs.insert(val, HashMap::new());
                    }
                    ValueKind::Store(s) => {
                        if !self.defs.contains_key(&s.dest()) {
                            continue;
                        }
                        // if the destination of store is part of an array, skip it
                        if !matches!(value_kind(func, s.dest()), ValueKind::Alloc(_)) {
                            continue;
                        }
                        self.defs.get_mut(&s.dest()).unwrap().insert(bb, s.value());
                    }
                    ValueKind::Load(l) => {
                        if !self.defs.contains_key(&l.src()) {
                            continue;
                        }
                        if !matches!(value_kind(func, l.src()), ValueKind::Alloc(_)) {
                            continue;
                        }
                        let mut not_cross_bb = true;
                        for &user in func.dfg().value(val).used_by() {
                            if func.layout().parent_bb(user).unwrap() != bb {
                                not_cross_bb = false;
                                break;
                            }
                        }
                        let def = self.defs.get(&l.src()).unwrap().get(&bb);
                        if def.is_some() && not_cross_bb {
                            self.replace_with.push((bb, val, *def.unwrap()));
                        }
                    }
                    _ => {}
                }
            }
        }

        for &(bb, origin, replace_by) in self.replace_with.iter() {
            for user in func.dfg().value(origin).used_by().clone() {
                let mut user_data = func.dfg().value(user).clone();
                match user_data.kind_mut() {
                    ValueKind::Branch(br) => *br.cond_mut() = replace_by,
                    ValueKind::Return(ret) => *ret.value_mut() = Some(replace_by),
                    ValueKind::Store(s) => *s.value_mut() = replace_by,
                    ValueKind::GetElemPtr(g) => *g.index_mut() = replace_by,
                    ValueKind::GetPtr(g) => *g.index_mut() = replace_by,
                    ValueKind::Binary(b) => {
                        if origin == b.lhs() {
                            *b.lhs_mut() = replace_by;
                        } else {
                            *b.rhs_mut() = replace_by;
                        }
                    }
                    ValueKind::Call(call) => {
                        for arg in call.args_mut() {
                            if *arg == origin {
                                *arg = replace_by;
                            }
                        }
                    }
                    _ => {}
                }
                func.dfg_mut().replace_value_with(user).raw(user_data);
            }
            func.dfg_mut().remove_value(origin);
            func.layout_mut().bb_mut(bb).insts_mut().remove(&origin);
        }

        self.defs.clear();
        self.replace_with.clear();
    }
}

fn value_kind(func: &FunctionData, val: Value) -> &ValueKind {
    func.dfg().value(val).kind()
}
