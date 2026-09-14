use std::os::raw::{c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CHostCaps { pub bits: u32 }
impl CHostCaps {
    pub const SSE2: u32 = 1<<0;
    pub const SSE4_1: u32 = 1<<1;
    pub const AVX: u32 = 1<<2;
    pub const AVX2: u32 = 1<<3;
    pub const FMA: u32 = 1<<4;
    pub const BMI2: u32 = 1<<5;
    pub const MOVBE: u32 = 1<<6;
    pub const AVX512F: u32 = 1<<7;
    pub fn has(self, bit: u32) -> bool { self.bits & bit != 0 }
}

#[repr(C)] #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CWidth { W8=0, W16, W32, W64, W128 }
#[repr(C)] #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CEndian { Little=0, Big }
#[repr(C)] #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CIrOpKind {
    Add=0, Sub, And, Or, Xor, Mul, Shl, Shr, Sar, Load, Store,
    SetFlags, ReadFlag, Branch, IndirectBranch, Call, Return,
    VecAdd, VecMul, VecFma, Intrinsic,
}
#[repr(C)] #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CFlagOp { AddOp=0, SubOp, AndOp, OrOp, XorOp, TestOp }
#[repr(C)] #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CFlagKind { Zero=0, Carry, Overflow, Negative }
#[repr(C)] #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum COperandKind { Reg=0, Imm }
#[repr(C)] #[derive(Clone, Copy, Debug)]
pub struct COperand { pub kind: COperandKind, pub reg: u32, pub imm: i64 }
pub const CIR_MAX_OPERANDS: usize = 8;
#[repr(C)] #[derive(Clone, Copy, Debug)]
pub struct CIrOp {
    pub kind: CIrOpKind, pub width: CWidth, pub endian: CEndian,
    pub sign_ext: u8, pub dst_valid: u8, pub dst: u32, pub num_operands: u32,
    pub operands: [COperand; CIR_MAX_OPERANDS],
    pub flag_op: CFlagOp, pub flag_kind: CFlagKind, pub intrinsic_id: u32, pub lanes: u8,
}
#[repr(C)] #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CLocationKind { Reg=0, Spill }
#[repr(C)] #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CHostGpr {
    Rax=0, Rbx, Rcx, Rdx, Rsi, Rdi, R8, R9, R10, R11, R12, R13, R14,
}
#[repr(C)] #[derive(Clone, Copy, Debug)]
pub struct CLocation { pub kind: CLocationKind, pub gpr: CHostGpr, pub offset: u32 }
#[repr(C)] #[derive(Clone, Copy, Debug)]
pub struct CAllocEntry { pub vreg: u32, pub loc: CLocation }
#[repr(C)] #[derive(Clone, Copy, Debug)]
pub struct CIrBlock {
    pub guest_pc: u64, pub ops: *const CIrOp, pub num_ops: u32,
    pub allocs: *const CAllocEntry, pub num_allocs: u32,
}
#[repr(C)] #[derive(Clone, Copy, Debug)]
pub struct CPatchSite { pub code_offset: u32, pub target_guest_pc: u64 }
#[repr(C)] #[derive(Clone, Copy, Debug)]
pub struct CEmitResult {
    pub code: *mut u8, pub code_len: usize,
    pub patch_sites: *mut CPatchSite, pub num_patch_sites: u32,
}
#[repr(C)] #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CJitStatus { Ok=0, InvalidInput=1, OutOfMemory=2, EmitFailed=3, IntrinsicOverflow=4 }

extern "C" {
    pub fn jit_detect_host_caps() -> CHostCaps;
    pub fn jit_emit_block(block: *const CIrBlock, caps: CHostCaps, force_baseline: u8, out: *mut CEmitResult) -> CJitStatus;
    pub fn jit_free_code(code: *mut u8);
    pub fn jit_patch_chain(code: *mut u8, code_len: usize, patch_offset: u32, target_entry: *const u8) -> CJitStatus;
}
#[allow(dead_code)]
fn _m() { let _: *mut c_void = std::ptr::null_mut(); let _: c_int = 0; }
