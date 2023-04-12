use std::collections::HashMap;

use koopa::ir::{
    builder_traits::{LocalInstBuilder, ValueBuilder},
    BasicBlock, BinaryOp, FunctionData, Value, ValueKind,
};
use smallvec::SmallVec;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
enum CellType {
    Top,
    Constant(i32),
    Bottom,
}

#[derive(Debug, Clone, Copy)]
struct LatticeCell {
    ty: CellType,
    bb: BasicBlock,
}

#[derive(Debug, Clone, Copy)]
struct SsaEdge {
    src: Value,
    dst: Value,
}

#[derive(Debug, Clone, Copy)]
struct FlowEdge {
    src: BasicBlock,
    dst: BasicBlock,
    executable: bool,
}

type EdgeId = usize;

#[derive(Debug, Default)]
pub struct SCCP {
    flow_worklist: Vec<EdgeId>,
    ssa_worklist: Vec<SsaEdge>,
    lattice_cells: HashMap<Value, LatticeCell>,
    edges: Vec<FlowEdge>,
    incoming_edges: HashMap<BasicBlock, SmallVec<[EdgeId; 4]>>,
    outcoming_edges: HashMap<BasicBlock, SmallVec<[EdgeId; 2]>>,
}

impl FunctionPass for SCCP {
    fn run_on(&mut self, f: &mut FunctionData) {
        if f.layout().entry_bb().is_some() {
            self.work(f);
        }
    }
}

impl SCCP {
    pub fn new() -> Self {
        Default::default()
    }

    fn init(&mut self, f: &FunctionData) {
        for &bb in f.layout().bbs().keys() {
            for &user in f.dfg().bb(bb).used_by() {
                let pred = f.layout().parent_bb(user).unwrap();
                let edge = FlowEdge::new(pred, bb);
                let idx = self.edges.len();
                self.edges.push(edge);

                match value_kind(f, user) {
                    ValueKind::Jump(_) | ValueKind::Branch(_) => {
                        self.incoming_edges.entry(bb).or_default().push(idx);
                        self.outcoming_edges.entry(pred).or_default().push(idx);
                    }
                    _ => unreachable!(),
                }
            }

            for &val in f.layout().bbs().node(&bb).unwrap().insts().keys() {
                if self.is_expr(f, val) {
                    self.lattice_cells
                        .insert(val, LatticeCell::new(CellType::Top, bb));
                }
            }

            for &p in f.dfg().bb(bb).params() {
                self.lattice_cells
                    .insert(p, LatticeCell::new(CellType::Top, bb));
            }
        }
    }

    fn work(&mut self, f: &mut FunctionData) {
        self.init(f);
        self.visit_entry(f);

        while !self.is_terminate() {
            while !self.flow_worklist.is_empty() {
                self.visit_flow_edge(f);
            }
            while !self.ssa_worklist.is_empty() {
                self.visit_ssa_edge(f);
            }
        }

        self.remove_all_consts(f);
        self.remove_trivial_branch(f);
    }

    fn visit_entry(&mut self, f: &FunctionData) {
        let entry_bb = f.layout().entry_bb().unwrap();
        for &val in f.layout().bbs().node(&entry_bb).unwrap().insts().keys() {
            if self.is_expr(f, val) {
                self.visit_expr(f, val);
            }
        }
        let out_edges = self.outcoming_edges.get(&entry_bb);
        if out_edges.is_none() {
            return;
        }
        let out_edges = out_edges.unwrap();
        if out_edges.len() == 1 {
            self.flow_worklist.push(out_edges[0]);
        }
    }

