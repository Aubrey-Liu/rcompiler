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
            'inst: for &val in node.insts().keys() {
                for &user in func.dfg().value(val).used_by() {
                    if func.layout().parent_bb(user).unwrap() != bb {
                        continue 'inst;
                    }
                }
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
                        let def = self.defs.get(&l.src()).unwrap().get(&bb);
                        if def.is_some() {
                            self.replace_with.push((bb, val, *def.unwrap()));
                        }
                    }
                    _ => {}
                }
            }
        }

        for &(bb, origin, replace_by) in self.replace_with.iter() {
            for user in func.dfg().value(origin).used_by().clone() {
                match value_kind(func, user).clone() {
                    ValueKind::Binary(b) => {
                        if func.dfg().value_eq(origin, b.lhs()) {
                            func.dfg_mut().replace_value_with(user).binary(
                                b.op(),
                                replace_by,
                                b.rhs(),
                            );
                        } else {
                            func.dfg_mut().replace_value_with(user).binary(
                                b.op(),
                                b.lhs(),
                                replace_by,
                            );
                        }
                    }
                    ValueKind::Branch(br) => {
                        func.dfg_mut().replace_value_with(user).branch(
                            replace_by,
                            br.true_bb(),
                            br.false_bb(),
                        );
                    }
                    ValueKind::Call(call) => {
                        let args: Vec<_> = call
                            .args()
                            .iter()
                            .map(|arg| {
                                if func.dfg().value_eq(*arg, origin) {
                                    replace_by
                                } else {
                                    *arg
                                }
                            })
                            .collect();
                        func.dfg_mut()
                            .replace_value_with(user)
                            .call(call.callee(), args);
                    }
                    ValueKind::Return(ret) => {
                        func.dfg_mut()
                            .replace_value_with(user)
                            .ret(Some(replace_by));
                    }
                    ValueKind::Store(s) => {
                        func.dfg_mut()
                            .replace_value_with(user)
                            .store(replace_by, s.dest());
                    }
                    ValueKind::GetElemPtr(g) => {
                        func.dfg_mut()
                            .replace_value_with(user)
                            .get_elem_ptr(g.src(), replace_by);
                    }
                    ValueKind::GetPtr(g) => {
                        func.dfg_mut()
                            .replace_value_with(user)
                            .get_ptr(g.src(), replace_by);
                    }
                    _ => {}
                }
            }
            func.layout_mut().bb_mut(bb).insts_mut().remove(&origin);
        }

        self.defs.clear();
        self.replace_with.clear();
    }
}

fn value_kind(func: &FunctionData, val: Value) -> &ValueKind {
    func.dfg().value(val).kind()
}
