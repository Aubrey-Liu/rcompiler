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

        /*        self.declare_func(program, "getint", vec![], IrType::get_i32());
        self.declare_func(program, "getch", vec![], IrType::get_i32());
        self.declare_func(
            program,
            "getarray",
            vec![IrType::get_pointer(IrType::get_i32())],
            IrType::get_i32(),
        );
        self.declare_func(
            program,
            "putint",
            vec![IrType::get_i32()],
            IrType::get_unit(),
        );
        self.declare_func(
            program,
            "putch",
            vec![IrType::get_i32()],
            IrType::get_unit(),
        );
        self.declare_func(
            program,
            "putarray",
            vec![IrType::get_i32(), IrType::get_pointer(IrType::get_i32())],
            IrType::get_unit(),
        );
        self.declare_func(program, "starttime", vec![], IrType::get_unit());
        self.declare_func(program, "stoptime", vec![], IrType::get_unit()); */
        symbols.insert("getint", Symbol::Func(Type::Int, vec![]));
        symbols.insert("getch", Symbol::Func(Type::Int, vec![]));
        symbols.insert(
            "getarray",
            Symbol::Func(Type::Int, vec![Type::Pointer(Box::new(Type::Int))]),
        );
        symbols.insert("putint", Symbol::Func(Type::Void, vec![Type::Int]));
        symbols.insert("putch", Symbol::Func(Type::Void, vec![Type::Int]));
        symbols.insert(
            "putarray",
            Symbol::Func(
                Type::Void,
                vec![Type::Int, Type::Pointer(Box::new(Type::Int))],
            ),
        );
        symbols.insert("starttime", Symbol::Func(Type::Void, vec![]));
        symbols.insert("stoptime", Symbol::Func(Type::Void, vec![]));

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
        let ret_ty = match &self.ret_ty {
            AstType::Int => Type::Int,
            AstType::Void => Type::Void,
            _ => unreachable!(),
        };
        let param_tys: Vec<_> = self
            .params
            .iter_mut()
            .map(|p| p.analyze(symbols).unwrap())
            .collect();
        symbols.insert(&self.ident, Symbol::Func(ret_ty, param_tys));

        self.block.analyze(symbols)
    }
}

impl Analyzer for FuncParam {
    type Out = Type;

    fn analyze(&mut self, symbols: &mut SymbolTable) -> Result<Self::Out> {
        let ty = match &self.ty {
            AstType::Int => Type::Int,
            AstType::Array => Type::Pointer(Box::new(Type::infer_from_dims(symbols, &self.dims))),
            _ => unreachable!(),
        };

        let symbol = match &ty {
            Type::Int => Symbol::Var(true),
            Type::Pointer(_) => Symbol::Pointer(ty.clone()),
            _ => unreachable!(),
        };
        symbols.insert(&self.ident, symbol);

        Ok(ty)
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
                    Symbol::Array(_, _) | Symbol::ConstArray(_, _) => e.analyze(symbols)?,
                    Symbol::Var(_) | Symbol::Pointer(_) => {}
                    _ => unreachable!(),
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
