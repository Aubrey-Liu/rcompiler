use super::*;
use lazy_static_include::lazy_static::lazy_static;

pub enum Directive {
    Data,
    Text,
    Word(i32),
    Zero(usize),
}

pub struct AsmProgram {
    pub values: Vec<AsmValue>,
}

lazy_static! {
    static ref T0: RegID = "t0".into_id();
    static ref SP: RegID = "sp".into_id();
}

impl AsmProgram {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn push(&mut self, val: AsmValue) {
        self.values.push(val)
    }

    pub fn directive(&mut self, d: Directive) {
        self.push(AsmValue::Directive(d));
    }

    pub fn mv(&mut self, dst: RegID, src: RegID) {
        if dst != src {
            self.unary(AsmUnaryOp::Move, dst, src);
        }
    }

    pub fn unary(&mut self, op: AsmUnaryOp, dst: RegID, opr: RegID) {
        self.push(AsmValue::Unary(op, dst, opr));
    }

    pub fn binary(&mut self, op: AsmBinaryOp, dst: RegID, lhs: RegID, rhs: RegID) {
        self.push(AsmValue::Binary(op, dst, lhs, rhs));
    }

    pub fn binary_with_imm(&mut self, op: AsmBinaryOp, dst: RegID, lhs: RegID, imm: i32) {
        self.push(AsmValue::BinaryImm(op, dst, lhs, imm));
    }

    pub fn load(&mut self, dst: RegID, src: RegID, offset: i32) {
        self.push(AsmValue::Load(dst, src, offset));
    }

    pub fn load_imm(&mut self, dst: RegID, imm: i32) {
        self.push(AsmValue::LoadImm(dst, imm));
    }

    pub fn load_address(&mut self, dst: RegID, lable: &str) {
        self.push(AsmValue::LoadAddress(dst, lable.to_owned()));
    }

    pub fn store(&mut self, src: RegID, dst: RegID, offset: i32) {
        self.push(AsmValue::Store(src, dst, offset));
    }

    pub fn local_symbol(&mut self, lable: &str) {
        self.push(AsmValue::LocalSymbol(lable.to_owned()));
    }

    pub fn global_symbol(&mut self, lable: &str) {
        self.push(AsmValue::GlobalSymbol(lable.to_owned()));
    }

    pub fn call(&mut self, lable: &str) {
        self.push(AsmValue::Call(lable.to_owned()));
    }

    pub fn branch(&mut self, cond: RegID, target: &str) {
        self.push(AsmValue::Branch(cond, target.to_owned()));
    }

    pub fn jump(&mut self, target: &str) {
        self.push(AsmValue::Jump(target.to_owned()));
    }

    pub fn ret(&mut self) {
        self.push(AsmValue::Return);
    }

    pub fn prologue(
        &mut self,
        func_name: &str,
        ctx: &Context,
        saved_regs: (i32, i32),
        is_leaf: bool,
    ) {
        self.directive(Directive::Text);
        self.global_symbol(func_name);

        let ss = ctx.cur_func().ss();
        self.binary_with_imm(AsmBinaryOp::Add, *SP, *SP, -ss);
        if !is_leaf {
            self.store("ra".into_id(), *SP, ss - 4);
        }

        let mut off: i32 = saved_regs.0;
        let mut id = 0;
        while off < saved_regs.1 {
            self.store(format!("s{}", id).into_id(), *SP, off);
            id += 1;
            off += 4;
        }
    }

    pub fn epilogue(&mut self, ctx: &Context, saved_regs: (i32, i32), is_leaf: bool) {
        let (ra, sp) = ("ra".into_id(), "sp".into_id());
        let ss = ctx.cur_func().ss();
        if !is_leaf {
            self.load(ra, sp, ss - 4);
        }
        let mut off = saved_regs.0;
        let mut id = 0;
        while off < saved_regs.1 {
            self.load(format!("s{}", id).into_id(), *SP, off);
            id += 1;
            off += 4;
        }
        self.binary_with_imm(AsmBinaryOp::Add, sp, sp, ss);
        self.ret();
    }

