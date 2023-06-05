use super::*;
use lazy_static_include::lazy_static::lazy_static;
use strum_macros::Display;

#[derive(Debug, Clone)]
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

    fn compress_extend(&mut self, values: &[AsmValue]) {
        self.values.pop();
        self.values.pop();
        self.values.extend(values.to_vec());
    }

    fn compress_back(&mut self, val: AsmValue) {
        self.values.pop();
        self.values.pop();
        self.values.push(val);
    }

    fn compress_branch(&mut self) -> bool {
        let invalid = "zero".into_id();
        let mut iter = self.values.iter().rev().peekable();
        match (iter.next().unwrap(), iter.next().unwrap()) {
            (
                AsmValue::Branch(BranchOp::Bnez, _, _, target),
                AsmValue::Unary(AsmUnaryOp::Seqz, _, opr),
            ) => self.compress_back(AsmValue::Branch(
                BranchOp::Beqz,
                *opr,
                invalid,
                target.clone(),
            )),
            (
                AsmValue::Branch(BranchOp::Bnez, _, _, target),
                AsmValue::Unary(AsmUnaryOp::Snez, _, opr),
            ) => self.compress_back(AsmValue::Branch(
                BranchOp::Bnez,
                *opr,
                invalid,
                target.clone(),
            )),
            (
                AsmValue::Branch(BranchOp::Bnez, _, _, target),
                AsmValue::Binary(AsmBinaryOp::Slt, _, lhs, rhs),
            ) => self.compress_back(AsmValue::Branch(BranchOp::Blt, *lhs, *rhs, target.clone())),
            (
                AsmValue::Branch(BranchOp::Bnez, _, _, target),
                AsmValue::Binary(AsmBinaryOp::Sgt, _, lhs, rhs),
            ) => self.compress_back(AsmValue::Branch(BranchOp::Bgt, *lhs, *rhs, target.clone())),
            (
                AsmValue::Branch(BranchOp::Beqz, lhs, _, target),
                AsmValue::BinaryImm(AsmBinaryOp::Xori, _, rhs, imm),
            ) => self.compress_extend(&[
                AsmValue::LoadImm(*lhs, *imm),
                AsmValue::Branch(BranchOp::Beq, *lhs, *rhs, target.clone()),
            ]),
            (
                AsmValue::Branch(BranchOp::Bnez, lhs, _, target),
                AsmValue::BinaryImm(AsmBinaryOp::Xori, _, rhs, imm),
            ) => self.compress_extend(&[
                AsmValue::LoadImm(*lhs, *imm),
                AsmValue::Branch(BranchOp::Bne, *lhs, *rhs, target.clone()),
            ]),
            _ => return false,
        }

        true
    }

    pub fn branch(&mut self, cond: RegID, target: &str) {
        self.push(AsmValue::Branch(
            BranchOp::Bnez,
            cond,
            "zero".into_id(),
            target.to_string(),
        ));
        while self.compress_branch() {}
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
        self.binary_with_imm(AsmBinaryOp::Addi, *SP, *SP, -ss);
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
        self.binary_with_imm(AsmBinaryOp::Addi, sp, sp, ss);
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
        let sp = *SP;
        if ctx.is_global(val) {
            let name = ctx.get_global_var(&val);
            self.load_address(dst, name);
            self.load(dst, dst, 0);
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

    pub fn try_remove_redundant_mv(&mut self, dst: RegID, src: RegID) -> bool {
        let mut iter = self.values.iter_mut().rev();
        while let Some(val) = iter.next() {
            match val {
                AsmValue::Binary(_, ddst, lhs, rhs) => {
                    if *ddst == src {
                        *ddst = dst;
                        break;
                    }
                    if *lhs == src || *rhs == src {
                        return false;
                    }
                }
                AsmValue::BinaryImm(_, ddst, opr, _) => {
                    if *ddst == src {
                        *ddst = dst;
                        break;
                    }
                    if *opr == src {
                        return false;
                    }
                }
                AsmValue::Unary(_, ddst, opr) => {
                    if *ddst == src {
                        *ddst = dst;
                        break;
                    }
                    if *opr == src {
                        return false;
                    }
                }
                AsmValue::Load(ddst, ssrc, _) => {
                    if *ddst == src {
                        *ddst = dst;
                        break;
                    }
                    if *ssrc == src {
                        return false;
                    }
                }
                AsmValue::LoadImm(ddst, _) if *ddst == src => {
                    *ddst = dst;
                    break;
                }
                AsmValue::Store(reg1, reg2, _) => {
                    if *reg1 == src || *reg2 == src {
                        return false;
                    }
                }
                AsmValue::Branch(_, reg1, reg2, _) => {
                    if *reg1 == src || *reg2 == src {
                        return false;
                    }
                }
                _ => return false,
            }
        }

        true
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
                (Place::Reg(src), Place::Reg(dst)) => {
                    if !self.try_remove_redundant_mv(dst, src) {
                        self.mv(dst, src)
                    }
                }
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

    pub fn ir_binary_with_imm(&mut self, op: BinaryOp, dst: RegID, lhs: RegID, imm: i32) {
        match op {
            BinaryOp::Add => self.binary_with_imm(AsmBinaryOp::Addi, dst, lhs, imm),
            BinaryOp::Sub => self.binary_with_imm(AsmBinaryOp::Addi, dst, lhs, -imm),
            BinaryOp::Mul => self.muli(dst, lhs, imm),
            BinaryOp::Div => self.divi(dst, lhs, imm),
            BinaryOp::Mod => self.remi(dst, lhs, imm),
            BinaryOp::And => self.binary_with_imm(AsmBinaryOp::Andi, dst, lhs, imm),
            BinaryOp::Or => self.binary_with_imm(AsmBinaryOp::Ori, dst, lhs, imm),
            BinaryOp::Eq => {
                self.binary_with_imm(AsmBinaryOp::Xori, dst, lhs, imm);
                self.unary(AsmUnaryOp::Seqz, dst, dst);
            }
            BinaryOp::NotEq => {
                self.binary_with_imm(AsmBinaryOp::Xori, dst, lhs, imm);
                self.unary(AsmUnaryOp::Snez, dst, dst);
            }
            BinaryOp::Ge => {
                self.binary_with_imm(AsmBinaryOp::Slti, dst, lhs, imm);
                self.unary(AsmUnaryOp::Seqz, dst, dst);
            }
            _ => {
                self.load_imm(dst, imm);
                self.ir_binary(op, dst, lhs, dst)
            }
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
            self.binary_with_imm(AsmBinaryOp::Slli, dst, opr, shift)
        } else {
            self.load_imm(dst, imm);
            self.binary(AsmBinaryOp::Mul, dst, opr, dst)
        }
    }

    pub fn divi(&mut self, dst: RegID, opr: RegID, imm: i32) {
        if imm == 1 {
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
            self.binary_with_imm(AsmBinaryOp::Srai, dst, opr, shift)
        } else {
            self.load_imm(dst, imm);
            self.binary(AsmBinaryOp::Div, dst, opr, dst)
        }
    }

    pub fn remi(&mut self, dst: RegID, opr: RegID, imm: i32) {
        if imm > 0 && (imm & (imm - 1)) == 0 {
            self.binary_with_imm(AsmBinaryOp::Andi, dst, opr, imm - 1)
        } else {
            self.load_imm(dst, imm);
            self.binary(AsmBinaryOp::Rem, dst, opr, dst)
        }
    }
}

#[derive(Debug, Clone)]
pub enum AsmValue {
    LoadAddress(RegID, Lable),                 // la dst, lable
    LoadImm(RegID, i32),                       // li dst, imm
    Load(RegID, RegID, i32),                   // load dst, src
    Store(RegID, RegID, i32),                  // store src, dst
    Binary(AsmBinaryOp, RegID, RegID, RegID),  // op dst, lhs, rhs
    BinaryImm(AsmBinaryOp, RegID, RegID, i32), // op dst, lhs, imm
    Unary(AsmUnaryOp, RegID, RegID),           // op dst, opr
    Branch(BranchOp, RegID, RegID, Lable),
    Jump(Lable),
    Call(Lable),
    Directive(Directive),
    GlobalSymbol(Lable),
    LocalSymbol(Lable),
    Return,
}

pub type Lable = String;

#[derive(Debug, Display, Clone, Copy)]
pub enum AsmBinaryOp {
    #[strum(serialize = "add")]
    Add,
    #[strum(serialize = "addi")]
    Addi,
    #[strum(serialize = "sub")]
    Sub,
    #[strum(serialize = "mul")]
    Mul,
    #[strum(serialize = "div")]
    Div,
    #[strum(serialize = "rem")]
    Rem,
    #[strum(serialize = "and")]
    And,
    #[strum(serialize = "andi")]
    Andi,
    #[strum(serialize = "or")]
    Or,
    #[strum(serialize = "ori")]
    Ori,
    #[strum(serialize = "slt")]
    Slt,
    #[strum(serialize = "slti")]
    Slti,
    #[strum(serialize = "sgt")]
    Sgt,
    #[strum(serialize = "xor")]
    Xor,
    #[strum(serialize = "xori")]
    Xori,
    #[strum(serialize = "slli")]
    Slli,
    #[strum(serialize = "srai")]
    Srai,
}

impl AsmBinaryOp {
    pub fn into_reg_type(self) -> Self {
        match self {
            Self::Addi => Self::Add,
            Self::Andi => Self::And,
            Self::Ori => Self::Or,
            Self::Slti => Self::Slt,
            Self::Xori => Self::Xor,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Display, Clone)]
pub enum AsmUnaryOp {
    #[strum(serialize = "seqz")]
    Seqz,
    #[strum(serialize = "snez")]
    Snez,
    #[strum(serialize = "mv")]
    Move,
}

#[derive(Debug, Display, Clone)]
pub enum BranchOp {
    #[strum(serialize = "bnez")]
    Bnez,
    #[strum(serialize = "beqz")]
    Beqz,
    #[strum(serialize = "blt")]
    Blt,
    #[strum(serialize = "bgt")]
    Bgt,
    #[strum(serialize = "beq")]
    Beq,
    #[strum(serialize = "bne")]
    Bne,
}
