use std::collections::HashMap;

use crate::ast::*;

pub trait ConstEval {
    fn const_eval(&self, eval: &Evaluator) -> Option<i32>;
}

impl ConstEval for UnaryExpr {
    fn const_eval(&self, eval: &Evaluator) -> Option<i32> {
        match self {
            Self::Unary(op, exp) => exp.const_eval(eval).map(|opr| eval_unary(*op, opr)),
            Self::Call(_) => None,
        }
    }
}

impl ConstEval for BinaryExpr {
    fn const_eval(&self, eval: &Evaluator) -> Option<i32> {
        let lhs = self.lhs.const_eval(eval);
        let rhs = self.rhs.const_eval(eval);
        if let (Some(lhs), Some(rhs)) = (lhs, rhs) {
            return Some(eval_binary(self.op, lhs, rhs));
        }
        if matches!(self.op, BinaryOp::And) && (matches!(lhs, Some(0)) || matches!(rhs, Some(0))) {
            return Some(0);
        }
        if matches!(self.op, BinaryOp::Or)
            && (matches!(lhs, Some(i) if i != 0) || matches!(rhs, Some(i) if i !=0))
        {
            return Some(1);
        }
        if matches!((self.lhs.as_ref(), self.rhs.as_ref()), (Expr::LVal(l), Expr::LVal(r)) if l.ident == r.ident)
        {
            match self.op {
                BinaryOp::Eq | BinaryOp::Le | BinaryOp::Ge => Some(1),
                BinaryOp::Neq | BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Sub => Some(0),
                _ => None,
            }
        } else {
            None
        }
    }
}

impl ConstEval for Expr {
    fn const_eval(&self, eval: &Evaluator) -> Option<i32> {
        match self {
            Self::Integer(i) => Some(*i),
            Self::UnaryExpr(uxp) => uxp.const_eval(eval),
            Self::BinaryExpr(bxp) => bxp.const_eval(eval),
            Self::LVal(lval) => eval.get(lval.ident.as_str()).copied(),
            Self::Error => panic!("expected an expression"),
        }
    }
}

impl Expr {
    pub fn get_i32(&self) -> i32 {
        if let Self::Integer(i) = self {
            *i
        } else {
            panic!("attempt to retrieve an integer from a non-const expression")
        }
    }
}

pub type Evaluator<'ast> = HashMap<&'ast str, i32>;

impl<'ast> MutVisitor<'ast> for Evaluator<'ast> {
    fn visit_const_decl(&mut self, c: &'ast mut ConstDecl) {
        if matches!(c.kind, ExprKind::Int) {
            if let InitVal::Expr(e) = &c.init {
                self.insert(&c.lval.ident, e.const_eval(self).unwrap());
            } else {
                panic!("invalid initializer");
            }
        } else {
            walk_const_decl(self, c);
        }
    }

    fn visit_assign(&mut self, a: &'ast mut Assign) {
        if self.contains_key(a.lval.ident.as_str()) {
            panic!("attempt to assign a const value");
        }
        walk_assign(self, a);
    }

    fn visit_expr(&mut self, e: &'ast mut Expr) {
        if matches!(e, Expr::Integer(_)) {
            return;
        }
        if let Some(i) = e.const_eval(self) {
            *e = Expr::Integer(i);
            return;
        }

        walk_expr(self, e);
    }
}