    pub fn read_value_addr(&mut self, ctx: &Context, dst: RegID, val: Value) -> RegID {
        let sp = "sp".into_id();
        if ctx.is_global(val) {
            let name = ctx.get_global_var(&val);
            self.load_address(dst, name);
            dst
        } else {
            match ctx.get_local_place(val) {
                Place::Reg(id) => id,
                Place::Mem(offset) => {
                    self.load(dst, sp, offset);
                    dst
                }
            }
        }
    }

    pub fn read_value(&mut self, ctx: &Context, dst: RegID, val: Value) -> RegID {
        let t0 = *T0;
        let sp = *SP;
        if ctx.is_global(val) {
            let name = ctx.get_global_var(&val);
            self.load_address(t0, name);
            self.load(dst, t0, 0);
            return dst;
        }
        if let ValueKind::Integer(imm) = ctx.value_kind(val) {
            self.load_imm(dst, imm.value());
            dst
        } else {
            match ctx.get_local_place(val) {
                Place::Reg(id) => id,
                Place::Mem(offset) => {
                    self.load(dst, sp, offset);
                    dst
                }
            }
        }
    }

    pub fn move_local_value(&mut self, ctx: &Context, dst: Value, src: Value) {
        if let ValueKind::Integer(imm) = ctx.value_kind(src) {
            match ctx.get_local_place(dst) {
                Place::Reg(dst) => self.load_imm(dst, imm.value()),
                Place::Mem(off) => {
                    self.load_imm(*T0, imm.value());
                    self.store(*T0, *SP, off);
                }
            }
        } else {
            match (ctx.get_local_place(src), ctx.get_local_place(dst)) {
                (Place::Reg(src), Place::Reg(dst)) => self.mv(dst, src),
                (Place::Reg(id), Place::Mem(off)) => self.store(id, *SP, off),
                (Place::Mem(off), Place::Reg(id)) => self.load(id, *SP, off),
                (Place::Mem(src_off), Place::Mem(dst_off)) => {
                    self.load(*T0, *SP, src_off);
                    self.store(*T0, *SP, dst_off);
                }
            }
        }
    }

    pub fn ir_binary(&mut self, op: BinaryOp, dst: RegID, lhs: RegID, rhs: RegID) {
        match op {
            BinaryOp::Add => self.binary(AsmBinaryOp::Add, dst, lhs, rhs),
            BinaryOp::Sub => self.binary(AsmBinaryOp::Sub, dst, lhs, rhs),
            BinaryOp::Mul => self.binary(AsmBinaryOp::Mul, dst, lhs, rhs),
            BinaryOp::Div => self.binary(AsmBinaryOp::Div, dst, lhs, rhs),
            BinaryOp::Mod => self.binary(AsmBinaryOp::Rem, dst, lhs, rhs),
            BinaryOp::And => self.binary(AsmBinaryOp::And, dst, lhs, rhs),
            BinaryOp::Or => self.binary(AsmBinaryOp::Or, dst, lhs, rhs),
            BinaryOp::Lt => self.binary(AsmBinaryOp::Slt, dst, lhs, rhs),
            BinaryOp::Gt => self.binary(AsmBinaryOp::Sgt, dst, lhs, rhs),
            BinaryOp::Eq => {
                self.binary(AsmBinaryOp::Xor, dst, lhs, rhs);
                self.unary(AsmUnaryOp::Seqz, dst, dst);
            }
            BinaryOp::NotEq => {
                self.binary(AsmBinaryOp::Xor, dst, lhs, rhs);
                self.unary(AsmUnaryOp::Snez, dst, dst);
            }
            BinaryOp::Le => {
                self.binary(AsmBinaryOp::Sgt, dst, lhs, rhs);
                self.unary(AsmUnaryOp::Seqz, dst, dst);
            }
            BinaryOp::Ge => {
                self.binary(AsmBinaryOp::Slt, dst, lhs, rhs);
                self.unary(AsmUnaryOp::Seqz, dst, dst);
            }
            _ => unreachable!(),
        }
    }

