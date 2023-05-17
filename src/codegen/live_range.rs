use std::{
    cmp::min,
    collections::{HashMap, HashSet},
};

use koopa::ir::{BasicBlock, Function, FunctionData, Value, ValueKind};

use super::*;

type ID = u32;

#[derive(Debug, Clone, Copy)]
pub struct Range {
    pub begin: ID,
    pub end: ID,
}

#[derive(Debug, Clone)]
pub struct LiveRange {
    number_mapping: HashMap<Value, ID>,
    idx_mapping: HashMap<Value, usize>,
    pub ranges: HashMap<Function, Vec<(Range, Value)>>,
    pub function_calls: HashMap<Function, Vec<ID>>,
}

impl LiveRange {
    pub fn analyze(&mut self, p: &Program) {
        self.set_numbers(p);
        self.find_live_ranges(p);
    }

    fn set_numbers(&mut self, p: &Program) {
        let mut id = ID::default();
        for (fid, f) in p.funcs() {
            if f.layout().entry_bb().is_none() {
                continue;
            }
            self.function_calls.insert(*fid, Vec::new());
            for node in f.layout().bbs().nodes() {
                for val in node.insts().keys() {
                    self.number_mapping.insert(*val, id);
                    let kind = f.dfg().value(*val).kind();
                    if matches!(kind, ValueKind::Call(_)) {
                        self.function_calls.get_mut(&fid).unwrap().push(id);
                    } else if let ValueKind::Store(store) = kind {
                        if matches!(f.dfg().value(store.value()).kind(), ValueKind::ZeroInit(_)) {
                            self.function_calls.get_mut(&fid).unwrap().push(id);
                        }
                    }
                    id += 1;
                }
            }
        }
    }

    fn find_live_ranges(&mut self, p: &Program) {
        for (&fid, f) in p.funcs() {
            if f.layout().entry_bb().is_none() {
                continue;
            }
            let func_entry = first_inst_of_bb(f, f.layout().entry_bb().unwrap());
            f.params()
                .iter()
                .for_each(|&p| self.find_live_range_of(fid, f, p, func_entry));

            let mut visited = HashSet::new();
            for (&bb, node) in f.layout().bbs() {
                visited.insert(bb);
                f.dfg()
                    .bb(bb)
                    .params()
                    .iter()
                    .for_each(|&p| self.find_live_range_of_param(fid, f, bb, p));
                for &val in node.insts().keys() {
                    let ty = f.dfg().value(val).ty();
                    if ty.is_unit() {
                        continue;
                    }
                    self.find_live_range_of(fid, f, val, val);
                }
                let bb_exit = *node.insts().back_key().unwrap();
                if let ValueKind::Jump(j) = f.dfg().value(bb_exit).kind() {
                    if !visited.contains(&j.target()) {
                        continue;
                    }
                    let loop_begin = first_inst_of_bb(f, j.target());
                    let loop_begin = *self.number_mapping.get(&loop_begin).unwrap();
                    let loop_end = *self.number_mapping.get(&bb_exit).unwrap();
                    self.update_use_in_loop(fid, loop_begin, loop_end);
                }
            }
        }

        self.ranges
            .values_mut()
            .for_each(|v| v.sort_by(|a, b| a.0.begin.cmp(&b.0.begin)));
    }

    fn update_use_in_loop(&mut self, fid: Function, loop_begin: ID, loop_end: ID) {
        for (r, _) in self.ranges.get_mut(&fid).unwrap() {
            if r.begin < loop_begin && r.end >= loop_begin && r.end < loop_end {
                r.end = loop_end;
            }
        }
    }

    fn find_live_range_of_param(
        &mut self,
        fid: Function,
        f: &FunctionData,
        bb: BasicBlock,
        val: Value,
    ) {
        let mut def_id = u32::MAX;
        for &user in f.dfg().bb(bb).used_by() {
            let pred = f.layout().parent_bb(user).unwrap();
            let exit = last_inst_of_bb(f, pred);
            let exit_id = *self.number_mapping.get(&exit).unwrap();
            def_id = min(def_id, exit_id);
        }
        let new_idx = self.ranges.entry(fid).or_default().len();
        self.idx_mapping.insert(val, new_idx);
        self.ranges
            .entry(fid)
            .and_modify(|v| v.push((Range::new(def_id, def_id), val)));
        for user in f.dfg().value(val).used_by() {
            self.update_range(fid, val, *self.number_mapping.get(user).unwrap());
        }
    }

    fn find_live_range_of(&mut self, fid: Function, f: &FunctionData, val: Value, def: Value) {
        let def_id = *self.number_mapping.get(&def).unwrap();
        let new_idx = self.ranges.entry(fid).or_default().len();
        self.idx_mapping.insert(val, new_idx);
        self.ranges
            .entry(fid)
            .and_modify(|v| v.push((Range::new(def_id, def_id), val)));
        for user in f.dfg().value(val).used_by() {
            self.update_range(fid, val, *self.number_mapping.get(user).unwrap());
        }
    }

    fn update_range(&mut self, fid: Function, val: Value, used_in: ID) {
        let idx = *self.idx_mapping.get(&val).unwrap();
        let mut r = &mut self.ranges.get_mut(&fid).unwrap().get_mut(idx).unwrap().0;
        if r.begin > used_in {
            r.begin = used_in;
        } else if r.end < used_in {
            r.end = used_in;
        }
    }

    pub fn new() -> Self {
        Self {
            number_mapping: HashMap::new(),
            idx_mapping: HashMap::new(),
            ranges: HashMap::new(),
            function_calls: HashMap::new(),
        }
    }
}

impl Range {
    pub fn new(begin: ID, end: ID) -> Self {
        Self { begin, end }
    }
}

fn last_inst_of_bb(f: &FunctionData, bb: BasicBlock) -> Value {
    f.layout()
        .bbs()
        .node(&bb)
        .map(|n| *n.insts().back_key().unwrap())
        .unwrap()
}

fn first_inst_of_bb(f: &FunctionData, bb: BasicBlock) -> Value {
    f.layout()
        .bbs()
        .node(&bb)
        .map(|n| *n.insts().front_key().unwrap())
        .unwrap()
}
