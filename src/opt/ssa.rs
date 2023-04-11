use std::collections::{HashMap, HashSet};

use super::*;
use koopa::ir::{
    builder_traits::{BasicBlockBuilder, ValueBuilder},
    *,
};
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy)]
pub enum Def {
    Assign(Value),
    Argument(Value),
}

#[derive(Debug, Default)]
pub struct SsaBuilder {
    /// mapping from a basic block to its predecessors
    preds: HashMap<BasicBlock, SmallVec<[BasicBlock; 4]>>,
    /// mapping from a local variable to its recent definition
    defs: HashMap<Value, HashMap<BasicBlock, Def>>,
    /// mapping from a load instruction to the previous definition
    replace_with: HashMap<Value, (BasicBlock, Def)>,
    /// basic block parameters that are waiting to be added into bbs
    bb_params: HashMap<BasicBlock, SmallVec<[Value; 6]>>,
    /// basic blocks that are filled (already been scanned)
    filled_bbs: HashSet<BasicBlock>,
    /// basic blocks that are not sealed (not all of its predecessors are filled),
    /// and variables waiting to find their cloest definitions
    incomplete_bbs: HashMap<BasicBlock, SmallVec<[Value; 6]>>,
}

impl FunctionPass for SsaBuilder {
    fn run_on(&mut self, f: &mut FunctionData) {
        if f.layout().entry_bb().is_some() {
            self.build_ssa(f);
        }
    }
}

