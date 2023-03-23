use super::*;
use crate::ast::*;

impl<'input> CompUnit {
    pub(in crate::irgen) fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
    ) -> Result<Program> {
        let mut program = Program::new();
        self.func_def.generate(symt, &mut program)?;
        Ok(program)
    }
}

impl<'input> FuncDef {
    pub(in crate::irgen) fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        program: &mut Program,
    ) -> Result<()> {
        let fib = new_func(program, &self.ident);
        let func = program.func_mut(fib);
        let mut flow = FlowGraph::new();
        self.block.generate_entry(symt, &mut flow, func)?;

        for (bb, flow) in &flow {
            match flow {
                Flow::Branch(cond, true_bb, false_bb) => {
                    branch_from(func, *cond, *bb, *true_bb, *false_bb);
                }
                Flow::Jump(target) => {
                    check_and_jump(func, *bb, *target);
                }
            }
        }

        Ok(())
    }
}

impl<'input> Block {
    pub(in crate::irgen) fn generate_entry(
        &'input self,
        symt: &mut SymbolTable<'input>,
        flow: &mut FlowGraph,
        func: &mut FunctionData,
    ) -> Result<()> {
        // Create the entry block
        let bb = new_bb(func, "%entry");
        self.generate(symt, flow, func, bb, bb)?;

        Ok(())
    }

    pub(in crate::irgen) fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        flow: &mut FlowGraph,
        func: &mut FunctionData,
        bb: BasicBlock,
        link_to: BasicBlock,
    ) -> Result<()> {
        let mut bb = bb;

        for item in &self.items {
            match item {
                BlockItem::Decl(decl) => decl.generate(symt, func, bb)?,
                BlockItem::Stmt(stmt) => bb = stmt.generate(symt, flow, func, bb, link_to)?,
            }

            if is_finish(func, bb) {
                return Ok(());
            }
        }

        Ok(())
    }
}

impl<'input> Decl {
    pub(in crate::irgen) fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        func: &mut FunctionData,
        bb: BasicBlock,
    ) -> Result<()> {
        let mut insts = Vec::new();
        match self {
            Decl::ConstDecl(decls) => {
                for d in decls {
                    symt.insert_const_var(&d.name, d.init.const_eval(symt))?;
                }
            }
            Decl::VarDecl(decls) => {
                for d in decls {
                    let dst = alloc(func);
                    set_value_name(func, dst, &d.name);
                    insts.push(dst);

                    if let Some(exp) = &d.init {
                        let val = exp.generate(symt, func, &mut insts);
                        insts.push(store(func, val, dst));
                        symt.insert_var(&d.name, dst, true)?;
                    } else {
                        symt.insert_var(&d.name, dst, false)?;
                    }
                }
            }
        }
        push_insts(func, bb, &mut insts);

        Ok(())
    }
}

impl<'input> Stmt {
    pub(in crate::irgen) fn generate(
        &'input self,
        symt: &mut SymbolTable<'input>,
        flow: &mut FlowGraph,
        func: &mut FunctionData,
        bb: BasicBlock,
        link_to: BasicBlock,
    ) -> Result<BasicBlock> {
        let mut insts = Vec::new();
        let mut move_to = bb;

        match self {
            Self::Assign(assign) => {
                let dst = match symt.get(&assign.name).unwrap() {
                    Symbol::Var { val, .. } => *val,
                    Symbol::ConstVar(_) => bail!("\"{}\" must be a modifiable lvalue", assign.name),
                };
                let val = assign.val.generate(symt, func, &mut insts);
                insts.push(store(func, val, dst));
                symt.initialize(&assign.name)?;
            }
            Self::Block(block) => {
                symt.enter_scope();
                block.generate(symt, flow, func, bb, link_to)?;
                symt.exit_scope();
            }
            Self::Exp(exp) => {
                exp.as_ref().map(|e| e.generate(symt, func, &mut insts));
            }
            Self::Return(val) => {
                return_from(symt, func, bb, val);
            }
            Self::Branch(br) => {
                move_to = br.generate(symt, flow, func, bb, link_to)?;
            }
            Self::While(w) => {
                move_to = w.generate(symt, flow, func, bb, link_to)?;
            }
            _ => todo!(),
        }

        if !insts.is_empty() {
            push_insts(func, bb, &insts);
        }

        Ok(move_to)
    }
}

impl UnaryExp {
    pub fn generate(
        &self,
        symt: &SymbolTable,
        func: &mut FunctionData,
        insts: &mut Vec<Value>,
    ) -> Value {
        let rhs = self.rhs.generate(symt, func, insts);

        let rkind = func.dfg().value(rhs).kind();
        if let ValueKind::Integer(r) = rkind {
            return integer(func, eval_unary(self.op, r.value()));
        }

        let val = match self.op {
            UnaryOp::Nop => rhs,
            UnaryOp::Neg => neg(func, rhs),
            UnaryOp::Not => not(func, rhs),
        };
        insts.push(val);

        val
    }
}

impl BinaryExp {
    pub fn generate(
        &self,
        symt: &SymbolTable,
        func: &mut FunctionData,
        insts: &mut Vec<Value>,
    ) -> Value {
        let lhs = self.lhs.generate(symt, func, insts);
        let rhs = self.rhs.generate(symt, func, insts);

        // evaluate when expression is const
        let lkind = func.dfg().value(lhs).kind();
        let rkind = func.dfg().value(rhs).kind();
        if let (ValueKind::Integer(l), ValueKind::Integer(r)) = (lkind, rkind) {
            return integer(func, eval_binary(self.op, l.value(), r.value()));
        }

        let val = match self.op {
            BinaryOp::And => land(func, lhs, rhs, insts),
            BinaryOp::Or => lor(func, lhs, rhs, insts),
            _ => binary(func, self.op.into(), lhs, rhs),
        };

        insts.push(val);

        val
    }
}

impl Exp {
    pub fn generate(
        &self,
        symt: &SymbolTable,
        func: &mut FunctionData,
        insts: &mut Vec<Value>,
    ) -> Value {
        match self {
            Exp::Integer(i) => integer(func, *i),
            Exp::Uxp(uxp) => uxp.generate(symt, func, insts),
            Exp::Bxp(bxp) => bxp.generate(symt, func, insts),
            Exp::LVal(name, ..) => match symt.get(name).unwrap() {
                Symbol::ConstVar(i) => integer(func, *i),
                Symbol::Var { val, init } => {
                    if !init {
                        panic!(
                            "uninitialized variable \"{}\" can't be used in an expression",
                            name
                        )
                    }
                    let load = load(func, *val);
                    insts.push(load);

                    load
                }
            },
            Exp::Error => panic!("expected an expression"),
        }
    }
}

impl From<BinaryOp> for IR_BinaryOp {
    fn from(value: BinaryOp) -> Self {
        match value {
            BinaryOp::Add => IR_BinaryOp::Add,
            BinaryOp::Sub => IR_BinaryOp::Sub,
            BinaryOp::Mul => IR_BinaryOp::Mul,
            BinaryOp::Div => IR_BinaryOp::Div,
            BinaryOp::Mod => IR_BinaryOp::Mod,
            BinaryOp::Eq => IR_BinaryOp::Eq,
            BinaryOp::Neq => IR_BinaryOp::NotEq,
            BinaryOp::Lt => IR_BinaryOp::Lt,
            BinaryOp::Le => IR_BinaryOp::Le,
            BinaryOp::Gt => IR_BinaryOp::Gt,
            BinaryOp::Ge => IR_BinaryOp::Ge,
            _ => unreachable!(),
        }
    }
}
