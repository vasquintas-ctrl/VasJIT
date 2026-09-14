pub mod switch_arm64;
pub mod wii_ppc;
use crate::core::ir::{BlockId, Endian, IrBuilder};
pub struct RegisterLayout{pub gpr_count:u8, pub fpr_count:u8, pub vector_reg_count:u8}
pub trait Frontend{
    type DecodedInsn;
    fn decode(&self, bytes:&[u8], pc:u64)->(Self::DecodedInsn, usize);
    fn lower_to_ir(&self, insn:&Self::DecodedInsn, ir:&mut IrBuilder, block:BlockId);
    fn register_file_layout(&self)->RegisterLayout;
    fn is_block_terminator(&self, insn:&Self::DecodedInsn)->bool;
    fn endianness(&self)->Endian;
}