impl SsaBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    fn build_ssa(&mut self, func: &mut FunctionData) {
        self.record_preds(func);
        self.walk_bbs(func);
        self.insert_bb_params(func);
        self.replace_load_with_def(func);
        self.remove_local_variables(func);
        self.clear();
    }

    fn walk_bbs(&mut self, func: &mut FunctionData) {
        for (&bb, node) in func.layout().bbs().iter() {
            for &val in node.insts().keys() {
                let val_data = func.dfg().value(val);
                match val_data.kind() {
                    ValueKind::Alloc(_) => {
                        // record local variables
                        if let TypeKind::Pointer(base_ty) = val_data.ty().kind() {
                            // only deal with integer type
                            if base_ty.is_i32() {
                                self.defs.insert(val, HashMap::new());
                            }
                        } else {
                            unreachable!()
                        }
                    }
                    ValueKind::Store(s) => {
                        if !self.defs.contains_key(&s.dest()) {
                            continue;
                        }
                        let mut def = Def::Assign(s.value());
                        if let ValueKind::Load(l) = value_kind(func, s.value()) {
                            if self.defs.contains_key(&l.src()) {
                                def = self.read_variable(func, l.src(), bb);
                            }
                        }
                        self.defs.get_mut(&s.dest()).unwrap().insert(bb, def);
                    }
                    ValueKind::Load(l) => {
                        if !self.defs.contains_key(&l.src()) {
                            continue;
                        }
                        let def = self.read_variable(func, l.src(), bb);
                        self.replace_with.insert(val, (bb, def));
                    }
                    _ => {}
                }
            }
            self.filled_bbs.insert(bb);
            self.try_seal(func);
        }
    }

    fn record_preds(&mut self, func: &mut FunctionData) {
        if !self.preds.is_empty() {
            self.preds.clear();
        }
        for &bb in func.layout().bbs().keys() {
            for &user in func.dfg().bb(bb).used_by() {
                let pred = func.layout().parent_bb(user).unwrap();
                match value_kind(func, user) {
                    ValueKind::Jump(_) | ValueKind::Branch(_) => {
                        self.preds.entry(bb).or_default().push(pred)
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn insert_bb_params(&mut self, func: &mut FunctionData) {
        // add params into basic blocks
        let mut new_bbs = Vec::with_capacity(self.bb_params.len());
        for (bb, var) in &self.bb_params {
            if var.is_empty() {
                continue;
            }

            let bb_with_param = func
                .dfg_mut()
                .new_bb()
                .basic_block_with_params(None, vec![Type::get_i32(); var.len()]);

            // replace the old bb with the new bb
            replace_bb_with(func, *bb, bb_with_param);
            func.dfg_mut().remove_bb(*bb);

            let (_, node) = func.layout_mut().bbs_mut().remove(bb).unwrap();
            func.layout_mut()
                .bbs_mut()
                .push_key_back(bb_with_param)
                .unwrap();
            for &inst in node.insts().keys() {
                func.layout_mut()
                    .bb_mut(bb_with_param)
                    .insts_mut()
                    .push_key_back(inst)
                    .unwrap();
            }

            new_bbs.push((*bb, bb_with_param));
        }

        // keep information consistent with the new dfg
        self.record_preds(func);
        for &(old_bb, new_bb) in &new_bbs {
            let params = self.bb_params.remove(&old_bb).unwrap();
            self.bb_params.insert(new_bb, params);

            self.defs.values_mut().for_each(|d| {
                let def = d.remove(&old_bb);
                if let Some(def) = def {
                    d.insert(new_bb, def);
                }
            });
            self.replace_with.values_mut().for_each(|(bb, _)| {
                if *bb == old_bb {
                    *bb = new_bb;
                }
            });
        }

        for (bb, var) in self.bb_params.clone() {
            let preds = self.preds.get(&bb).unwrap().clone();
            for pred in preds {
                if !self.preds.contains_key(&pred) && func.layout().entry_bb().unwrap() != pred {
                    continue;
                }
                // arg is the def of variable
                let mut args: SmallVec<[Value; 6]> = SmallVec::new();
                for (_, v) in var.iter().enumerate() {
                    args.push(match self.read_variable(func, *v, pred) {
                        Def::Assign(val) => val,
                        Def::Argument(variable) => self.read_argument_value(func, variable, pred),
                    });
                }
                self.add_params_to_inst(func, pred, bb, args);
            }
        }
    }

    fn add_params_to_inst(
        &self,
        func: &mut FunctionData,
        bb: BasicBlock,
        target: BasicBlock,
        args: SmallVec<[Value; 6]>,
    ) {
        for user in func.dfg().bb(target).used_by().clone() {
            if func.layout().parent_bb(user).unwrap() != bb {
                continue;
            }
            let mut user_data = func.dfg().value(user).clone();
            match user_data.kind_mut() {
                ValueKind::Jump(j) => {
                    *j.args_mut() = args.to_vec();
                }
                ValueKind::Branch(br) => {
                    if br.true_bb() == target {
                        *br.true_args_mut() = args.to_vec();
                    } else {
                        *br.false_args_mut() = args.to_vec();
                    }
                }
                _ => unreachable!(),
            }
            func.dfg_mut().replace_value_with(user).raw(user_data);
        }
    }

    fn read_variable(&mut self, func: &FunctionData, variable: Value, bb: BasicBlock) -> Def {
        let def = self.defs.get(&variable).unwrap().get(&bb);
        if let Some(def) = def {
            *def
        } else {
            self.read_variable_recur(func, variable, bb)
        }
    }

    fn read_variable_recur(&mut self, func: &FunctionData, variable: Value, bb: BasicBlock) -> Def {
        let preds = self.preds.get(&bb);
        if preds.is_none() {
            return Def::Argument(variable);
        }
        let preds = preds.unwrap().clone();
        let def = if !self.is_sealed(bb) {
            self.incomplete_bbs.entry(bb).or_default().push(variable);
            self.bb_params.entry(bb).or_default().push(variable);

            Def::Argument(variable)
        } else if preds.len() == 1 {
            self.read_variable(func, variable, *preds.first().unwrap())
        } else {
            self.bb_params.entry(bb).or_default().push(variable);
            self.defs
                .get_mut(&variable)
                .unwrap()
                .insert(bb, Def::Argument(variable));
            for pred in preds {
                self.read_variable(func, variable, pred);
            }

            Def::Argument(variable)
        };
        self.defs.get_mut(&variable).unwrap().insert(bb, def);

        def
    }

    fn read_argument_value(&self, func: &FunctionData, variable: Value, bb: BasicBlock) -> Value {
        let preds = self.preds.get(&bb).unwrap();
        if preds.len() == 1 {
            return self.read_argument_value(func, variable, *preds.first().unwrap());
        }
        let mut i = 0;
        let arg_idx = loop {
            if self.bb_params.get(&bb).unwrap()[i] == variable {
                break i;
            }
            i += 1;
        };
        func.dfg().bb(bb).params()[arg_idx]
    }

    fn replace_var_with_arg(&self, func: &mut FunctionData, origin: Value, variable: Value) {
        let bb = func.layout().parent_bb(origin).unwrap();
        let replace_by = self.read_argument_value(func, variable, bb);
        replace_variable(func, origin, replace_by);
    }

    /// Replace the load of local variables with the variable's definition
    fn replace_load_with_def(&mut self, func: &mut FunctionData) {
        for (&origin, &(bb, replace_by)) in &self.replace_with {
            match replace_by {
                Def::Assign(val) => replace_variable(func, origin, val),
                Def::Argument(variable) => self.replace_var_with_arg(func, origin, variable),
            }
            func.dfg_mut().remove_value(origin);
            func.layout_mut().bb_mut(bb).insts_mut().remove(&origin);
        }
    }

    /// Remove all local variables, except arrays
    fn remove_local_variables(&self, func: &mut FunctionData) {
        let entry_bb = func.layout().entry_bb().unwrap();
        for &local in self.defs.keys() {
            for store in func.dfg().value(local).used_by().clone() {
                let bb = func.layout().parent_bb(store).unwrap();
                func.layout_mut().bb_mut(bb).insts_mut().remove(&store);
                func.dfg_mut().remove_value(store);
            }
            func.dfg_mut().remove_value(local);
            func.layout_mut()
                .bb_mut(entry_bb)
                .insts_mut()
                .remove(&local);
        }
    }

    fn is_sealed(&self, bb: BasicBlock) -> bool {
        for pred in self.preds.get(&bb).unwrap() {
            if !self.filled_bbs.contains(pred) {
                return false;
            }
        }

        true
    }

    fn try_seal(&mut self, func: &FunctionData) {
        for (bb, vars) in self.incomplete_bbs.clone() {
            if self.is_sealed(bb) {
                for v in vars {
                    for pred in self.preds.get(&bb).unwrap().clone() {
                        self.read_variable(func, v, pred);
                    }
                }
                self.incomplete_bbs.remove(&bb);
            }
        }
    }

    fn clear(&mut self) {
        self.defs.clear();
        self.replace_with.clear();
        self.bb_params.clear();
        self.preds.clear();
        self.filled_bbs.clear();
        self.incomplete_bbs.clear();
    }
}
