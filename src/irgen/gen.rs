use koopa::ir::builder_traits::{LocalInstBuilder, ValueBuilder};

use super::*;
use crate::ast::*;

pub trait GenerateIR<'i> {
    type Out;

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out>;
}

impl<'i> GenerateIR<'i> for CompUnit {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        self.func_def.generate_ir(program, recorder)
    }
}

impl<'i> GenerateIR<'i> for FuncDef {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        // generate the function and its entry & end blocks
        recorder.new_func(program, &self.ident, Type::get_i32());

        // enter the entry block
        let entry_bb = recorder.func().entry_bb();
        recorder.func_mut().push_bb(program, entry_bb);
        // allocate the return value
        let ret_val = recorder.new_value(program).alloc(Type::get_i32());
        recorder
            .func()
            .set_value_name(program, "%ret".to_owned(), ret_val);
        recorder.func().push_inst(program, ret_val);
        recorder.func_mut().set_ret_val(ret_val);

        // jump to the main body block
        let main_body = recorder.func().new_anonymous_bb(program);
        let jump = recorder.new_value(program).jump(main_body);
        recorder.func().push_inst(program, jump);

        // enter the main body block
        recorder.func_mut().push_bb(program, main_body);
        // generate IR for the main body block
        self.block.generate_ir(program, recorder)?;

        // jump to the end block
        let end_bb = recorder.func().end_bb();
        let jump = recorder.new_value(program).jump(end_bb);
        recorder.func().push_inst(program, jump);

        // enter the end block
        recorder.func_mut().push_bb(program, end_bb);