    fn visit_flow_edge(&mut self, f: &FunctionData) {
        let id = self.flow_worklist.pop().unwrap();
        let mut edge = self.edges.get_mut(id).unwrap();
        if edge.executable {
            return;
        }

        // Mark the ExecutableFlag of the edge as true
        edge.executable = true;

        // Visit all block arguments
        let bb = edge.dst;
        for &p in f.dfg().bb(bb).params() {
            self.visit_param(f, bb, p);
        }

        // If the current node is visited for the first time, visit all expressions
        if !self.is_visited(bb) {
            for &val in f.layout().bbs().node(&bb).unwrap().insts().keys() {
                if self.is_expr(f, val) {
                    self.visit_expr(f, val);
                }
            }
        }

        let out_edges = self.outcoming_edges.get(&bb);
        if out_edges.is_none() {
            return;
        }
        let out_edges = out_edges.unwrap();
        if out_edges.len() == 1 {
            self.flow_worklist.push(out_edges[0]);
        }
    }

    fn visit_ssa_edge(&mut self, f: &FunctionData) {
        let edge = self.ssa_worklist.pop().unwrap();
        let src = edge.src;
        let dst = edge.dst;

        let param = self.is_part_of_phi(f, src, dst);
        if let Some((bb, param)) = param {
            self.visit_param(f, bb, param);
        } else {
            let dst_bb = f.layout().parent_bb(dst).unwrap();
            if self.is_reachable(dst_bb) {
                self.visit_expr(f, dst);
            }
        }
    }

    fn visit_param(&mut self, f: &FunctionData, bb: BasicBlock, param: Value) {
        if let ValueKind::BlockArgRef(arg) = value_kind(f, param) {
            let arg_idx = arg.index();
            let mut oprands: SmallVec<[CellType; 4]> = SmallVec::new();
            for &id in self.incoming_edges.get(&bb).unwrap() {
                let edge = self.edges[id];
                if !edge.executable {
                    continue;
                }
                let src = edge.src;
                let src_exit = f
                    .layout()
                    .bbs()
                    .node(&src)
                    .map(|n| *n.insts().back_key().unwrap())
                    .unwrap();
                let arg = match value_kind(f, src_exit) {
                    ValueKind::Jump(j) => j.args()[arg_idx],
                    ValueKind::Branch(br) if br.true_bb() == bb => br.true_args()[arg_idx],
                    ValueKind::Branch(br) => br.false_args()[arg_idx],
                    _ => unreachable!(),
                };

                oprands.push(self.value_to_type(f, arg));
            }

            let new_ty = self.meet(&oprands);

            let old_cell = *self.lattice_cells.get(&param).unwrap();
            if old_cell.ty == new_ty {
                return;
            }

            self.lattice_cells
                .insert(param, LatticeCell::new(new_ty, old_cell.bb));
            self.add_ssa_edges(f, param);
        }
    }

    fn visit_expr(&mut self, f: &FunctionData, expr: Value) {
        if let ValueKind::Binary(b) = value_kind(f, expr) {
            let old_cell = *self.lattice_cells.get(&expr).unwrap();
            let lhs = self.value_to_type(f, b.lhs());
            let rhs = self.value_to_type(f, b.rhs());
            let new_ty = self.evaluate(b.op(), lhs, rhs);
            if old_cell.ty == new_ty {
                return;
            }
            self.lattice_cells
                .insert(expr, LatticeCell::new(new_ty, old_cell.bb));
            self.add_ssa_edges(f, expr);
        }
    }

    fn add_ssa_edges(&mut self, f: &FunctionData, val: Value) {
        let cell_ty = self.lattice_cells.get(&val).unwrap().ty;
        for &user in f.dfg().value(val).used_by() {
            let bb = f.layout().parent_bb(user).unwrap();
            if self.is_control_br(f, user, val) {
                if let ValueKind::Branch(br) = value_kind(f, user) {
                    match cell_ty {
                        CellType::Top => {}
                        CellType::Constant(i) if i != 0 => {
                            let id = self.get_edge_id(bb, br.true_bb());
                            self.flow_worklist.push(id);
                        }
                        CellType::Constant(_) => {
                            let id = self.get_edge_id(bb, br.false_bb());
                            self.flow_worklist.push(id);
                        }
                        CellType::Bottom => {
                            let edges = self.outcoming_edges.get(&bb).unwrap();
                            self.flow_worklist.extend(edges);
                        }
                    }
                }
            }
            if self.is_expr(f, user) {
                self.ssa_worklist.push(SsaEdge {
                    src: val,
                    dst: user,
                });
            }
        }
    }

