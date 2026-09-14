//! AArch64 class-based frontend (subset).
use crate::core::ir::{BinOp, BlockId, Endian, IrBuilder, VOperand, Width};
use crate::frontends::{Frontend, RegisterLayout};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct DecodedA64{pub raw:u32,pub insn:A64Insn}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum A64Insn{
    Nop,
    AddImm{rd:u8,rn:u8,imm:u32,sf:bool},
    Movz{rd:u8,imm:u16,hw:u8,sf:bool},
    Br{rn:u8,ret:bool},
    BImm{target:u64,link:bool},
    Unknown{raw:u32},
}
impl A64Insn{
    pub fn mnemonic(self)->&'static str{
        match self{
            A64Insn::Nop=>"nop", A64Insn::AddImm{..}=>"add", A64Insn::Movz{..}=>"movz",
            A64Insn::Br{ret:true,..}=>"ret", A64Insn::Br{..}=>"br",
            A64Insn::BImm{link:true,..}=>"bl", A64Insn::BImm{..}=>"b",
            A64Insn::Unknown{..}=>"unknown",
        }
    }
    pub fn is_terminator(self)->bool{
        matches!(self, A64Insn::Br{..}|A64Insn::BImm{..}|A64Insn::Unknown{..})
    }
}

pub fn decode_word(raw:u32,pc:u64)->DecodedA64{
    let class=((raw>>25)&0xf) as u8;
    let insn=match class{
        0b1000|0b1001 => {
            let op0=(raw>>23)&7; let sf=(raw>>31)!=0; let rd=(raw&0x1f) as u8; let rn=((raw>>5)&0x1f) as u8;
            if op0==0b010 || op0==0b011 {
                let imm=(raw>>10)&0xfff;
                A64Insn::AddImm{rd,rn,imm,sf}
            } else if op0==0b101 {
                let imm=((raw>>5)&0xffff) as u16; let hw=((raw>>21)&3) as u8;
                A64Insn::Movz{rd,imm,hw,sf}
            } else { A64Insn::Unknown{raw} }
        }
        0b1010|0b1011 => {
            if (raw>>26)==0x05 || (raw>>26)==0x25 {
                let link=(raw>>31)&1!=0;
                let imm26=(raw&0x03ff_ffff) as i32;
                let off=(imm26<<6)>>4;
                A64Insn::BImm{target:pc.wrapping_add(off as i64 as u64),link}
            } else if (raw>>25)&0x7f==0b1101011 {
                let opc=(raw>>21)&0xf; let rn=((raw>>5)&0x1f) as u8;
                match opc{
                    0b0010 => A64Insn::Br{rn,ret:true},
                    0b0000 => A64Insn::Br{rn,ret:false},
                    _ => A64Insn::Unknown{raw},
                }
            } else { A64Insn::Unknown{raw} }
        }
        _ if raw==0xd503201f => A64Insn::Nop,
        _ => A64Insn::Unknown{raw},
    };
    DecodedA64{raw,insn}
}

pub struct SwitchFrontend;
impl Frontend for SwitchFrontend{
    type DecodedInsn=DecodedA64;
    fn decode(&self,bytes:&[u8],pc:u64)->(DecodedA64,usize){
        let raw=u32::from_le_bytes([bytes[0],bytes[1],bytes[2],bytes[3]]);
        (decode_word(raw,pc),4)
    }
    fn lower_to_ir(&self,insn:&DecodedA64,ir:&mut IrBuilder,block:BlockId){
        ir.endian=Endian::Little;
        match insn.insn{
            A64Insn::Nop=>{}
            A64Insn::AddImm{rd,rn,imm,sf}=>{
                let w=if sf{Width::W64}else{Width::W32};
                let a=if rn==31{VOperand::Imm(0)}else{VOperand::Reg(ir.read_gpr(block,rn))};
                let v=ir.emit_binary(block,BinOp::Add,a,VOperand::Imm(imm as i64),w);
                if rd!=31{ir.write_gpr(rd,v);}
            }
            A64Insn::Movz{rd,imm,hw,sf}=>{
                let w=if sf{Width::W64}else{Width::W32};
                let v=ir.emit_binary(block,BinOp::Or,VOperand::Imm(0),VOperand::Imm(((imm as u64)<<(hw as u32*16)) as i64),w);
                if rd!=31{ir.write_gpr(rd,v);}
            }
            A64Insn::BImm{target,link}=>{
                if link{ let lr=ir.emit_binary(block,BinOp::Add,VOperand::Imm(ir.next_pc() as i64),VOperand::Imm(0),Width::W64); ir.write_gpr(30,lr); }
                ir.emit_exit_uncond(block,target);
            }
            A64Insn::Br{rn,..}=>{ let t=ir.read_gpr(block,rn); ir.emit_exit_indirect(block,t); }
            A64Insn::Unknown{raw}=>ir.emit_unimplemented(block,raw),
        }
    }
    fn register_file_layout(&self)->RegisterLayout{ RegisterLayout{gpr_count:31,fpr_count:32,vector_reg_count:32} }
    fn is_block_terminator(&self,insn:&DecodedA64)->bool{ insn.insn.is_terminator() }
    fn endianness(&self)->Endian{ Endian::Little }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn ret_decode(){
        let d=decode_word(0xd65f03c0,0);
        assert!(d.insn.is_terminator());
        assert_eq!(d.insn.mnemonic(),"ret");
    }
}
