use crate::{ast::*, sema::ty::TypeKind};
use koopa::ir::builder_traits::{GlobalInstBuilder, LocalInstBuilder, ValueBuilder};

use super::*;

pub trait GenerateIR<'i> {
    type Out;

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out>;
}

impl<'i> GenerateIR<'i> for CompUnit {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        recorder.install_lib();

        self.items
            .iter()
            .try_for_each(|item| item.generate_ir(recorder))
    }
}

impl<'i> GenerateIR<'i> for GlobalItem {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        match self {
            GlobalItem::Decl(i) => i.generate_ir(recorder),
            GlobalItem::Func(i) => i.generate_ir(recorder),
        }
    }
}

impl<'i> GenerateIR<'i> for FuncDef {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        // generate the function and its entry & end blocks
        recorder.enter_func(self);

        // enter the entry block
        let entry_bb = recorder.func().get_entry_bb();
        recorder.push_bb(entry_bb);

        let param_values: Vec<Value> = recorder.get_func_data().params().to_vec();
        let ty = recorder.get_ty(&self.ident).clone();
        let (ret_ty, param_tys) = if let TypeKind::Func(ret_ty, param_tys) = ty.kind() {
            (ret_ty, param_tys)
        } else {
            unreachable!()
        };

        (0..param_values.len()).for_each(|i| {
            let ident = &self.params[i].ident;
            let ty = param_tys[i].get_ir_ty();
            let value = param_values[i];
            let alloc = local_alloc(recorder, ty, Some(format!("%{}", ident)));
            let store = recorder.new_value().store(value, alloc);
            recorder.push_inst(store);
            recorder.insert_value(ident, alloc);
        });

        // allocate the return value
        if !matches!(self.ret_kind, ExprKind::Void) {
            let ret_val = local_alloc(recorder, ret_ty.get_ir_ty(), Some("%ret".to_owned()));
            recorder.func_mut().set_ret_val(ret_val);
        }

        // enter the main body block
        let main_body = recorder.new_anonymous_bb();
        recorder.push_bb(main_body);
        // generate IR for the main body block
        self.block.generate_ir(recorder)?;

        // finishing off the function
        let entry = recorder.func().get_entry_bb();
        let jump = recorder.new_value().jump(main_body);
        recorder.push_inst_to(entry, jump);

        let end_bb = recorder.func().get_end_bb();
        let jump = recorder.new_value().jump(end_bb);
        recorder.push_inst(jump);

        // enter the end block
        recorder.push_bb(end_bb);

        // load the return value and return
        if matches!(self.ret_kind, ExprKind::Void) {
            let ret = recorder.new_value().ret(None);
            recorder.push_inst(ret);
        } else {
            let ret_val = recorder.func().get_ret_val().unwrap();
            let ld = recorder.new_value().load(ret_val);
            let ret = recorder.new_value().ret(Some(ld));
            recorder.push_inst(ld);
            recorder.push_inst(ret);
        }
        recorder.exit_func();

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Block {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        self.items.iter().try_for_each(|item| match item {
            BlockItem::Decl(decl) => decl.generate_ir(recorder),
            BlockItem::Stmt(stmt) => stmt.generate_ir(recorder),
        })?;

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Decl {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        match self {
            Decl::ConstDecl(decls) => decls.iter().try_for_each(|d| d.generate_ir(recorder)),
            Decl::VarDecl(decls) => decls.iter().try_for_each(|d| d.generate_ir(recorder)),
        }
    }
}

impl<'i> GenerateIR<'i> for VarDecl {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        let id = &self.lval.ident;
        let ty = recorder.get_ty(id).clone();

