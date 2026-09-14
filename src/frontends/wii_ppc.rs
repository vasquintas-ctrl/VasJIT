use crate::core::intrinsics;
use crate::core::ir::{BinOp, BlockId, Endian, IrBuilder, IrOp, SideEffects, VOperand, Width};
use crate::frontends::{Frontend, RegisterLayout};

#[inline] pub fn opcd(w:u32)->u8{(w>>26) as u8}
#[inline] pub fn rd(w:u32)->u8{((w>>21)&0x1f) as u8}
#[inline] pub fn rs(w:u32)->u8{rd(w)}
#[inline] pub fn ra(w:u32)->u8{((w>>16)&0x1f) as u8}
#[inline] pub fn rb(w:u32)->u8{((w>>11)&0x1f) as u8}
#[inline] pub fn rc_bit(w:u32)->bool{w&1!=0}
#[inline] pub fn simm(w:u32)->i16{w as i16}
#[inline] pub fn uimm(w:u32)->u16{w as u16}
#[inline] pub fn xo_x(w:u32)->u16{((w>>1)&0x3ff) as u16}
#[inline] pub fn xo_a(w:u32)->u8{((w>>1)&0x1f) as u8}
#[inline] pub fn sh(w:u32)->u8{rb(w)}
#[inline] pub fn mb(w:u32)->u8{((w>>6)&0x1f) as u8}
#[inline] pub fn me(w:u32)->u8{((w>>1)&0x1f) as u8}
#[inline] pub fn lk(w:u32)->bool{w&1!=0}
#[inline] pub fn aa(w:u32)->bool{w&2!=0}
#[inline] pub fn spr_field(w:u32)->u16{((((w>>16)&0x1f)<<5)|((w>>11)&0x1f)) as u16}
#[inline] pub fn li_target(w:u32,pc:u64)->u64{
    let li=(((w as i32)<<6)>>6)&!3;
    if aa(w){(li as u32) as u64}else{pc.wrapping_add(li as i64 as u64)}
}
#[inline] pub fn bd_target(w:u32,pc:u64)->u64{
    let bd=(((w as i32)<<16)>>16)&!3;
    if aa(w){(bd as u32) as u64}else{pc.wrapping_add(bd as i64 as u64)}
}
pub fn mask_mbme(mb:u8,me:u8)->u32{
    let mb=mb as u32; let me=me as u32;
    if mb<=me{
        let start=u32::MAX.wrapping_shr(mb);
        let end=if me>=31{u32::MAX}else{!u32::MAX.wrapping_shr(me+1)};
        start&end
    }else{
        let start=u32::MAX.wrapping_shr(mb);
        let end=if me>=31{u32::MAX}else{u32::MAX.wrapping_shl(31-me)};
        start|end
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct DecodedPpc{pub raw:u32,pub opcode:u8,pub insn:PpcInsn}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PpcInsn{
    Addi{rd:u8,ra:u8,simm:i16}, Addis{rd:u8,ra:u8,simm:i16},
    Ori{ra:u8,rs:u8,uimm:u16}, Oris{ra:u8,rs:u8,uimm:u16},
    Andi{ra:u8,rs:u8,uimm:u16}, Xori{ra:u8,rs:u8,uimm:u16},
    Add{rd:u8,ra:u8,rb:u8,rc:bool}, Subf{rd:u8,ra:u8,rb:u8,rc:bool},
    Or{ra:u8,rs:u8,rb:u8,rc:bool}, And{ra:u8,rs:u8,rb:u8,rc:bool}, Xor{ra:u8,rs:u8,rb:u8,rc:bool},
    Lwz{rd:u8,ra:u8,d:i16,update:bool}, Stw{rs:u8,ra:u8,d:i16,update:bool},
    Lbz{rd:u8,ra:u8,d:i16}, Stb{rs:u8,ra:u8,d:i16},
    Rlwinm{ra:u8,rs:u8,sh:u8,mb:u8,me:u8,rc:bool},
    Mfspr{rd:u8,spr:u16}, Mtspr{spr:u16,rs:u8},
    B{target:u64,lk:bool}, Bclr{bo:u8,bi:u8,lk:bool}, Bc{bo:u8,bi:u8,target:u64,lk:bool},
    Crand{crbd:u8,crba:u8,crbb:u8},
    PsAdd{frd:u8,fra:u8,frb:u8},
    Unknown{raw:u32},
}
impl PpcInsn{
    pub fn mnemonic(self)->&'static str{
        use PpcInsn::*;
        match self{
            Addi{..}=>"addi",Addis{..}=>"addis",Ori{..}=>"ori",Oris{..}=>"oris",
            Andi{..}=>"andi.",Xori{..}=>"xori",Add{..}=>"add",Subf{..}=>"subf",
            Or{..}=>"or",And{..}=>"and",Xor{..}=>"xor",
            Lwz{update:false,..}=>"lwz",Lwz{update:true,..}=>"lwzu",
            Stw{update:false,..}=>"stw",Stw{update:true,..}=>"stwu",
            Lbz{..}=>"lbz",Stb{..}=>"stb",Rlwinm{..}=>"rlwinm",
            Mfspr{..}=>"mfspr",Mtspr{..}=>"mtspr",
            B{lk:false,..}=>"b",B{lk:true,..}=>"bl",
            Bclr{..}=>"bclr",Bc{..}=>"bc",Crand{..}=>"crand",PsAdd{..}=>"ps_add",
            Unknown{..}=>"unknown",
        }
    }
    pub fn is_terminator(self)->bool{
        matches!(self,PpcInsn::B{..}|PpcInsn::Bclr{..}|PpcInsn::Bc{..}|PpcInsn::Unknown{..})
    }
}
impl DecodedPpc{
    pub fn disasm(&self,pc:u64)->String{ format!("{pc:08x}:  {:08x}  {}",self.raw,self.insn.mnemonic()) }
}

pub fn decode_word(raw:u32,pc:u64)->DecodedPpc{
    let opcode=opcd(raw);
    let insn=match opcode{
        4 if xo_a(raw)==21 => PpcInsn::PsAdd{frd:rd(raw),fra:ra(raw),frb:rb(raw)},
        14 => PpcInsn::Addi{rd:rd(raw),ra:ra(raw),simm:simm(raw)},
        15 => PpcInsn::Addis{rd:rd(raw),ra:ra(raw),simm:simm(raw)},
        16 => PpcInsn::Bc{bo:rd(raw),bi:ra(raw),target:bd_target(raw,pc),lk:lk(raw)},
        18 => PpcInsn::B{target:li_target(raw,pc),lk:lk(raw)},
        19 => match xo_x(raw){
            16 => PpcInsn::Bclr{bo:rd(raw),bi:ra(raw),lk:lk(raw)},
            257 => PpcInsn::Crand{crbd:rd(raw),crba:ra(raw),crbb:rb(raw)},
            _ => PpcInsn::Unknown{raw},
        },
        21 => PpcInsn::Rlwinm{ra:ra(raw),rs:rs(raw),sh:sh(raw),mb:mb(raw),me:me(raw),rc:rc_bit(raw)},
        24 => PpcInsn::Ori{ra:ra(raw),rs:rs(raw),uimm:uimm(raw)},
        25 => PpcInsn::Oris{ra:ra(raw),rs:rs(raw),uimm:uimm(raw)},
        26 => PpcInsn::Xori{ra:ra(raw),rs:rs(raw),uimm:uimm(raw)},
        28 => PpcInsn::Andi{ra:ra(raw),rs:rs(raw),uimm:uimm(raw)},
        31 => {
            let xo=xo_x(raw); let xo9=xo&0x1ff; let rc=rc_bit(raw);
            match xo9{
                40 => PpcInsn::Subf{rd:rd(raw),ra:ra(raw),rb:rb(raw),rc},
                266 => PpcInsn::Add{rd:rd(raw),ra:ra(raw),rb:rb(raw),rc},
                _ => match xo{
                    28 => PpcInsn::And{ra:ra(raw),rs:rd(raw),rb:rb(raw),rc},
                    316 => PpcInsn::Xor{ra:ra(raw),rs:rd(raw),rb:rb(raw),rc},
                    339 => PpcInsn::Mfspr{rd:rd(raw),spr:spr_field(raw)},
                    444 => PpcInsn::Or{ra:ra(raw),rs:rd(raw),rb:rb(raw),rc},
                    467 => PpcInsn::Mtspr{spr:spr_field(raw),rs:rd(raw)},
                    _ => PpcInsn::Unknown{raw},
                }
            }
        },
        32 => PpcInsn::Lwz{rd:rd(raw),ra:ra(raw),d:simm(raw),update:false},
        33 => PpcInsn::Lwz{rd:rd(raw),ra:ra(raw),d:simm(raw),update:true},
        34 => PpcInsn::Lbz{rd:rd(raw),ra:ra(raw),d:simm(raw)},
        36 => PpcInsn::Stw{rs:rs(raw),ra:ra(raw),d:simm(raw),update:false},
        37 => PpcInsn::Stw{rs:rs(raw),ra:ra(raw),d:simm(raw),update:true},
        38 => PpcInsn::Stb{rs:rs(raw),ra:ra(raw),d:simm(raw)},
        _ => PpcInsn::Unknown{raw},
    };
    DecodedPpc{raw,opcode,insn}
}

pub struct WiiFrontend;
impl Frontend for WiiFrontend{
    type DecodedInsn=DecodedPpc;
    fn decode(&self,bytes:&[u8],pc:u64)->(DecodedPpc,usize){
        let raw=u32::from_be_bytes([bytes[0],bytes[1],bytes[2],bytes[3]]);
        (decode_word(raw,pc),4)
    }
    fn lower_to_ir(&self,insn:&DecodedPpc,ir:&mut IrBuilder,block:BlockId){
        ir.endian=Endian::Big; lower(insn.insn,insn.raw,ir,block);
    }
    fn register_file_layout(&self)->RegisterLayout{ RegisterLayout{gpr_count:32,fpr_count:32,vector_reg_count:32} }
    fn is_block_terminator(&self,insn:&DecodedPpc)->bool{ insn.insn.is_terminator() }
    fn endianness(&self)->Endian{ Endian::Big }
}

fn lower(insn:PpcInsn,_raw:u32,ir:&mut IrBuilder,block:BlockId){
    use PpcInsn::*;
    match insn{
        Addi{rd,ra,simm}=>{
            let a=ir.read_gpr_ra(block,ra);
            let v=ir.emit_binary(block,BinOp::Add,a,VOperand::Imm(simm as i64),Width::W32);
            ir.write_gpr(rd,v);
        }
        Addis{rd,ra,simm}=>{
            let a=ir.read_gpr_ra(block,ra);
            let v=ir.emit_binary(block,BinOp::Add,a,VOperand::Imm(((simm as i32)<<16) as i64),Width::W32);
            ir.write_gpr(rd,v);
        }
        Ori{ra,rs,uimm}=>{
            let a=VOperand::Reg(ir.read_gpr(block,rs));
            let v=ir.emit_binary(block,BinOp::Or,a,VOperand::Imm(uimm as i64),Width::W32);
            ir.write_gpr(ra,v);
        }
        Oris{ra,rs,uimm}=>{
            let a=VOperand::Reg(ir.read_gpr(block,rs));
            let v=ir.emit_binary(block,BinOp::Or,a,VOperand::Imm(((uimm as u32)<<16) as i64),Width::W32);
            ir.write_gpr(ra,v);
        }
        Andi{ra,rs,uimm}=>{
            let a=VOperand::Reg(ir.read_gpr(block,rs));
            let v=ir.emit_binary(block,BinOp::And,a,VOperand::Imm(uimm as i64),Width::W32);
            ir.write_gpr(ra,v); ir.emit_rc0_from_result(block,v,Width::W32);
        }
        Xori{ra,rs,uimm}=>{
            let a=VOperand::Reg(ir.read_gpr(block,rs));
            let v=ir.emit_binary(block,BinOp::Xor,a,VOperand::Imm(uimm as i64),Width::W32);
            ir.write_gpr(ra,v);
        }
        Add{rd,ra,rb,rc}=>{
            let a=VOperand::Reg(ir.read_gpr(block,ra));
            let b=VOperand::Reg(ir.read_gpr(block,rb));
            let v=ir.emit_binary(block,BinOp::Add,a,b,Width::W32);
            ir.write_gpr(rd,v); if rc{ir.emit_rc0_from_result(block,v,Width::W32);}
        }
        Subf{rd,ra,rb,rc}=>{
            let a=VOperand::Reg(ir.read_gpr(block,ra));
            let b=VOperand::Reg(ir.read_gpr(block,rb));
            let v=ir.emit_binary(block,BinOp::Sub,b,a,Width::W32);
            ir.write_gpr(rd,v); if rc{ir.emit_rc0_from_result(block,v,Width::W32);}
        }
        Or{ra,rs,rb,rc}|And{ra,rs,rb,rc}|Xor{ra,rs,rb,rc}=>{
            let op=match insn{Or{..}=>BinOp::Or,And{..}=>BinOp::And,_=>BinOp::Xor};
            let a=VOperand::Reg(ir.read_gpr(block,rs));
            let b=VOperand::Reg(ir.read_gpr(block,rb));
            let v=ir.emit_binary(block,op,a,b,Width::W32);
            ir.write_gpr(ra,v); if rc{ir.emit_rc0_from_result(block,v,Width::W32);}
        }
        Lwz{rd,ra,d,update}=>{
            let base=ir.read_gpr_ra(block,ra);
            let ea=ir.emit_binary(block,BinOp::Add,base,VOperand::Imm(d as i64),Width::W32);
            let v=ir.emit_load(block,VOperand::Reg(ea),Width::W32,false);
            ir.write_gpr(rd,v); if update{ir.write_gpr(ra,ea);}
        }
        Stw{rs,ra,d,update}=>{
            let base=ir.read_gpr_ra(block,ra);
            let ea=ir.emit_binary(block,BinOp::Add,base,VOperand::Imm(d as i64),Width::W32);
            let val=VOperand::Reg(ir.read_gpr(block,rs));
            ir.emit_store(block,VOperand::Reg(ea),val,Width::W32);
            if update{ir.write_gpr(ra,ea);}
        }
        Lbz{rd,ra,d}=>{
            let base=ir.read_gpr_ra(block,ra);
            let ea=ir.emit_binary(block,BinOp::Add,base,VOperand::Imm(d as i64),Width::W32);
            let v=ir.emit_load(block,VOperand::Reg(ea),Width::W8,false);
            ir.write_gpr(rd,v);
        }
        Stb{rs,ra,d}=>{
            let base=ir.read_gpr_ra(block,ra);
            let ea=ir.emit_binary(block,BinOp::Add,base,VOperand::Imm(d as i64),Width::W32);
            let val=VOperand::Reg(ir.read_gpr(block,rs));
            ir.emit_store(block,VOperand::Reg(ea),val,Width::W8);
        }
        Rlwinm{ra,rs,sh,mb,me,rc}=>{
            let src=VOperand::Reg(ir.read_gpr(block,rs));
            let rot=if sh==0{ match src{VOperand::Reg(v)=>v,_=>ir.emit_binary(block,BinOp::Or,src,VOperand::Imm(0),Width::W32)} }
            else{
                let hi=ir.emit_binary(block,BinOp::Shl,src,VOperand::Imm(sh as i64),Width::W32);
                let lo=ir.emit_binary(block,BinOp::Shr,src,VOperand::Imm((32-sh) as i64),Width::W32);
                ir.emit_binary(block,BinOp::Or,VOperand::Reg(hi),VOperand::Reg(lo),Width::W32)
            };
            let mask=mask_mbme(mb,me);
            let v=ir.emit_binary(block,BinOp::And,VOperand::Reg(rot),VOperand::Imm(mask as i64),Width::W32);
            ir.write_gpr(ra,v); if rc{ir.emit_rc0_from_result(block,v,Width::W32);}
        }
        Mfspr{rd,spr}=>{
            let v=match spr{1=>ir.read_xer(block),8=>ir.read_lr(block),9=>ir.read_ctr(block),
                _=>{let d=ir.new_vreg(); ir.push(block,IrOp::Intrinsic{id:crate::core::ir::IntrinsicId(intrinsics::GET_SPR),
                    effects:SideEffects::Pure,operands:vec![VOperand::Imm(spr as i64)],dst:Some(d)}); d}};
            ir.write_gpr(rd,v);
        }
        Mtspr{spr,rs}=>{
            let v=ir.read_gpr(block,rs);
            match spr{1=>ir.write_xer(v),8=>ir.write_lr(v),9=>ir.write_ctr(v),
                _=>ir.push(block,IrOp::Intrinsic{id:crate::core::ir::IntrinsicId(intrinsics::SET_SPR),
                    effects:SideEffects::Volatile,operands:vec![VOperand::Imm(spr as i64),VOperand::Reg(v)],dst:None})};
        }
        B{target,lk}=>{
            if lk{ let lr=ir.emit_binary(block,BinOp::Add,VOperand::Imm(ir.next_pc() as i64),VOperand::Imm(0),Width::W32); ir.write_lr(lr); }
            ir.emit_exit_uncond(block,target);
        }
        Bclr{bo,bi,lk}=>{
            if lk{ let lr=ir.emit_binary(block,BinOp::Add,VOperand::Imm(ir.next_pc() as i64),VOperand::Imm(0),Width::W32); ir.write_lr(lr); }
            let _=(bo,bi);
            let lr=ir.read_lr(block);
            ir.emit_exit_indirect(block,lr);
        }
        Bc{bo,bi,target,lk}=>{
            if lk{ let lr=ir.emit_binary(block,BinOp::Add,VOperand::Imm(ir.next_pc() as i64),VOperand::Imm(0),Width::W32); ir.write_lr(lr); }
            let ignore_cr=(bo&0x10)!=0;
            if ignore_cr{ ir.emit_exit_uncond(block,target); }
            else{
                let bit=ir.emit_get_cr_bit(block,bi);
                let want_set=(bo&0x08)!=0;
                let cond=if want_set{bit}else{ir.emit_binary(block,BinOp::Xor,VOperand::Reg(bit),VOperand::Imm(1),Width::W32)};
                ir.emit_exit_cond(block,cond,target,ir.next_pc());
            }
        }
        Crand{crbd,crba,crbb}=>{
            let a=ir.emit_get_cr_bit(block,crba); let b=ir.emit_get_cr_bit(block,crbb);
            let v=ir.emit_binary(block,BinOp::And,VOperand::Reg(a),VOperand::Reg(b),Width::W32);
            ir.emit_set_cr_field(block,crbd/4,v);
        }
        PsAdd{frd,fra,frb}=>{
            let a=VOperand::Reg(ir.read_vec(block,fra));
            let b=VOperand::Reg(ir.read_vec(block,frb));
            let dst=ir.new_vreg();
            ir.push(block,IrOp::VecAdd{dst,a,b,lanes:2,width:Width::W32});
            ir.write_vec(frd,dst);
        }
        Unknown{raw}=>ir.emit_unimplemented(block,raw),
    }
}

pub mod enc{
    pub fn addi(rd:u8,ra:u8,simm:i16)->u32{(14u32<<26)|((rd as u32)<<21)|((ra as u32)<<16)|(simm as u16 as u32)}
    pub fn add(rd:u8,ra:u8,rb:u8)->u32{(31u32<<26)|((rd as u32)<<21)|((ra as u32)<<16)|((rb as u32)<<11)|(266<<1)}
    pub fn ori(ra:u8,rs:u8,uimm:u16)->u32{(24u32<<26)|((rs as u32)<<21)|((ra as u32)<<16)|(uimm as u32)}
    pub fn lwz(rd:u8,ra:u8,d:i16)->u32{(32u32<<26)|((rd as u32)<<21)|((ra as u32)<<16)|(d as u16 as u32)}
    pub fn stw(rs:u8,ra:u8,d:i16)->u32{(36u32<<26)|((rs as u32)<<21)|((ra as u32)<<16)|(d as u16 as u32)}
    pub fn blr()->u32{(19u32<<26)|(20u32<<21)|(16<<1)}
    pub fn addis(rd:u8,ra:u8,simm:i16)->u32{(15u32<<26)|((rd as u32)<<21)|((ra as u32)<<16)|(simm as u16 as u32)}
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn addi_fields(){
        match decode_word(enc::addi(3,0,42),0).insn{
            PpcInsn::Addi{rd:3,ra:0,simm:42}=>{}, other=>panic!("{:?}",other),
        }
    }
    #[test] fn blr_term(){ assert!(decode_word(enc::blr(),0).insn.is_terminator()); }
    #[test] fn mask(){ assert_eq!(mask_mbme(0,31),0xffff_ffff); assert_eq!(mask_mbme(16,31),0xffff); }
}
