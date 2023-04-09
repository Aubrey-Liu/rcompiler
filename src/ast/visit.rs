use super::*;

pub trait MutVisitor<'ast>: Sized {
    fn visit_comp_unit(&mut self, c: &'ast mut CompUnit) {
        walk_comp_unit(self, c);
    }

    fn visit_global_item(&mut self, g: &'ast mut GlobalItem) {
        walk_global_item(self, g);
    }

    fn visit_decl(&mut self, d: &'ast mut Decl) {
        walk_decl(self, d);
    }

    fn visit_var_decl(&mut self, v: &'ast mut VarDecl) {
        walk_var_decl(self, v);
    }

    fn visit_const_decl(&mut self, c: &'ast mut ConstDecl) {
        walk_const_decl(self, c);
    }

    fn visit_func_def(&mut self, f: &'ast mut FuncDef) {
        walk_func_def(self, f);
    }

    fn visit_func_param(&mut self, f: &'ast mut FuncParam) {
        walk_func_param(self, f);
    }

    fn visit_block(&mut self, b: &'ast mut Block) {
        walk_block(self, b);
    }

    fn visit_block_item(&mut self, b: &'ast mut BlockItem) {
        walk_block_item(self, b);
    }

    fn visit_stmt(&mut self, s: &'ast mut Stmt) {
        walk_stmt(self, s);
    }

    fn visit_assign(&mut self, a: &'ast mut Assign) {
        walk_assign(self, a);
    }

    fn visit_branch(&mut self, b: &'ast mut Branch) {
        walk_branch(self, b);
    }

    fn visit_break(&mut self, _b: &'ast mut Break) {}

    fn visit_continue(&mut self, _c: &'ast mut Continue) {}

    fn visit_while(&mut self, w: &'ast mut While) {
        walk_while(self, w);
    }

    fn visit_return(&mut self, r: &'ast mut Return) {
        walk_return(self, r);
    }

    fn visit_expr(&mut self, e: &'ast mut Expr) {
        walk_expr(self, e);
    }

    fn visit_binary_expr(&mut self, b: &'ast mut BinaryExpr) {
        walk_binary_expr(self, b);
    }

    fn visit_unary_expr(&mut self, u: &'ast mut UnaryExpr) {
        walk_unary_expr(self, u);
    }

    fn visit_call(&mut self, c: &'ast mut Call) {
        walk_call(self, c);
    }

    fn visit_lval(&mut self, l: &'ast mut LVal) {
        walk_lval(self, l);
    }

    fn visit_initval(&mut self, e: &'ast mut InitVal) {
        walk_initval(self, e);
    }
}

macro_rules! walk_list {
    ($visitor: expr, $method: ident, $list: expr $(, $($extra_args: expr),* )?) => {
        {
            #[allow(for_loops_over_fallibles)]
            for elem in $list {
                $visitor.$method(elem $(, $($extra_args,)* )?)
            }
        }
    }
}

pub fn walk_comp_unit<'a, V: MutVisitor<'a>>(visitor: &mut V, comp_unit: &'a mut CompUnit) {
    walk_list!(visitor, visit_global_item, &mut comp_unit.items)
}

pub fn walk_global_item<'a, V: MutVisitor<'a>>(visitor: &mut V, global_item: &'a mut GlobalItem) {
    match global_item {
        GlobalItem::Decl(decl) => visitor.visit_decl(decl),
        GlobalItem::Func(func) => visitor.visit_func_def(func),
    }
}

pub fn walk_decl<'a, V: MutVisitor<'a>>(visitor: &mut V, decl: &'a mut Decl) {
    match decl {
        Decl::ConstDecl(const_decl) => walk_list!(visitor, visit_const_decl, const_decl),
        Decl::VarDecl(var_decl) => walk_list!(visitor, visit_var_decl, var_decl),
    }
}

pub fn walk_var_decl<'a, V: MutVisitor<'a>>(visitor: &mut V, var_decl: &'a mut VarDecl) {
    if let Some(initval) = &mut var_decl.init {
        visitor.visit_initval(initval);
    }
    visitor.visit_lval(&mut var_decl.lval);
}

pub fn walk_const_decl<'a, V: MutVisitor<'a>>(visitor: &mut V, const_decl: &'a mut ConstDecl) {
    visitor.visit_initval(&mut const_decl.init);
    visitor.visit_lval(&mut const_decl.lval);
}

pub fn walk_func_def<'a, V: MutVisitor<'a>>(visitor: &mut V, func_def: &'a mut FuncDef) {
    walk_list!(visitor, visit_func_param, &mut func_def.params);
    visitor.visit_block(&mut func_def.block);
}

pub fn walk_func_param<'a, V: MutVisitor<'a>>(visitor: &mut V, func_param: &'a mut FuncParam) {
    walk_list!(visitor, visit_expr, &mut func_param.dims);
}

pub fn walk_block<'a, V: MutVisitor<'a>>(visitor: &mut V, block: &'a mut Block) {
    walk_list!(visitor, visit_block_item, &mut block.items);
}

pub fn walk_block_item<'a, V: MutVisitor<'a>>(visitor: &mut V, block_item: &'a mut BlockItem) {
    match block_item {
        BlockItem::Decl(decl) => visitor.visit_decl(decl),
        BlockItem::Stmt(stmt) => visitor.visit_stmt(stmt),
    }
}

pub fn walk_stmt<'a, V: MutVisitor<'a>>(visitor: &mut V, stmt: &'a mut Stmt) {
    match stmt {
        Stmt::Assign(assign) => visitor.visit_assign(assign),
        Stmt::Block(block) => visitor.visit_block(block),
        Stmt::Branch(br) => visitor.visit_branch(br),
        Stmt::Expr(e) => {
            if let Some(e) = e {
                visitor.visit_expr(e);
            }
        }
        Stmt::Return(ret) => visitor.visit_return(ret),
        Stmt::While(w) => visitor.visit_while(w),
        Stmt::Break(b) => visitor.visit_break(b),
        Stmt::Continue(c) => visitor.visit_continue(c),
    }
}

pub fn walk_assign<'a, V: MutVisitor<'a>>(visitor: &mut V, assign: &'a mut Assign) {
    visitor.visit_expr(&mut assign.val);
    visitor.visit_lval(&mut assign.lval);
}

pub fn walk_branch<'a, V: MutVisitor<'a>>(visitor: &mut V, branch: &'a mut Branch) {
    visitor.visit_expr(&mut branch.cond);
    visitor.visit_stmt(&mut branch.if_stmt);
    if let Some(el_stmt) = &mut branch.el_stmt {
        visitor.visit_stmt(el_stmt);
    }
}

pub fn walk_while<'a, V: MutVisitor<'a>>(visitor: &mut V, w: &'a mut While) {
    visitor.visit_expr(&mut w.cond);
    visitor.visit_stmt(&mut w.stmt);
}

pub fn walk_return<'a, V: MutVisitor<'a>>(visitor: &mut V, ret: &'a mut Return) {
    if let Some(e) = &mut ret.ret_val {
        visitor.visit_expr(e);
    }
}

pub fn walk_expr<'a, V: MutVisitor<'a>>(visitor: &mut V, exp: &'a mut Expr) {
    match exp {
        Expr::Binary(bxp) => visitor.visit_binary_expr(bxp),
        Expr::Unary(uxp) => visitor.visit_unary_expr(uxp),
        Expr::LVal(lval) => visitor.visit_lval(lval),
        Expr::Integer(_) => {}
        Expr::Error => panic!("expected an expression"),
    }
}

pub fn walk_binary_expr<'a, V: MutVisitor<'a>>(visitor: &mut V, bxp: &'a mut BinaryExpr) {
    visitor.visit_expr(&mut bxp.lhs);
    visitor.visit_expr(&mut bxp.rhs);
}

pub fn walk_unary_expr<'a, V: MutVisitor<'a>>(visitor: &mut V, uxp: &'a mut UnaryExpr) {
    match uxp {
        UnaryExpr::Unary(_, exp) => visitor.visit_expr(exp),
        UnaryExpr::Call(call) => visitor.visit_call(call),
    }
}

pub fn walk_call<'a, V: MutVisitor<'a>>(visitor: &mut V, call: &'a mut Call) {
    walk_list!(visitor, visit_expr, &mut call.args);
}

pub fn walk_lval<'a, V: MutVisitor<'a>>(visitor: &mut V, lval: &'a mut LVal) {
    walk_list!(visitor, visit_expr, &mut lval.dims);
}

pub fn walk_initval<'a, V: MutVisitor<'a>>(visitor: &mut V, initval: &'a mut InitVal) {
    match initval {
        InitVal::Expr(exp) => visitor.visit_expr(exp),
        InitVal::List(init_list) => walk_list!(visitor, visit_initval, init_list),
    }
}
