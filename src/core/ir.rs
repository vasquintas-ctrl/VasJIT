//! Universal IR
use std::fmt;
use crate::core::intrinsics;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub struct VReg(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct BlockId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct IntrinsicId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Width { W8, W16, #[default] W32, W64, W128 }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Endian { #[default] Little, Big }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VOperand { Reg(VReg), Imm(i64) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp { Add, Sub, And, Or, Xor, Mul, Shl, Shr, Sar }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagOp { AddOp, SubOp, AndOp, OrOp, XorOp, TestOp }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagKind { Zero, Carry, Overflow, Negative }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideEffects { Pure, ReadsMem, WritesMem, Volatile }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrOp {
    Add{dst:VReg,a:VOperand,b:VOperand,width:Width},
    Sub{dst:VReg,a:VOperand,b:VOperand,width:Width},
    And{dst:VReg,a:VOperand,b:VOperand,width:Width},
    Or{dst:VReg,a:VOperand,b:VOperand,width:Width},
    Xor{dst:VReg,a:VOperand,b:VOperand,width:Width},
    Mul{dst:VReg,a:VOperand,b:VOperand,width:Width},
    Shl{dst:VReg,a:VOperand,b:VOperand,width:Width},
    Shr{dst:VReg,a:VOperand,b:VOperand,width:Width},
    Sar{dst:VReg,a:VOperand,b:VOperand,width:Width},
    Load{dst:VReg,addr:VOperand,width:Width,endian:Endian,sign_ext:bool},
    Store{addr:VOperand,val:VOperand,width:Width,endian:Endian},
    SetFlags{op:FlagOp,a:VOperand,b:VOperand,width:Width},
    ReadFlag{dst:VReg,flag:FlagKind},
    Branch{cond:Option<VOperand>,target:BlockId,taken_pc:u64,fallthrough_pc:u64},
    IndirectBranch{target:VOperand},
    Call{target:VOperand},
    Return,
    VecAdd{dst:VReg,a:VOperand,b:VOperand,lanes:u8,width:Width},
    VecMul{dst:VReg,a:VOperand,b:VOperand,lanes:u8,width:Width},
    VecFma{dst:VReg,a:VOperand,b:VOperand,c:VOperand,lanes:u8},
    Intrinsic{id:IntrinsicId,effects:SideEffects,operands:Vec<VOperand>,dst:Option<VReg>},
}
impl IrOp {
    pub fn is_control_flow(&self)->bool {
        matches!(self, IrOp::Branch{..}|IrOp::IndirectBranch{..}|IrOp::Call{..}|IrOp::Return)
    }
    pub fn dst(&self)->Option<VReg> {
        match self {
            IrOp::Add{dst,..}|IrOp::Sub{dst,..}|IrOp::And{dst,..}|IrOp::Or{dst,..}|IrOp::Xor{dst,..}|
            IrOp::Mul{dst,..}|IrOp::Shl{dst,..}|IrOp::Shr{dst,..}|IrOp::Sar{dst,..}|IrOp::Load{dst,..}|
            IrOp::ReadFlag{dst,..}|IrOp::VecAdd{dst,..}|IrOp::VecMul{dst,..}|IrOp::VecFma{dst,..} => Some(*dst),
            IrOp::Intrinsic{dst,..} => *dst, _ => None,
        }
    }
    pub fn operands(&self)->Vec<VOperand> {
        match self {
            IrOp::Add{a,b,..}|IrOp::Sub{a,b,..}|IrOp::And{a,b,..}|IrOp::Or{a,b,..}|IrOp::Xor{a,b,..}|
            IrOp::Mul{a,b,..}|IrOp::Shl{a,b,..}|IrOp::Shr{a,b,..}|IrOp::Sar{a,b,..}|
            IrOp::VecAdd{a,b,..}|IrOp::VecMul{a,b,..}|IrOp::SetFlags{a,b,..} => vec![*a,*b],
            IrOp::VecFma{a,b,c,..} => vec![*a,*b,*c],
            IrOp::Load{addr,..} => vec![*addr],
            IrOp::Store{addr,val,..} => vec![*addr,*val],
            IrOp::Branch{cond,..} => cond.into_iter().copied().collect(),
            IrOp::IndirectBranch{target}|IrOp::Call{target} => vec![*target],
            IrOp::Intrinsic{operands,..} => operands.clone(), _ => vec![],
        }
    }
}
impl fmt::Display for IrOp {
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result { write!(f,"{self:?}") }
}

#[derive(Clone, Debug, Default)]
pub struct IrBlock { pub guest_pc: u64, pub ops: Vec<IrOp> }

#[derive(Default)]
pub struct IrBuilder {
    pub blocks: Vec<IrBlock>, pub next_vreg: u32, pub endian: Endian,
    pub current_pc: u64, pub insn_len: u64,
    gpr: [Option<VReg>;32], fpr: [Option<VReg>;32], vec: [Option<VReg>;32],
    cr: Option<VReg>, lr: Option<VReg>, ctr: Option<VReg>, xer: Option<VReg>,
}
impl IrBuilder {
    pub fn new()->Self { Self{insn_len:4, ..Default::default()} }
    pub fn new_block(&mut self, guest_pc:u64)->BlockId {
        let id=BlockId(self.blocks.len() as u32);
        self.blocks.push(IrBlock{guest_pc, ops:Vec::new()});
        self.gpr=[None;32]; self.fpr=[None;32]; self.vec=[None;32];
        self.cr=None; self.lr=None; self.ctr=None; self.xer=None; id
    }
    pub fn next_pc(&self)->u64 { self.current_pc.wrapping_add(self.insn_len) }
    pub fn new_vreg(&mut self)->VReg { let v=VReg(self.next_vreg); self.next_vreg+=1; v }
    pub fn push(&mut self, block:BlockId, op:IrOp){ self.blocks[block.0 as usize].ops.push(op); }
    pub fn emit_binary(&mut self, block:BlockId, op:BinOp, a:VOperand, b:VOperand, width:Width)->VReg {
        let dst=self.new_vreg();
        let ir=match op {
            BinOp::Add=>IrOp::Add{dst,a,b,width}, BinOp::Sub=>IrOp::Sub{dst,a,b,width},
            BinOp::And=>IrOp::And{dst,a,b,width}, BinOp::Or=>IrOp::Or{dst,a,b,width},
            BinOp::Xor=>IrOp::Xor{dst,a,b,width}, BinOp::Mul=>IrOp::Mul{dst,a,b,width},
            BinOp::Shl=>IrOp::Shl{dst,a,b,width}, BinOp::Shr=>IrOp::Shr{dst,a,b,width},
            BinOp::Sar=>IrOp::Sar{dst,a,b,width},
        }; self.push(block,ir); dst
    }
    pub fn emit_load(&mut self, block:BlockId, addr:VOperand, width:Width, sign_ext:bool)->VReg {
        let dst=self.new_vreg();
        self.push(block, IrOp::Load{dst,addr,width,endian:self.endian,sign_ext}); dst
    }
    pub fn emit_store(&mut self, block:BlockId, addr:VOperand, val:VOperand, width:Width) {
        self.push(block, IrOp::Store{addr,val,width,endian:self.endian});
    }
    pub fn read_gpr(&mut self, block:BlockId, n:u8)->VReg {
        if let Some(v)=self.gpr[n as usize]{ return v; }
        let dst=self.new_vreg();
        self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::GET_GPR),effects:SideEffects::Pure,
            operands:vec![VOperand::Imm(n as i64)],dst:Some(dst)});
        self.gpr[n as usize]=Some(dst); dst
    }
    pub fn read_gpr_ra(&mut self, block:BlockId, ra:u8)->VOperand {
        if ra==0 { VOperand::Imm(0) } else { VOperand::Reg(self.read_gpr(block,ra)) }
    }
    pub fn write_gpr(&mut self, n:u8, v:VReg){ self.gpr[n as usize]=Some(v); }
    pub fn read_fpr(&mut self, block:BlockId, n:u8)->VReg {
        if let Some(v)=self.fpr[n as usize]{ return v; }
        let dst=self.new_vreg();
        self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::GET_FPR),effects:SideEffects::Pure,
            operands:vec![VOperand::Imm(n as i64)],dst:Some(dst)});
        self.fpr[n as usize]=Some(dst); dst
    }
    pub fn write_fpr(&mut self, n:u8, v:VReg){ self.fpr[n as usize]=Some(v); }
    pub fn read_vec(&mut self, block:BlockId, n:u8)->VReg {
        if let Some(v)=self.vec[n as usize]{ return v; }
        let dst=self.new_vreg();
        self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::GET_VEC),effects:SideEffects::Pure,
            operands:vec![VOperand::Imm(n as i64)],dst:Some(dst)});
        self.vec[n as usize]=Some(dst); dst
    }
    pub fn write_vec(&mut self, n:u8, v:VReg){ self.vec[n as usize]=Some(v); }
    pub fn read_cr(&mut self, block:BlockId)->VReg {
        if let Some(v)=self.cr{ return v; }
        let dst=self.new_vreg();
        self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::GET_CR),effects:SideEffects::Pure,operands:vec![],dst:Some(dst)});
        self.cr=Some(dst); dst
    }
    pub fn write_cr(&mut self, v:VReg){ self.cr=Some(v); }
    pub fn read_lr(&mut self, block:BlockId)->VReg {
        if let Some(v)=self.lr{ return v; }
        let dst=self.new_vreg();
        self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::GET_LR),effects:SideEffects::Pure,operands:vec![],dst:Some(dst)});
        self.lr=Some(dst); dst
    }
    pub fn write_lr(&mut self, v:VReg){ self.lr=Some(v); }
    pub fn read_ctr(&mut self, block:BlockId)->VReg {
        if let Some(v)=self.ctr{ return v; }
        let dst=self.new_vreg();
        self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::GET_CTR),effects:SideEffects::Pure,operands:vec![],dst:Some(dst)});
        self.ctr=Some(dst); dst
    }
    pub fn write_ctr(&mut self, v:VReg){ self.ctr=Some(v); }
    pub fn read_xer(&mut self, block:BlockId)->VReg {
        if let Some(v)=self.xer{ return v; }
        let dst=self.new_vreg();
        self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::GET_XER),effects:SideEffects::Pure,operands:vec![],dst:Some(dst)});
        self.xer=Some(dst); dst
    }
    pub fn write_xer(&mut self, v:VReg){ self.xer=Some(v); }
    pub fn emit_get_cr_bit(&mut self, block:BlockId, bit:u8)->VReg {
        let dst=self.new_vreg();
        self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::GET_CR_BIT),effects:SideEffects::Pure,
            operands:vec![VOperand::Imm(bit as i64)],dst:Some(dst)}); dst
    }
    pub fn emit_set_cr_field(&mut self, block:BlockId, field:u8, val:VReg) {
        self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::SET_CR_FIELD),effects:SideEffects::Volatile,
            operands:vec![VOperand::Imm(field as i64), VOperand::Reg(val)],dst:None});
        self.cr=None;
    }
    pub fn emit_rc0_from_result(&mut self, block:BlockId, result:VReg, width:Width) {
        self.push(block, IrOp::SetFlags{op:FlagOp::AndOp,a:VOperand::Reg(result),b:VOperand::Imm(-1),width});
        let z=self.new_vreg(); let n=self.new_vreg();
        self.push(block, IrOp::ReadFlag{dst:z,flag:FlagKind::Zero});
        self.push(block, IrOp::ReadFlag{dst:n,flag:FlagKind::Negative});
        let shifted=self.emit_binary(block,BinOp::Shl,VOperand::Reg(z),VOperand::Imm(2),Width::W32);
        let field=self.emit_binary(block, BinOp::Or, VOperand::Reg(shifted), VOperand::Reg(n), Width::W32);
        self.emit_set_cr_field(block,0,field);
    }
    pub fn emit_cmp_cr_field(&mut self, block:BlockId, crf:u8, a:VOperand, b:VOperand, width:Width, _signed:bool) {
        self.push(block, IrOp::SetFlags{op:FlagOp::SubOp,a,b,width});
        let z=self.new_vreg(); let n=self.new_vreg();
        self.push(block, IrOp::ReadFlag{dst:z,flag:FlagKind::Zero});
        self.push(block, IrOp::ReadFlag{dst:n,flag:FlagKind::Negative});
        let shifted=self.emit_binary(block,BinOp::Shl,VOperand::Reg(z),VOperand::Imm(2),Width::W32);
        let field=self.emit_binary(block, BinOp::Or, VOperand::Reg(shifted), VOperand::Reg(n), Width::W32);
        self.emit_set_cr_field(block,crf,field);
    }
    pub fn flush_guest_state(&mut self, block:BlockId) {
        for i in 0..32u8 {
            if let Some(v)=self.gpr[i as usize] {
                self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::SET_GPR),effects:SideEffects::Volatile,
                    operands:vec![VOperand::Imm(i as i64),VOperand::Reg(v)],dst:None});
            }
            if let Some(v)=self.fpr[i as usize] {
                self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::SET_FPR),effects:SideEffects::Volatile,
                    operands:vec![VOperand::Imm(i as i64),VOperand::Reg(v)],dst:None});
            }
            if let Some(v)=self.vec[i as usize] {
                self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::SET_VEC),effects:SideEffects::Volatile,
                    operands:vec![VOperand::Imm(i as i64),VOperand::Reg(v)],dst:None});
            }
        }
        if let Some(v)=self.cr {
            self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::SET_CR),effects:SideEffects::Volatile,
                operands:vec![VOperand::Reg(v)],dst:None});
        }
        if let Some(v)=self.lr {
            self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::SET_LR),effects:SideEffects::Volatile,
                operands:vec![VOperand::Reg(v)],dst:None});
        }
        if let Some(v)=self.ctr {
            self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::SET_CTR),effects:SideEffects::Volatile,
                operands:vec![VOperand::Reg(v)],dst:None});
        }
        if let Some(v)=self.xer {
            self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::SET_XER),effects:SideEffects::Volatile,
                operands:vec![VOperand::Reg(v)],dst:None});
        }
    }
    pub fn emit_set_pc(&mut self, block:BlockId, pc:VOperand) {
        self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::SET_PC),effects:SideEffects::Volatile,
            operands:vec![pc],dst:None});
    }
    pub fn emit_exit_uncond(&mut self, block:BlockId, target:u64) {
        self.flush_guest_state(block); self.emit_set_pc(block, VOperand::Imm(target as i64));
        self.push(block, IrOp::Branch{cond:None,target:BlockId(0),taken_pc:target,fallthrough_pc:target});
    }
    pub fn emit_exit_cond(&mut self, block:BlockId, cond:VReg, taken:u64, fall:u64) {
        self.flush_guest_state(block);
        self.push(block, IrOp::Branch{cond:Some(VOperand::Reg(cond)),target:BlockId(0),taken_pc:taken,fallthrough_pc:fall});
    }
    pub fn emit_exit_indirect(&mut self, block:BlockId, target:VReg) {
        self.flush_guest_state(block); self.emit_set_pc(block, VOperand::Reg(target));
        self.push(block, IrOp::IndirectBranch{target:VOperand::Reg(target)});
    }
    pub fn emit_return(&mut self, block:BlockId) {
        self.flush_guest_state(block); self.push(block, IrOp::Return);
    }
    pub fn emit_unimplemented(&mut self, block:BlockId, raw:u32) {
        self.push(block, IrOp::Intrinsic{id:IntrinsicId(intrinsics::UNIMPLEMENTED),effects:SideEffects::Volatile,
            operands:vec![VOperand::Imm(raw as i64)],dst:None});
    }
}
