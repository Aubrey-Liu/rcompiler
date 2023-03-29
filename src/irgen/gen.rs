use koopa::ir::builder_traits::{GlobalInstBuilder, LocalInstBuilder, ValueBuilder};

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
        recorder.declare_func(program, "getint", vec![], IrType::get_i32())?;
        recorder.declare_func(program, "getch", vec![], IrType::get_i32())?;
        recorder.declare_func(
            program,
            "getarray",
            vec![IrType::get_pointer(IrType::get_i32())],
            IrType::get_i32(),
        )?;
        recorder.declare_func(
            program,
            "putint",
            vec![IrType::get_i32()],
            IrType::get_unit(),
        )?;
        recorder.declare_func(
            program,
            "putch",
            vec![IrType::get_i32()],
            IrType::get_unit(),
        )?;
        recorder.declare_func(
            program,
            "putarray",
            vec![IrType::get_i32(), IrType::get_pointer(IrType::get_i32())],
            IrType::get_unit(),
        )?;
        recorder.declare_func(program, "starttime", vec![], IrType::get_unit())?;
        recorder.declare_func(program, "stoptime", vec![], IrType::get_unit())?;

        self.items
            .iter()
            .try_for_each(|item| item.generate_ir(program, recorder))
    }
}

impl<'i> GenerateIR<'i> for GlobalItem {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        match self {
            GlobalItem::Decl(i) => i.generate_ir(program, recorder),
            GlobalItem::Func(i) => i.generate_ir(program, recorder),
        }
    }
}

impl<'i> GenerateIR<'i> for FuncDef {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        recorder.enter_scope();

        // generate the function and its entry & end blocks
        let id = recorder.new_func(program, self);
        recorder.insert_func(&self.ident, id)?;

        // enter the entry block
        let entry_bb = recorder.func().entry_bb();
        recorder.func_mut().push_bb(program, entry_bb);

        let param_values: Vec<Value> = program
            .func(recorder.func().id())
            .params()
            .iter()
            .map(|v| *v)
            .collect();
        for (value, param) in param_values.iter().zip(&self.params) {
            let dst = alloc(
                recorder,
                program,
                param.ty.into_ty(),
                Some(format!("%{}", &param.ident)),
            );
            recorder.insert_var(&param.ident, dst, true).unwrap();
            let st = recorder.new_value(program).store(*value, dst);
            recorder.func().push_inst(program, st);
        }

        // allocate the return value
        if !matches!(self.ret_ty, Type::Void) {
            let ret_val = alloc(
                recorder,
                program,
                IrType::get_i32(),
                Some("%ret".to_owned()),
            );
            recorder.func_mut().set_ret_val(ret_val);
        }

        // enter the main body block
        let main_body = recorder.func().new_anonymous_bb(program);
        recorder.func_mut().push_bb(program, main_body);
        // generate IR for the main body block
        self.block.generate_ir(program, recorder)?;

        // finishing off the function
        let entry = recorder.func().entry_bb();
        let jump = recorder.new_value(program).jump(main_body);
        recorder.func().push_inst_to(program, entry, jump);

        let end_bb = recorder.func().end_bb();
        let jump = recorder.new_value(program).jump(end_bb);
        recorder.func().push_inst(program, jump);

        // enter the end block
        recorder.func_mut().push_bb(program, end_bb);

        // load the return value and return
        if matches!(self.ret_ty, Type::Void) {
            let ret = recorder.new_value(program).ret(None);
            recorder.func().push_inst(program, ret);
        } else {
            let ret_val = recorder.func().ret_val().unwrap();
            let ld = recorder.new_value(program).load(ret_val);
            let ret = recorder.new_value(program).ret(Some(ld));
            recorder.func().push_inst(program, ld);
            recorder.func().push_inst(program, ret);
        }
        recorder.exit_scope();

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
        self.items.iter().try_for_each(|item| match item {
            BlockItem::Decl(decl) => decl.generate_ir(program, recorder),
            BlockItem::Stmt(stmt) => stmt.generate_ir(program, recorder),
        })?;
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
            Decl::ConstDecl(decls) => decls
                .iter()
                .try_for_each(|d| d.generate_ir(program, recorder)),
            Decl::VarDecl(decls) => decls
                .iter()
                .try_for_each(|d| d.generate_ir(program, recorder)),
        }
    }
}

