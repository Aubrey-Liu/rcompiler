use std::collections::HashMap;

use koopa::ir::{BasicBlock, FunctionData, ValueKind};

use super::*;

pub struct RemoveCommonExpression;

impl FunctionPass for RemoveCommonExpression {
    fn run_on(&mut self, f: &mut FunctionData) {
        while self.work(f) {}
    }
}

impl RemoveCommonExpression {
    fn work(&self, f: &mut FunctionData) -> bool {
        let mut removed_values: HashMap<BasicBlock, Vec<_>> = HashMap::new();
        for (&bb, node) in f.layout().bbs() {
            let mut visited_values = Vec::new();
            for &val in node.insts().keys() {
                let mut found_common_expr = false;
                let mut same = val;
                for &expr in &visited_values {
                    same = expr;
                    match (value_kind(f, expr), value_kind(f, val)) {
                        (ValueKind::GetElemPtr(lhs), ValueKind::GetElemPtr(rhs)) => {
                            if value_eq(f, lhs.index(), rhs.index()) && lhs.src() == rhs.src() {
                                found_common_expr = true;
                                break;
                            }
                        }
                        (ValueKind::GetPtr(lhs), ValueKind::GetPtr(rhs)) => {
                            if value_eq(f, lhs.index(), rhs.index()) && lhs.src() == rhs.src() {
                                found_common_expr = true;
                                break;
                            }
                        }
                        (ValueKind::Binary(lhs), ValueKind::Binary(rhs)) => {
                            if lhs.op() == rhs.op()
                                && lhs.lhs() == rhs.lhs()
                                && value_eq(f, lhs.rhs(), rhs.rhs())
                            {
                                found_common_expr = true;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if found_common_expr {
                    removed_values.entry(bb).or_default().push((val, same));
                } else {
                    visited_values.push(val);
                }
            }
        }
        for (&bb, pairs) in &removed_values {
            for &(val, replace_by) in pairs {
                replace_variable(f, val, replace_by);
                f.dfg_mut().remove_value(val);
                f.layout_mut().bb_mut(bb).insts_mut().remove(&val);
            }
        }

        return !removed_values.is_empty();
    }
}
