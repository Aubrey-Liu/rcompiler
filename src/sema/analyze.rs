#![allow(unused_variables)]

use super::*;
use crate::ast::*;

use anyhow::*;

pub trait Analyzer {
    type Out;

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out>;
}

impl Analyzer for CompUnit {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        self.items
            .iter_mut()
            .try_for_each(|item| item.analyze(symbols))?;

        if symbols.contains("main") {
            Ok(())
        } else {
            Err(anyhow!("main function is not defined"))
        }
    }
}

impl Analyzer for GlobalItem {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        match self {
            Self::Decl(i) => i.analyze(symbols),
            Self::Func(i) => i.analyze(symbols),
        }
    }
}

impl Analyzer for FuncDef {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        let symbol: Symbol = Symbol::from_func_def(self);
        symbols.insert(&self.ident, symbol);

        self.params
            .iter()
            .for_each(|p| symbols.insert(&p.ident, Symbol::Var(true)));

        self.block.analyze(symbols)
    }
}

impl Analyzer for Block {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        self.items
            .iter_mut()
            .try_for_each(|item| item.analyze(symbols))
    }
}

impl Analyzer for BlockItem {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        match self {
            Self::Decl(i) => i.analyze(symbols),
            Self::Stmt(i) => i.analyze(symbols),
        }
    }
}

impl Analyzer for Decl {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        match self {
            Self::ConstDecl(d) => d.iter_mut().try_for_each(|d| d.analyze(symbols)),
            Self::VarDecl(d) => d.iter_mut().try_for_each(|d| d.analyze(symbols)),
        }
    }
}

impl Analyzer for ConstDecl {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        self.init.analyze(symbols)?;

        let symbol = Symbol::from_const_decl(self, symbols);
        symbols.insert(&self.lval.ident, symbol);

        self.lval
            .dims
            .iter_mut()
            .try_for_each(|d| d.analyze(symbols))
    }
}

impl Analyzer for VarDecl {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        if let Some(e) = &mut self.init {
            e.analyze(symbols)?;
        }

        let symbol = Symbol::from_var_decl(self, symbols);
        symbols.insert(&self.lval.ident, symbol);

        self.lval
            .dims
            .iter_mut()
            .try_for_each(|d| d.analyze(symbols))
    }
}

impl Analyzer for InitVal {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        match self {
            Self::Exp(e) => e.analyze(symbols),
            Self::List(e) => e.iter_mut().try_for_each(|e| e.analyze(symbols)),
        }
    }
}

impl Analyzer for Stmt {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        match self {
            Self::Assign(s) => s.analyze(symbols),
            Self::Block(s) => s.analyze(symbols),
            Self::Branch(s) => s.analyze(symbols),
            Self::Exp(s) => s.as_mut().map_or(Ok(()), |s| s.analyze(symbols)),
            Self::Return(s) => s.analyze(symbols),
            Self::While(s) => s.analyze(symbols),
            _ => Ok(()),
        }
    }
}

impl Analyzer for Return {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        if let Some(e) = &mut self.ret_val {
            e.analyze(symbols)
        } else {
            Ok(())
        }
    }
}

impl Analyzer for While {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        self.cond.analyze(symbols)?;
        self.stmt.analyze(symbols)
    }
}

impl Analyzer for Branch {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        self.cond.analyze(symbols)?;
        self.if_stmt.analyze(symbols)?;
        if let Some(s) = &mut self.el_stmt {
            s.analyze(symbols)
        } else {
            Ok(())
        }
    }
}

impl Analyzer for Assign {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        symbols.assign(&self.lval.ident);
        self.lval.analyze(symbols)?;
        self.val.analyze(symbols)
    }
}

impl Analyzer for Exp {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        if let Some(r) = self.const_eval(symbols) {
            *self = Exp::Integer(r);
        }

        match self {
            Self::Bxp(e) => e.analyze(symbols),
            Self::Uxp(e) => e.analyze(symbols),
            Self::LVal(e) => {
                match symbols.get(&e.ident) {
                    Symbol::ConstVar(i) => *self = Exp::Integer(*i),
                    Symbol::Var(init) if !init => bail!("attempt to use an uninitialized variable"),
                    _ => e.analyze(symbols)?,
                }
                Ok(())
            }
            Self::Integer(_) => Ok(()),
            _ => unreachable!(),
        }
    }
}

impl Analyzer for BinaryExp {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        self.lhs.analyze(symbols)?;
        self.rhs.analyze(symbols)
    }
}

impl Analyzer for UnaryExp {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        match self {
            Self::Unary(_, e) => e.analyze(symbols),
            Self::Call(e) => e.analyze(symbols),
        }
    }
}

impl Analyzer for Call {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        self.args
            .iter_mut()
            .try_for_each(|arg| arg.analyze(symbols))
    }
}

impl Analyzer for LVal {
    type Out = ();

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        self.dims.iter_mut().try_for_each(|d| d.analyze(symbols))
    }
}