impl<'i> GenerateIR<'i> for VarDecl {
    type Out = ();

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        if recorder.is_global() {
            let init = if let Some(exp) = &self.init {
                let val = exp.const_eval(recorder).unwrap();
                program.new_value().integer(val)
            } else {
                program.new_value().zero_init(IrType::get_i32())
            };
            let var = program.new_value().global_alloc(init);
            program.set_value_name(var, Some(format!("@{}", self.name)));
            recorder.insert_var(&self.name, var, true)?;

            Ok(())
        } else {
            let var = alloc(
                recorder,
                program,
                IrType::get_i32(),
                Some(format!("@{}", &self.name)),
            );
            if let Some(exp) = &self.init {
                let init_val = exp.generate_ir(program, recorder)?;
                let init = recorder.new_value(program).store(init_val, var);
                recorder.func().push_inst(program, init);
                recorder.insert_var(&self.name, var, true)
            } else {
                recorder.insert_var(&self.name, var, false)
            }
        }
    }
}

impl<'i> GenerateIR<'i> for ConstDecl {
    type Out = ();

    fn generate_ir(
        &'i self,
        _program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        recorder.insert_const_var(&self.name, self.init.const_eval(recorder).unwrap())
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
        let true_bb = recorder.func().new_bb(program, "%br_then");
        let false_bb = recorder.func().new_bb(program, "%br_else");
        let end_bb = recorder.func().new_bb(program, "%br_end");

        let result = self.cond.generate_ir(program, recorder)?;
        let br = recorder
            .new_value(program)
            .branch(result, true_bb, false_bb);
        recorder.func().push_inst(program, br);

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
        let result = self.cond.generate_ir(program, recorder)?;
        let br = recorder
            .new_value(program)
            .branch(result, loop_body, loop_exit);
        recorder.func().push_inst(program, br);

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
            Exp::LVal(name) => match recorder.get_symbol(name).unwrap() {
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
        if let Some(result) = self.const_eval(recorder) {
            return Ok(recorder.new_value(program).integer(result));
        }

        match self.op {
            BinaryOp::And | BinaryOp::Or => {
                let name = if matches!(self.op, BinaryOp::And) {
                    "%land_end"
                } else {
                    "%lor_end"
                };
                let end_bb = recorder.func().new_bb(program, name);
                let result = short_circuit(program, recorder, self, end_bb)?;
                recorder.func_mut().push_bb(program, end_bb);
                let ld = recorder.new_value(program).load(result);
                recorder.func().push_inst(program, ld);

                Ok(ld)
            }
            _ => {
                let lhs = self.lhs.generate_ir(program, recorder)?;
                let rhs = self.rhs.generate_ir(program, recorder)?;

                Ok(binary(program, recorder, self.op.into(), lhs, rhs))
            }
        }
    }
}

impl<'i> GenerateIR<'i> for UnaryExp {
    type Out = Value;

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        if let Some(result) = self.const_eval(recorder) {
            return Ok(recorder.new_value(program).integer(result));
        }

        match self {
            UnaryExp::Unary(op, exp) => {
                let opr = exp.generate_ir(program, recorder)?;
                let val = match op {
                    UnaryOp::Nop => opr,
                    UnaryOp::Neg => negative(program, recorder, opr),
                    UnaryOp::Not => logical_not(program, recorder, opr),
                };
                Ok(val)
            }
            UnaryExp::Call(call) => call.generate_ir(program, recorder),
        }
    }
}

impl<'i> GenerateIR<'i> for Call {
    type Out = Value;

