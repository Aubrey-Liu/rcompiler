use koopa::ir::BinaryOp as IR_BinaryOp;
use koopa::ir::{FunctionData, Value, ValueKind};

use super::*;
use crate::ast::*;

pub trait GenerateValue {
    fn generate(
        &self,
        symt: &SymbolTable,
        func: &mut FunctionData,
        insts: &mut Vec<Value>,
    ) -> Value;
}

impl GenerateValue for UnaryExp {
    fn generate(
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

impl GenerateValue for BinaryExp {
    fn generate(
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

impl GenerateValue for Exp {
    fn generate(
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
                            "uninitialized variable {} can't be used in an expression",
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