        // load the return value and return
        let ret_val = recorder.func().ret_val().unwrap();
        let ld = recorder.new_value(program).load(ret_val);
        let ret = recorder.new_value(program).ret(Some(ld));
        recorder.func().push_inst(program, ld);
        recorder.func().push_inst(program, ret);

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Block {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        recorder.enter_scope();
        for item in &self.items {
            match item {
                BlockItem::Decl(decl) => decl.generate_ir(program, recorder)?,
                BlockItem::Stmt(stmt) => stmt.generate_ir(program, recorder)?,
            }
        }
        recorder.exit_scope();

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Decl {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        match self {
            Decl::ConstDecl(decls) => {
                for d in decls {
                    recorder.insert_const_var(&d.name, d.init.const_eval(recorder))?;
                }
            }

            Decl::VarDecl(decls) => {
                for d in decls {
                    let var = recorder.new_value(program).alloc(Type::get_i32());
                    recorder
                        .func()
                        .set_value_name(program, format!("@{}", &d.name), var);
                    recorder.func().push_inst(program, var);

                    if let Some(exp) = &d.init {
                        let init_val = exp.generate_ir(program, recorder)?;
                        let init = recorder.new_value(program).store(init_val, var);
                        recorder.func().push_inst(program, init);
                        recorder.insert_var(&d.name, var, true)?;
                    } else {
                        recorder.insert_var(&d.name, var, false)?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Stmt {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        match self {
            Self::Assign(s) => s.generate_ir(program, recorder),
            Self::Block(s) => s.generate_ir(program, recorder),
            Self::Exp(s) => s
                .as_ref()
                .map_or(Ok(()), |exp| exp.generate_ir(program, recorder).map(|_| ())),
            Self::Return(s) => s.generate_ir(program, recorder),
            Self::Branch(s) => s.generate_ir(program, recorder),
            Self::While(s) => s.generate_ir(program, recorder),
            Self::Break(s) => s.generate_ir(program, recorder),
            Self::Continue(s) => s.generate_ir(program, recorder),
        }
    }
}

impl<'i> GenerateIR<'i> for Assign {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        let dst = match recorder.get_symbol(&self.name).unwrap() {
            Symbol::Var { val, .. } => *val,
            Symbol::ConstVar(_) => bail!("\"{}\" must be a modifiable lvalue", self.name),
        };
        let val = self.val.generate_ir(program, recorder)?;
        let st = recorder.new_value(program).store(val, dst);
        recorder.func().push_inst(program, st);
        recorder.initialize(&self.name)?;

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Branch {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        let true_bb = recorder.func().new_bb(program, "%then");
        let false_bb = recorder.func().new_bb(program, "%else");
        let end_bb = recorder.func().new_bb(program, "%if_end");

        shortcut(program, recorder, &self.cond, true_bb, false_bb)?;

        // enter the "true" block
        recorder.func_mut().push_bb(program, true_bb);
        self.if_stmt.generate_ir(program, recorder)?;

        // jump to the if-end block
        let jump = recorder.new_value(program).jump(end_bb);
        recorder.func().push_inst(program, jump);

        // enter the "false" block
        recorder.func_mut().push_bb(program, false_bb);
        if let Some(el_stmt) = &self.el_stmt {
            el_stmt.generate_ir(program, recorder)?;
        }
        // jump to the if-end block
        let jump = recorder.new_value(program).jump(end_bb);
        recorder.func().push_inst(program, jump);

        // enter the if-end block
        recorder.func_mut().push_bb(program, end_bb);

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for While {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        let loop_entry = recorder.func().new_bb(program, "%loop_entry");
        let loop_body = recorder.func().new_bb(program, "%loop_body");
        let loop_exit = recorder.func().new_bb(program, "%loop_exit");

        // record the loop information
        recorder.enter_loop(loop_entry, loop_exit);

        // jump to the loop entry
        let jump = recorder.new_value(program).jump(loop_entry);
        recorder.func().push_inst(program, jump);

        // check the loop condition
        recorder.func_mut().push_bb(program, loop_entry);
        shortcut(program, recorder, &self.cond, loop_body, loop_exit)?;

        // enter the loop body block
        recorder.func_mut().push_bb(program, loop_body);
        self.stmt.generate_ir(program, recorder)?;

        // jump back to the loop entry
        let jump = recorder.new_value(program).jump(loop_entry);
        recorder.func().push_inst(program, jump);

        // enter the exit of loop
        recorder.func_mut().push_bb(program, loop_exit);
        recorder.exit_loop();

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Break {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        if !recorder.inside_loop() {
            bail!("break statement occurs outside the loop");
        }

        let loop_exit = recorder.loop_exit();
        let jump = recorder.new_value(program).jump(loop_exit);
        recorder.func().push_inst(program, jump);

        let next_bb = recorder.func().new_anonymous_bb(program);
        recorder.func_mut().push_bb(program, next_bb);

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Continue {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        if !recorder.inside_loop() {
            bail!("continue statement occurs outside the loop");
        }

        // instantly jump to the loop entry
        let loop_entry = recorder.loop_entry();
        let jump = recorder.new_value(program).jump(loop_entry);
        recorder.func().push_inst(program, jump);

        // enter the next block (unreachable)
        let next_bb = recorder.func().new_anonymous_bb(program);
        recorder.func_mut().push_bb(program, next_bb);

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Return {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        if let Some(ret_val) = &self.ret_val {
            let ret_val = ret_val.generate_ir(program, recorder)?;
            let dst = recorder.func().ret_val().unwrap();
            let st = recorder.new_value(program).store(ret_val, dst);
            recorder.func().push_inst(program, st);
        }
        let end_bb = recorder.func().end_bb();
        let jump = recorder.new_value(program).jump(end_bb);
        recorder.func().push_inst(program, jump);

        let next_bb = recorder.func().new_anonymous_bb(program);
        recorder.func_mut().push_bb(program, next_bb);

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Exp {
    type Out = Value;

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        let val = match self {
            Exp::Integer(i) => recorder.new_value(program).integer(*i),
            Exp::Uxp(uxp) => uxp.generate_ir(program, recorder)?,
            Exp::Bxp(bxp) => bxp.generate_ir(program, recorder)?,
            Exp::LVal(name, ..) => match recorder.get_symbol(name).unwrap() {
                Symbol::ConstVar(i) => recorder.new_value(program).integer(*i),
                Symbol::Var { val, init } => load_var(program, recorder, *val, *init),
            },
            Exp::Error => panic!("expected an expression"),
        };

        Ok(val)
    }
}

impl<'i> GenerateIR<'i> for BinaryExp {
    type Out = Value;

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        let lhs = self.lhs.generate_ir(program, recorder)?;
        let rhs = self.rhs.generate_ir(program, recorder)?;

        let lkind = recorder.value_kind(program, lhs);
        let rkind = recorder.value_kind(program, rhs);
        if let (ValueKind::Integer(l), ValueKind::Integer(r)) = (lkind, rkind) {
            let result = eval_binary(self.op, l.value(), r.value());
            return Ok(recorder.new_value(program).integer(result));
        }

        let val = match self.op {
            BinaryOp::And => logical_and(program, recorder, lhs, rhs),
            BinaryOp::Or => logical_or(program, recorder, lhs, rhs),
            _ => binary(program, recorder, self.op.into(), lhs, rhs),
        };

        Ok(val)
    }
}

impl<'i> GenerateIR<'i> for UnaryExp {
    type Out = Value;

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        let opr = self.rhs.generate_ir(program, recorder)?;

        if let ValueKind::Integer(i) = recorder.value_kind(program, opr) {
            let result = eval_unary(self.op, i.value());
            return Ok(recorder.new_value(program).integer(result));
        }

        let val = match self.op {
            UnaryOp::Nop => opr,
            UnaryOp::Neg => negative(program, recorder, opr),
            UnaryOp::Not => logical_not(program, recorder, opr),
        };

        Ok(val)
    }
}

fn shortcut<'i>(
    program: &mut Program,
    recorder: &mut ProgramRecorder<'i>,
    cond: &'i Box<Exp>,
    true_bb: BasicBlock,
    false_bb: BasicBlock,
) -> Result<()> {
    if !cond.is_logical_exp() {
        let cond = cond.generate_ir(program, recorder)?;
        let br = recorder.new_value(program).branch(cond, true_bb, false_bb);
        recorder.func().push_inst(program, br);

        return Ok(());
    }

    let cond = cond.get_bxp().unwrap();
    match cond.op {
        BinaryOp::And => {
            let check_rhs = recorder.func().new_anonymous_bb(program);
            shortcut(program, recorder, &cond.lhs, check_rhs, false_bb)?;
            recorder.func_mut().push_bb(program, check_rhs);
            shortcut(program, recorder, &cond.rhs, true_bb, false_bb)
        }
        BinaryOp::Or => {
            let check_rhs = recorder.func().new_anonymous_bb(program);
            shortcut(program, recorder, &cond.lhs, true_bb, check_rhs)?;
            recorder.func_mut().push_bb(program, check_rhs);
            shortcut(program, recorder, &cond.rhs, true_bb, false_bb)
        }
        _ => unreachable!(),
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