    fn remove_all_consts(&self, f: &mut FunctionData) {
        for (val, cell) in &self.lattice_cells {
            if let CellType::Constant(i) = cell.ty {
                if let ValueKind::BlockArgRef(arg) = value_kind(f, *val).clone() {
                    self.remove_const_param(f, cell.bb, arg.index(), i);
                } else {
                    f.dfg_mut().replace_value_with(*val).integer(i);
                    f.layout_mut().bb_mut(cell.bb).insts_mut().remove(val);
                }
            }
        }
    }

    fn remove_const_param(&self, f: &mut FunctionData, bb: BasicBlock, idx: usize, val: i32) {
        self.remove_unused_arg(f, bb, idx);
        let param = f.dfg_mut().bb_mut(bb).params_mut().remove(idx);
        f.dfg_mut().replace_value_with(param).integer(val);
    }

    fn remove_unused_arg(&self, f: &mut FunctionData, bb: BasicBlock, idx: usize) {
        for &id in self.incoming_edges.get(&bb).unwrap() {
            let edge = &self.edges[id];
            let exit = f
                .layout()
                .bbs()
                .node(&edge.src)
                .map(|n| *n.insts().back_key().unwrap())
                .unwrap();
            let mut data = f.dfg().value(exit).clone();
            match data.kind_mut() {
                ValueKind::Jump(j) => {
                    j.args_mut().remove(idx);
                }
                ValueKind::Branch(br) => {
                    if br.true_bb() == bb {
                        br.true_args_mut().remove(idx);
                    } else {
                        br.false_args_mut().remove(idx);
                    }
                }
                _ => unreachable!(),
            }
            f.dfg_mut().replace_value_with(exit).raw(data);
        }
    }

    fn remove_trivial_branch(&self, f: &mut FunctionData) {
        let flow_insts: Vec<_> = f
            .layout()
            .bbs()
            .nodes()
            .map(|node| *node.insts().back_key().unwrap())
            .collect();

        for val in flow_insts {
            match value_kind(f, val).clone() {
                ValueKind::Branch(br) => {
                    let cond = br.cond();
                    if let ValueKind::Integer(i) = value_kind(f, cond).clone() {
                        let (target, args) = if i.value() != 0 {
                            (br.true_bb(), br.true_args().to_vec())
                        } else {
                            (br.false_bb(), br.false_args().to_vec())
                        };
                        f.dfg_mut()
                            .replace_value_with(val)
                            .jump_with_args(target, args);
                    }
                }
                _ => {}
            }
        }
    }

    fn is_reachable(&self, bb: BasicBlock) -> bool {
        if let Some(edges) = self.incoming_edges.get(&bb) {
            for &id in edges {
                let e = self.edges.get(id).unwrap();
                if e.executable {
                    return true;
                }
            }
        }

        false
    }

    fn is_part_of_phi(
        &self,
        f: &FunctionData,
        def: Value,
        val: Value,
    ) -> Option<(BasicBlock, Value)> {
        match value_kind(f, val) {
            ValueKind::Jump(j) => {
                let idx = j
                    .args()
                    .iter()
                    .enumerate()
                    .find(|(_, &arg)| arg == def)
                    .unwrap()
                    .0;
                let param = f.dfg().bb(j.target()).params()[idx];

                Some((j.target(), param))
            }
            ValueKind::Branch(br) => {
                let l_idx = br
                    .true_args()
                    .iter()
                    .enumerate()
                    .find(|(_, &arg)| arg == def);
                let r_idx = br
                    .false_args()
                    .iter()
                    .enumerate()
                    .find(|(_, &arg)| arg == def);

                let result = if let Some((i, _)) = l_idx {
                    (br.true_bb(), f.dfg().bb(br.true_bb()).params()[i])
                } else if let Some((i, _)) = r_idx {
                    (br.false_bb(), f.dfg().bb(br.false_bb()).params()[i])
                } else {
                    return None;
                };

                Some(result)
            }
            _ => None,
        }
    }