        if recorder.is_global() {
            let init_val = match &ty.kind() {
                TypeKind::Integer => match &self.init {
                    Some(InitVal::Expr(e)) => recorder.new_global_value().integer(e.get_i32()),
                    None => recorder.new_global_value().zero_init(IrType::get_i32()),
                    _ => unreachable!(),
                },
                TypeKind::Array(_, _) => match &self.init {
                    Some(init) => {
                        let elems = eval_array(init, &ty);
                        init_global_array(recorder, &ty, &elems)
                    }
                    None => recorder.new_global_value().zero_init(ty.get_ir_ty()),
                },
                _ => unreachable!(),
            };
            let alloc = recorder.new_global_value().global_alloc(init_val);
            recorder.set_global_value_name(format!("@{}", &id), alloc);
            recorder.insert_value(id, alloc);
        } else {
            let val = local_alloc(recorder, ty.get_ir_ty(), Some(format!("@{}", &id)));
            recorder.insert_value(id, val);

            match &ty.kind() {
                TypeKind::Integer => match &self.init {
                    Some(InitVal::Expr(e)) => {
                        let init_val = e.generate_ir(recorder)?;
                        let store = recorder.new_value().store(init_val, val);
                        recorder.push_inst(store);
                    }
                    None => {}
                    _ => unreachable!(),
                },
                TypeKind::Array(_, _) => match &self.init {
                    Some(init) => {
                        let elems = eval_array(init, &ty);
                        init_array(recorder, val, &ty, &elems);
                    }
                    None => {}
                },
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for ConstDecl {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        if matches!(self.kind, ExprKind::Int) {
            return Ok(());
        }

        let id = &self.lval.ident;
        let ty = recorder.get_ty(id).clone();

        if recorder.is_global() {
            let init_val = if matches!(ty.kind(), TypeKind::Array(_, _)) {
                let elems = eval_array(&self.init, &ty);
                init_global_array(recorder, &ty, &elems)
            } else {
                unreachable!()
            };

            let alloc = recorder.new_global_value().global_alloc(init_val);
            recorder.set_global_value_name(format!("@{}", &id), alloc);
            recorder.insert_value(id, alloc);
        } else {
            let val = local_alloc(recorder, ty.get_ir_ty(), Some(format!("@{}", &id)));
            recorder.insert_value(id, val);

            if matches!(ty.kind(), TypeKind::Array(_, _)) {
                let elems = eval_array(&self.init, &ty);
                init_array(recorder, val, &ty, &elems);
            } else {
                unreachable!()
            }
        }

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Stmt {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        match self {
            Self::Assign(s) => s.generate_ir(recorder),
            Self::Block(s) => s.generate_ir(recorder),
            Self::Expr(s) => s
                .as_ref()
                .map_or(Ok(()), |exp| exp.generate_ir(recorder).map(|_| ())),
            Self::Return(s) => s.generate_ir(recorder),
            Self::Branch(s) => s.generate_ir(recorder),
            Self::While(s) => s.generate_ir(recorder),
            Self::Break(s) => s.generate_ir(recorder),
            Self::Continue(s) => s.generate_ir(recorder),
        }
    }
}

impl<'i> GenerateIR<'i> for Assign {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        let dst = get_lval_ptr(recorder, &self.lval);
        let value = self.val.generate_ir(recorder)?;
        let store = recorder.new_value().store(value, dst);
        recorder.push_inst(store);

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Branch {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        let true_bb = recorder.new_anonymous_bb();
        let false_bb = recorder.new_anonymous_bb();
        let end_bb = recorder.new_anonymous_bb();

        let result = self.cond.generate_ir(recorder)?;
        let br = recorder.new_value().branch(result, true_bb, false_bb);
        recorder.push_inst(br);

        // enter the "true" block
        recorder.push_bb(true_bb);
        self.if_stmt.generate_ir(recorder)?;

        // jump to the if-end block
        let jump = recorder.new_value().jump(end_bb);
        recorder.push_inst(jump);

        // enter the "false" block
        recorder.push_bb(false_bb);
        if let Some(el_stmt) = &self.el_stmt {
            el_stmt.generate_ir(recorder)?;
        }
        // jump to the if-end block
        let jump = recorder.new_value().jump(end_bb);
        recorder.push_inst(jump);

        // enter the if-end block
        recorder.push_bb(end_bb);

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for While {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        let loop_entry = recorder.new_anonymous_bb();
        let loop_body = recorder.new_anonymous_bb();
        let loop_exit = recorder.new_anonymous_bb();

        // record the loop information
        recorder.enter_loop(loop_entry, loop_exit);

        // jump to the loop entry
        let jump = recorder.new_value().jump(loop_entry);
        recorder.push_inst(jump);

        // check the loop condition
        recorder.push_bb(loop_entry);
        let result = self.cond.generate_ir(recorder)?;
        let br = recorder.new_value().branch(result, loop_body, loop_exit);
        recorder.push_inst(br);

        // enter the loop body block
        recorder.push_bb(loop_body);
        self.stmt.generate_ir(recorder)?;

        // jump back to the loop entry
        let jump = recorder.new_value().jump(loop_entry);
        recorder.push_inst(jump);

        // enter the exit of loop
        recorder.push_bb(loop_exit);
        recorder.exit_loop();

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Break {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        if !recorder.inside_loop() {
            bail!("break outside of loop");
        }

        let loop_exit = recorder.loop_exit();
        let jump = recorder.new_value().jump(loop_exit);
        recorder.push_inst(jump);

        let next_bb = recorder.new_anonymous_bb();
        recorder.push_bb(next_bb);

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Continue {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        if !recorder.inside_loop() {
            bail!("continue outside of loop");
        }

        // instantly jump to the loop entry
        let loop_entry = recorder.loop_entry();
        let jump = recorder.new_value().jump(loop_entry);
        recorder.push_inst(jump);

        // enter the next block (unreachable)
        let next_bb = recorder.new_anonymous_bb();
        recorder.push_bb(next_bb);

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Return {
    type Out = ();

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        if let Some(ret_val) = &self.ret_val {
            let ret_val = ret_val.generate_ir(recorder)?;
            let dst = recorder.func().get_ret_val().unwrap();
            let st = recorder.new_value().store(ret_val, dst);
            recorder.push_inst(st);
        }
        let end_bb = recorder.func().get_end_bb();
        let jump = recorder.new_value().jump(end_bb);
        recorder.push_inst(jump);

        let next_bb = recorder.new_anonymous_bb();
        recorder.push_bb(next_bb);

        Ok(())
    }
}

impl<'i> GenerateIR<'i> for Expr {
    type Out = Value;

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        Ok(match self {
            Expr::Integer(i) => recorder.new_value().integer(*i),
            Expr::UnaryExpr(uxp) => uxp.generate_ir(recorder)?,
            Expr::BinaryExpr(bxp) => bxp.generate_ir(recorder)?,
            Expr::LVal(lval) => load_lval(recorder, lval),
            Expr::Error => panic!("expected an expression"),
        })
    }
}

impl<'i> GenerateIR<'i> for BinaryExpr {
    type Out = Value;

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        Ok(match self.op {
            BinaryOp::And | BinaryOp::Or => {
                let end_bb = recorder.new_anonymous_bb();
                let result = short_circuit(recorder, self, end_bb)?;
                recorder.push_bb(end_bb);
                let ld = recorder.new_value().load(result);
                recorder.push_inst(ld);

                ld
            }
            _ => {
                let lhs = self.lhs.generate_ir(recorder)?;
                let rhs = self.rhs.generate_ir(recorder)?;

                binary(recorder, self.op.into(), lhs, rhs)
            }
        })
    }
}

impl<'i> GenerateIR<'i> for UnaryExpr {
    type Out = Value;

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        match self {
            Self::Unary(op, exp) => {
                let opr = exp.generate_ir(recorder)?;
                let val = match op {
                    UnaryOp::Nop => opr,
                    UnaryOp::Neg => negative(recorder, opr),
                    UnaryOp::Not => logical_not(recorder, opr),
                };
                Ok(val)
            }
            Self::Call(call) => call.generate_ir(recorder),
        }
    }
}

impl<'i> GenerateIR<'i> for Call {
    type Out = Value;

    fn generate_ir(&'i self, recorder: &mut ProgramRecorder<'i>) -> Result<Self::Out> {
        let func = recorder.get_func_id(&self.ident);
        let ty = recorder.get_ty(&self.ident).clone();
        let param_tys = if let TypeKind::Func(_, param_tys) = ty.kind() {
            param_tys
        } else {
            unreachable!()
        };

        let arg_values: Vec<_> = self
            .args
            .iter()
            .zip(param_tys.iter())
            .map(|(arg, ty)| match (ty.kind(), arg) {
                (TypeKind::Pointer(_), Expr::LVal(lval)) => {
                    let mut val = get_lval_ptr(recorder, lval);
                    if matches!(recorder.get_ty(&lval.ident).kind(), TypeKind::Array(_, _))
                        || !lval.dims.is_empty()
                    {
                        // convert an array to the pointer of its first elements
                        val = into_ptr(recorder, val);
                    }
                    val
                }
                _ => arg.generate_ir(recorder).unwrap(),
            })
            .collect();
        let call = recorder.new_value().call(func, arg_values);
        recorder.push_inst(call);

        Ok(call)
    }
}

fn short_circuit<'i>(
    recorder: &mut ProgramRecorder<'i>,
    cond: &'i BinaryExpr,
    end_bb: BasicBlock,
) -> Result<Value> {
    let result = local_alloc(recorder, IrType::get_i32(), None);

    match cond.op {
        BinaryOp::And => {
            let check_rhs = recorder.new_anonymous_bb();
            let lhs = cond.lhs.generate_ir(recorder)?;
            let zero = recorder.new_value().integer(0);
            let lhs_checked = recorder.new_value().binary(IrBinaryOp::NotEq, lhs, zero);
            let st = recorder.new_value().store(lhs_checked, result);
            let br = recorder.new_value().branch(lhs_checked, check_rhs, end_bb);
            recorder.push_inst(lhs_checked);
            recorder.push_inst(st);
            recorder.push_inst(br);

            recorder.push_bb(check_rhs);
            let rhs = cond.rhs.generate_ir(recorder)?;
            let zero = recorder.new_value().integer(0);
            let rhs_checked = recorder.new_value().binary(IrBinaryOp::NotEq, rhs, zero);
            let st = recorder.new_value().store(rhs_checked, result);
            let jump = recorder.new_value().jump(end_bb);
            recorder.push_inst(rhs_checked);
            recorder.push_inst(st);
            recorder.push_inst(jump);
        }

        BinaryOp::Or => {
            let check_rhs = recorder.new_anonymous_bb();
            let lhs = cond.lhs.generate_ir(recorder)?;
            let zero = recorder.new_value().integer(0);
            let lhs_checked = recorder.new_value().binary(IrBinaryOp::NotEq, lhs, zero);
            let st = recorder.new_value().store(lhs_checked, result);
            let br = recorder.new_value().branch(lhs_checked, end_bb, check_rhs);
            recorder.push_inst(lhs_checked);
            recorder.push_inst(st);
            recorder.push_inst(br);

            recorder.push_bb(check_rhs);
            let rhs = cond.rhs.generate_ir(recorder)?;
            let zero = recorder.new_value().integer(0);
            let rhs_checked = recorder.new_value().binary(IrBinaryOp::NotEq, rhs, zero);
            let st = recorder.new_value().store(rhs_checked, result);
            let jump = recorder.new_value().jump(end_bb);
            recorder.push_inst(rhs_checked);
            recorder.push_inst(st);
            recorder.push_inst(jump);
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
