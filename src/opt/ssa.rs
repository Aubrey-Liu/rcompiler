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
    cfg: HashMap<BasicBlock, SmallVec<[BasicBlock; 4]>>,
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
        self.build_ssa(f);
    }
}

impl SsaBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    fn build_ssa(&mut self, f: &mut FunctionData) {
        self.walk_bbs(f);
        self.insert_bb_params(f);
        self.replace_load_with_def(f);
        self.remove_local_variables(f);
        self.clear();
    }

    fn walk_bbs(&mut self, f: &mut FunctionData) {
        self.update_cfg(f);

        for (&bb, node) in f.layout().bbs().iter() {
            for &val in node.insts().keys() {
                let val_data = f.dfg().value(val);
                match val_data.kind() {
                    ValueKind::Alloc(_) => {
                        // record local variables
                        if let TypeKind::Pointer(base_ty) = val_data.ty().kind() {
                            // only deal with integer type
                            if matches!(base_ty.kind(), TypeKind::Int32 | TypeKind::Pointer(_)) {
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
                        if let ValueKind::Load(l) = value_kind(f, s.value()) {
                            if self.defs.contains_key(&l.src()) {
                                def = self.read_variable(f, l.src(), bb);
                            }
                        }
                        self.defs.get_mut(&s.dest()).unwrap().insert(bb, def);
                    }
                    ValueKind::Load(l) => {
                        if !self.defs.contains_key(&l.src()) {
                            continue;
                        }
                        let def = self.read_variable(f, l.src(), bb);
                        self.replace_with.insert(val, (bb, def));
                    }
                    _ => {}
                }
            }
            self.filled_bbs.insert(bb);
            self.try_seal(f);
        }
    }

    fn update_cfg(&mut self, f: &mut FunctionData) {
        if !self.cfg.is_empty() {
            self.cfg.clear();
        }
        for &bb in f.layout().bbs().keys() {
            for &user in f.dfg().bb(bb).used_by() {
                let pred = f.layout().parent_bb(user).unwrap();
                match value_kind(f, user) {
                    ValueKind::Jump(_) | ValueKind::Branch(_) => {
                        self.cfg.entry(bb).or_default().push(pred)
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn insert_bb_params(&mut self, f: &mut FunctionData) {
        // add params into basic blocks
        let mut new_bbs = Vec::with_capacity(self.bb_params.len());
        for (bb, var) in &self.bb_params {
            if var.is_empty() {
                continue;
            }

            let bb_with_param = f
                .dfg_mut()
                .new_bb()
                .basic_block_with_params(None, vec![Type::get_i32(); var.len()]);

            // replace the old bb with the new bb
            replace_bb_with(f, *bb, bb_with_param);
            f.dfg_mut().remove_bb(*bb);

            f.layout_mut()
                .bbs_mut()
                .cursor_mut(*bb)
                .insert_key_after(bb_with_param)
                .unwrap();
            let (_, node) = f.layout_mut().bbs_mut().remove(bb).unwrap();
            for &inst in node.insts().keys() {
                f.layout_mut()
                    .bb_mut(bb_with_param)
                    .insts_mut()
                    .push_key_back(inst)
                    .unwrap();
            }

            new_bbs.push((*bb, bb_with_param));
        }

        // keep information consistent with the new dfg
        self.update_cfg(f);
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
            let preds = self.cfg.get(&bb).unwrap().clone();
            for pred in preds {
                if !self.cfg.contains_key(&pred) && f.layout().entry_bb().unwrap() != pred {
                    continue;
                }
                // arg is the def of variable
                let mut args: SmallVec<[Value; 6]> = SmallVec::new();
                for (_, v) in var.iter().enumerate() {
                    args.push(match self.read_variable(f, *v, pred) {
                        Def::Assign(val) => val,
                        Def::Argument(variable) => self.read_argument_value(f, variable, pred),
                    });
                }
                self.add_params_to_inst(f, pred, bb, args);
            }
        }
    }

    fn add_params_to_inst(
        &self,
        f: &mut FunctionData,
        bb: BasicBlock,
        target: BasicBlock,
        args: SmallVec<[Value; 6]>,
    ) {
        let exit = last_inst_of_bb(f, bb);
        let mut user_data = f.dfg().value(exit).clone();
        match user_data.kind_mut() {
            ValueKind::Jump(j) => *j.args_mut() = args.to_vec(),
            ValueKind::Branch(br) => {
                if br.true_bb() == target {
                    *br.true_args_mut() = args.to_vec();
                }
                if br.false_bb() == target {
                    *br.false_args_mut() = args.to_vec();
                }
            }
            _ => unreachable!(),
        }
        f.dfg_mut().replace_value_with(exit).raw(user_data);
    }

    fn read_variable(&mut self, f: &FunctionData, variable: Value, bb: BasicBlock) -> Def {
        let def = self.defs.get(&variable).unwrap().get(&bb);
        if let Some(def) = def {
            *def
        } else {
            self.read_variable_recur(f, variable, bb)
        }
    }

    fn read_variable_recur(&mut self, f: &FunctionData, variable: Value, bb: BasicBlock) -> Def {
        let preds = self.cfg.get(&bb).unwrap().clone();
        let def = if !self.is_sealed(bb) {
            self.incomplete_bbs.entry(bb).or_default().push(variable);
            self.bb_params.entry(bb).or_default().push(variable);

            Def::Argument(variable)
        } else if preds.len() == 1 {
            self.read_variable(f, variable, *preds.first().unwrap())
        } else {
            self.bb_params.entry(bb).or_default().push(variable);
            self.defs
                .get_mut(&variable)
                .unwrap()
                .insert(bb, Def::Argument(variable));
            for pred in preds {
                self.read_variable(f, variable, pred);
            }

            Def::Argument(variable)
        };
        self.defs.get_mut(&variable).unwrap().insert(bb, def);

        def
    }

    fn read_argument_value(&self, f: &FunctionData, variable: Value, bb: BasicBlock) -> Value {
        let preds = self.cfg.get(&bb).unwrap();
        if preds.len() == 1 {
            return self.read_argument_value(f, variable, *preds.first().unwrap());
        }
        let mut i = 0;
        let arg_idx = loop {
            if self.bb_params.get(&bb).unwrap()[i] == variable {
                break i;
            }
            i += 1;
        };
        f.dfg().bb(bb).params()[arg_idx]
    }

    fn replace_var_with_arg(&self, f: &mut FunctionData, origin: Value, variable: Value) {
        let bb = f.layout().parent_bb(origin).unwrap();
        let replace_by = self.read_argument_value(f, variable, bb);
        replace_variable(f, origin, replace_by);
    }

    /// Replace the load of local variables with the variable's definition
    fn replace_load_with_def(&mut self, f: &mut FunctionData) {
        for (&origin, &(bb, replace_by)) in &self.replace_with {
            match replace_by {
                Def::Assign(val) => replace_variable(f, origin, val),
                Def::Argument(variable) => self.replace_var_with_arg(f, origin, variable),
            }
            f.dfg_mut().remove_value(origin);
            f.layout_mut().bb_mut(bb).insts_mut().remove(&origin);
        }
    }

    /// Remove all local variables, except arrays
    fn remove_local_variables(&self, f: &mut FunctionData) {
        let entry_bb = f.layout().entry_bb().unwrap();
        for &local in self.defs.keys() {
            for store in f.dfg().value(local).used_by().clone() {
                let bb = f.layout().parent_bb(store).unwrap();
                f.layout_mut().bb_mut(bb).insts_mut().remove(&store);
                f.dfg_mut().remove_value(store);
            }
            f.dfg_mut().remove_value(local);
            f.layout_mut().bb_mut(entry_bb).insts_mut().remove(&local);
        }
    }

    fn is_sealed(&self, bb: BasicBlock) -> bool {
        for pred in self.cfg.get(&bb).unwrap() {
            if !self.filled_bbs.contains(pred) {
                return false;
            }
        }

        true
    }

    fn try_seal(&mut self, f: &FunctionData) {
        for (bb, vars) in self.incomplete_bbs.clone() {
            if self.is_sealed(bb) {
                for v in vars {
                    for pred in self.cfg.get(&bb).unwrap().clone() {
                        self.read_variable(f, v, pred);
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
        self.cfg.clear();
        self.filled_bbs.clear();
        self.incomplete_bbs.clear();
    }
}