    fn is_terminate(&self) -> bool {
        self.flow_worklist.is_empty() && self.ssa_worklist.is_empty()
    }

    fn is_visited(&self, bb: BasicBlock) -> bool {
        let mut executable_in_edges = 0;
        for &id in self.incoming_edges.get(&bb).unwrap() {
            let edge = self.edges.get(id).unwrap();
            if edge.executable {
                executable_in_edges += 1;
            }
        }

        executable_in_edges != 1
    }

    fn meet(&self, operands: &[CellType]) -> CellType {
        let mut res = CellType::Top;
        for &opr in operands {
            match (res, opr) {
                (CellType::Top, _) => res = opr,
                (_, CellType::Top) | (CellType::Bottom, _) => {}
                (CellType::Constant(i), CellType::Constant(j)) if i == j => {}
                (CellType::Constant(_), CellType::Constant(_)) | (_, CellType::Bottom) => {
                    res = CellType::Bottom;
                }
            }
        }

        res
    }

    fn value_to_type(&self, f: &FunctionData, val: Value) -> CellType {
        if let ValueKind::Integer(i) = value_kind(f, val) {
            CellType::Constant(i.value())
        } else if self.lattice_cells.contains_key(&val) {
            self.lattice_cells.get(&val).unwrap().ty
        } else {
            CellType::Bottom
        }
    }

    fn is_control_br(&self, f: &FunctionData, br: Value, val: Value) -> bool {
        if let ValueKind::Branch(br) = value_kind(f, br) {
            if br.cond() == val {
                return true;
            }
        }

        false
    }

    fn is_expr(&self, f: &FunctionData, val: Value) -> bool {
        matches!(value_kind(f, val), ValueKind::Binary(_))
    }

    fn get_edge_id(&self, src: BasicBlock, dst: BasicBlock) -> EdgeId {
        let ids = self.outcoming_edges.get(&src).unwrap();
        let edge0 = self.edges[ids[0]];
        if edge0.src == src && edge0.dst == dst {
            ids[0]
        } else {
            ids[1]
        }
    }

    fn evaluate(&mut self, op: BinaryOp, lhs_ty: CellType, rhs_ty: CellType) -> CellType {
        match (lhs_ty, rhs_ty) {
            (CellType::Bottom, _) | (_, CellType::Bottom) => CellType::Bottom,
            (CellType::Top, _) | (_, CellType::Top) => CellType::Top,
            (CellType::Constant(lhs), CellType::Constant(rhs)) => {
                let lhs = lhs;
                let rhs = rhs;
                let result = match op {
                    BinaryOp::Add => lhs + rhs,
                    BinaryOp::Sub => lhs - rhs,
                    BinaryOp::Mul => lhs * rhs,
                    BinaryOp::And => (lhs != 0 && rhs != 0) as i32,
                    BinaryOp::Or => (lhs != 0 || rhs != 0) as i32,
                    BinaryOp::Eq => (lhs == rhs) as i32,
                    BinaryOp::NotEq => (lhs != rhs) as i32,
                    BinaryOp::Lt => (lhs < rhs) as i32,
                    BinaryOp::Le => (lhs <= rhs) as i32,
                    BinaryOp::Gt => (lhs > rhs) as i32,
                    BinaryOp::Ge => (lhs >= rhs) as i32,
                    BinaryOp::Div => {
                        if rhs != 0 {
                            lhs / rhs
                        } else {
                            panic!("attempt to divide an integer by zero");
                        }
                    }
                    BinaryOp::Mod => {
                        if rhs != 0 {
                            lhs % rhs
                        } else {
                            panic!("attempt to calculate the remainder of an integer with a divisor of zero");
                        }
                    }
                    _ => unimplemented!(),
                };

                CellType::Constant(result)
            }
        }
    }
}

impl LatticeCell {
    fn new(ty: CellType, bb: BasicBlock) -> Self {
        Self { ty, bb }
    }
}

impl FlowEdge {
    fn new(src: BasicBlock, dst: BasicBlock) -> Self {
        Self {
            src,
            dst,
            executable: false,
        }
    }
}