    pub fn write_back(&mut self, ctx: &Context, src: RegID, val: Value) {
        let t0 = *T0;
        let sp = *SP;
        if ctx.is_global(val) {
            let label = ctx.get_global_var(&val);
            self.load_address(t0, label);
            self.store(src, t0, 0);
        } else {
            match ctx.get_local_place(val) {
                Place::Reg(reg) => {
                    if reg != src {
                        self.mv(reg, src);
                    }
                }
                Place::Mem(off) => self.store(src, sp, off),
            }
        }
    }

    pub fn muli(&mut self, dst: RegID, opr: RegID, imm: i32) {
        if imm == 0 {
            self.unary(AsmUnaryOp::Move, dst, "zero".into_id());
        } else if imm == 1 {
            if dst != opr {
                self.unary(AsmUnaryOp::Move, dst, opr);
            }
        } else if imm > 0 && (imm & (imm - 1)) == 0 {
            let mut shift = 0;
            let mut imm = imm >> 1;
            while imm != 0 {
                shift += 1;
                imm >>= 1;
            }
            self.binary_with_imm(AsmBinaryOp::Sll, dst, opr, shift)
        } else {
            self.load_imm("t0".into_id(), imm);
            self.binary(AsmBinaryOp::Mul, dst, opr, "t0".into_id())
        }
    }
}

pub enum AsmValue {
    LoadAddress(RegID, Lable),                 // la dst, lable
    LoadImm(RegID, i32),                       // li dst, imm
    Load(RegID, RegID, i32),                   // load dst, src
    Store(RegID, RegID, i32),                  // store src, dst
    Binary(AsmBinaryOp, RegID, RegID, RegID),  // op dst, lhs, rhs
    BinaryImm(AsmBinaryOp, RegID, RegID, i32), // op dst, lhs, imm
    Unary(AsmUnaryOp, RegID, RegID),           // op dst, opr
    Branch(RegID, Lable),
    Jump(Lable),
    Call(Lable),
    Directive(Directive),
    GlobalSymbol(Lable),
    LocalSymbol(Lable),
    Return,
}

pub type Lable = String;

pub enum AsmBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Slt,
    Sgt,
    Xor,
    Sll,
    #[allow(dead_code)]
    Sra,
}

pub enum AsmUnaryOp {
    Seqz,
    Snez,
    Move,
}

impl AsmBinaryOp {
    pub fn asm_name(&self) -> &'static str {
        lazy_static! {
            static ref OP_NAMES: Vec<&'static str> = vec![
                "add", "sub", "mul", "div", "rem", "and", "or", "slt", "sgt", "xor", "sll", "sra"
            ];
        };

        OP_NAMES[self.index()]
    }

    fn index(&self) -> usize {
        match self {
            Self::Add => 0,
            Self::Sub => 1,
            Self::Mul => 2,
            Self::Div => 3,
            Self::Rem => 4,
            Self::And => 5,
            Self::Or => 6,
            Self::Slt => 7,
            Self::Sgt => 8,
            Self::Xor => 9,
            Self::Sll => 10,
            Self::Sra => 11,
        }
    }
}

impl AsmUnaryOp {
    pub fn asm_name(&self) -> &'static str {
        match self {
            Self::Seqz => "seqz",
            Self::Snez => "snez",
            Self::Move => "mv",
        }
    }
}

impl Place {
    #[allow(dead_code)]
    pub fn reg_id(&self) -> RegID {
        if let Place::Reg(id) = self {
            *id
        } else {
            unreachable!()
        }
    }
}