    fn generate_ir(
        &'i self,
        program: &mut Program,
        recorder: &mut ProgramRecorder<'i>,
    ) -> Result<Self::Out> {
        let mut arg_values = Vec::new();
        for arg in &self.args {
            let val = arg.generate_ir(program, recorder)?;
            arg_values.push(val);
        }
        let func_id = *recorder.get_func(&self.func_id).unwrap();
        let call = recorder.new_value(program).call(func_id, arg_values);
        recorder.func().push_inst(program, call);

        Ok(call)
    }
}

fn short_circuit<'i>(
    program: &mut Program,
    recorder: &mut ProgramRecorder<'i>,
    cond: &'i BinaryExp,
    end_bb: BasicBlock,
) -> Result<Value> {
    let result = alloc(recorder, program, IrType::get_i32(), None);

    match cond.op {
        BinaryOp::And => {
            let check_rhs = recorder.func().new_bb(program, "%land_rhs");
            let lhs = cond.lhs.generate_ir(program, recorder)?;
            let zero = recorder.new_value(program).integer(0);
            let lhs_checked = recorder
                .new_value(program)
                .binary(IrBinaryOp::NotEq, lhs, zero);
            let st = recorder.new_value(program).store(lhs_checked, result);
            let br = recorder
                .new_value(program)
                .branch(lhs_checked, check_rhs, end_bb);
            recorder.func().push_inst(program, lhs_checked);
            recorder.func().push_inst(program, st);
            recorder.func().push_inst(program, br);

            recorder.func_mut().push_bb(program, check_rhs);
            let rhs = cond.rhs.generate_ir(program, recorder)?;
            let zero = recorder.new_value(program).integer(0);
            let rhs_checked = recorder
                .new_value(program)
                .binary(IrBinaryOp::NotEq, rhs, zero);
            let st = recorder.new_value(program).store(rhs_checked, result);
            let jump = recorder.new_value(program).jump(end_bb);
            recorder.func().push_inst(program, rhs_checked);
            recorder.func().push_inst(program, st);
            recorder.func().push_inst(program, jump);
        }

        BinaryOp::Or => {
            let check_rhs = recorder.func().new_bb(program, "%lor_rhs");
            let lhs = cond.lhs.generate_ir(program, recorder)?;
            let zero = recorder.new_value(program).integer(0);
            let lhs_checked = recorder
                .new_value(program)
                .binary(IrBinaryOp::NotEq, lhs, zero);
            let st = recorder.new_value(program).store(lhs_checked, result);
            let br = recorder
                .new_value(program)
                .branch(lhs_checked, end_bb, check_rhs);
            recorder.func().push_inst(program, lhs_checked);
            recorder.func().push_inst(program, st);
            recorder.func().push_inst(program, br);

            recorder.func_mut().push_bb(program, check_rhs);
            let rhs = cond.rhs.generate_ir(program, recorder)?;
            let zero = recorder.new_value(program).integer(0);
            let rhs_checked = recorder
                .new_value(program)
                .binary(IrBinaryOp::NotEq, rhs, zero);
            let st = recorder.new_value(program).store(rhs_checked, result);
            let jump = recorder.new_value(program).jump(end_bb);
            recorder.func().push_inst(program, rhs_checked);
            recorder.func().push_inst(program, st);
            recorder.func().push_inst(program, jump);
        }
        _ => unreachable!(),
    }

    Ok(result)
}

impl From<BinaryOp> for IrBinaryOp {
    fn from(value: BinaryOp) -> Self {
        match value {
            BinaryOp::Add => IrBinaryOp::Add,
            BinaryOp::Sub => IrBinaryOp::Sub,
            BinaryOp::Mul => IrBinaryOp::Mul,
            BinaryOp::Div => IrBinaryOp::Div,
            BinaryOp::Mod => IrBinaryOp::Mod,
            BinaryOp::Eq => IrBinaryOp::Eq,
            BinaryOp::Neq => IrBinaryOp::NotEq,
            BinaryOp::Lt => IrBinaryOp::Lt,
            BinaryOp::Le => IrBinaryOp::Le,
            BinaryOp::Gt => IrBinaryOp::Gt,
            BinaryOp::Ge => IrBinaryOp::Ge,
            _ => unreachable!(),
        }
    }
}
