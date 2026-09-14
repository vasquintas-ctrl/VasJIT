use crate::core::ir::{IrBlock, IrOp, VOperand, Width, Endian, FlagOp, FlagKind};
use crate::core::regalloc::Allocation;
use crate::ffi::*;
use std::ptr;

pub fn detect_host_caps() -> CHostCaps { unsafe { jit_detect_host_caps() } }

pub struct CompiledBlock {
    pub code_ptr: *mut u8,
    pub code_len: usize,
    pub patchable_exits: Vec<CPatchSite>,
}
impl Drop for CompiledBlock {
    fn drop(&mut self) {
        if !self.code_ptr.is_null() {
            unsafe { jit_free_code(self.code_ptr) };
            self.code_ptr = ptr::null_mut();
        }
    }
}
unsafe impl Send for CompiledBlock {}
unsafe impl Sync for CompiledBlock {}

pub fn emit_block(block:&IrBlock, alloc:&Allocation, caps:CHostCaps, force_baseline:bool)->Result<CompiledBlock,CJitStatus> {
    let c_ops: Vec<CIrOp> = block.ops.iter().map(|op| to_c_op(op)).collect::<Result<Vec<_>,_>>()?;
    let c_allocs = alloc.to_c_entries();
    let cir = CIrBlock {
        guest_pc: block.guest_pc, ops: c_ops.as_ptr(), num_ops: c_ops.len() as u32,
        allocs: c_allocs.as_ptr(), num_allocs: c_allocs.len() as u32,
    };
    let mut out = CEmitResult{code:ptr::null_mut(),code_len:0,patch_sites:ptr::null_mut(),num_patch_sites:0};
    let status = unsafe { jit_emit_block(&cir, caps, if force_baseline{1}else{0}, &mut out) };
    if status != CJitStatus::Ok { return Err(status); }
    let mut sites=Vec::new();
    if out.num_patch_sites>0 && !out.patch_sites.is_null() {
        unsafe { sites.extend_from_slice(std::slice::from_raw_parts(out.patch_sites, out.num_patch_sites as usize)); }
    }
    Ok(CompiledBlock{code_ptr:out.code, code_len:out.code_len, patchable_exits:sites})
}

fn to_c_width(w:Width)->CWidth {
    match w { Width::W8=>CWidth::W8, Width::W16=>CWidth::W16, Width::W32=>CWidth::W32, Width::W64=>CWidth::W64, Width::W128=>CWidth::W128 }
}
fn to_c_endian(e:Endian)->CEndian { match e { Endian::Little=>CEndian::Little, Endian::Big=>CEndian::Big } }
fn empty_ops()->[COperand;CIR_MAX_OPERANDS] {
    [COperand{kind:COperandKind::Imm,reg:0,imm:0}; CIR_MAX_OPERANDS]
}
fn push_op(arr:&mut [COperand;CIR_MAX_OPERANDS], n:&mut u32, op:VOperand)->Result<(),CJitStatus> {
    if *n as usize >= CIR_MAX_OPERANDS { return Err(CJitStatus::IntrinsicOverflow); }
    arr[*n as usize]=match op {
        VOperand::Reg(v)=>COperand{kind:COperandKind::Reg,reg:v.0,imm:0},
        VOperand::Imm(i)=>COperand{kind:COperandKind::Imm,reg:0,imm:i},
    };
    *n+=1; Ok(())
}
fn to_c_op(op:&IrOp)->Result<CIrOp,CJitStatus> {
    let mut operands=empty_ops(); let mut num=0u32;
    let mut kind=CIrOpKind::Add; let mut width=CWidth::W32; let mut endian=CEndian::Little;
    let mut sign_ext=0u8; let mut dst_valid=0u8; let mut dst=0u32;
    let mut flag_op=CFlagOp::AddOp; let mut flag_kind=CFlagKind::Zero;
    let mut intrinsic_id=0u32; let mut lanes=0u8;
    match op {
        IrOp::Add{dst:d,a,b,width:w}|IrOp::Sub{dst:d,a,b,width:w}|IrOp::And{dst:d,a,b,width:w}|
        IrOp::Or{dst:d,a,b,width:w}|IrOp::Xor{dst:d,a,b,width:w}|IrOp::Mul{dst:d,a,b,width:w}|
        IrOp::Shl{dst:d,a,b,width:w}|IrOp::Shr{dst:d,a,b,width:w}|IrOp::Sar{dst:d,a,b,width:w} => {
            kind=match op {
                IrOp::Add{..}=>CIrOpKind::Add, IrOp::Sub{..}=>CIrOpKind::Sub, IrOp::And{..}=>CIrOpKind::And,
                IrOp::Or{..}=>CIrOpKind::Or, IrOp::Xor{..}=>CIrOpKind::Xor, IrOp::Mul{..}=>CIrOpKind::Mul,
                IrOp::Shl{..}=>CIrOpKind::Shl, IrOp::Shr{..}=>CIrOpKind::Shr, IrOp::Sar{..}=>CIrOpKind::Sar, _=>unreachable!()
            };
            width=to_c_width(*w); dst_valid=1; dst=d.0;
            push_op(&mut operands,&mut num,*a)?; push_op(&mut operands,&mut num,*b)?;
        }
        IrOp::Load{dst:d,addr,width:w,endian:e,sign_ext:sx} => {
            kind=CIrOpKind::Load; width=to_c_width(*w); endian=to_c_endian(*e);
            sign_ext=if *sx{1}else{0}; dst_valid=1; dst=d.0; push_op(&mut operands,&mut num,*addr)?;
        }
        IrOp::Store{addr,val,width:w,endian:e} => {
            kind=CIrOpKind::Store; width=to_c_width(*w); endian=to_c_endian(*e);
            push_op(&mut operands,&mut num,*addr)?; push_op(&mut operands,&mut num,*val)?;
        }
        IrOp::SetFlags{op:fop,a,b,width:w} => {
            kind=CIrOpKind::SetFlags; width=to_c_width(*w);
            flag_op=match fop {
                FlagOp::AddOp=>CFlagOp::AddOp, FlagOp::SubOp=>CFlagOp::SubOp, FlagOp::AndOp=>CFlagOp::AndOp,
                FlagOp::OrOp=>CFlagOp::OrOp, FlagOp::XorOp=>CFlagOp::XorOp, FlagOp::TestOp=>CFlagOp::TestOp,
            };
            push_op(&mut operands,&mut num,*a)?; push_op(&mut operands,&mut num,*b)?;
        }
        IrOp::ReadFlag{dst:d,flag} => {
            kind=CIrOpKind::ReadFlag; dst_valid=1; dst=d.0;
            flag_kind=match flag {
                FlagKind::Zero=>CFlagKind::Zero, FlagKind::Carry=>CFlagKind::Carry,
                FlagKind::Overflow=>CFlagKind::Overflow, FlagKind::Negative=>CFlagKind::Negative,
            };
        }
        IrOp::Branch{cond,taken_pc,fallthrough_pc,..} => {
            kind=CIrOpKind::Branch;
            if let Some(c)=cond { push_op(&mut operands,&mut num,*c)?; }
            push_op(&mut operands,&mut num,VOperand::Imm(*taken_pc as i64))?;
            push_op(&mut operands,&mut num,VOperand::Imm(*fallthrough_pc as i64))?;
        }
        IrOp::IndirectBranch{target} => { kind=CIrOpKind::IndirectBranch; push_op(&mut operands,&mut num,*target)?; }
        IrOp::Call{target} => { kind=CIrOpKind::Call; push_op(&mut operands,&mut num,*target)?; }
        IrOp::Return => { kind=CIrOpKind::Return; }
        IrOp::VecAdd{dst:d,a,b,lanes:l,width:w}|IrOp::VecMul{dst:d,a,b,lanes:l,width:w} => {
            kind=match op { IrOp::VecAdd{..}=>CIrOpKind::VecAdd, _=>CIrOpKind::VecMul };
            width=to_c_width(*w); lanes=*l; dst_valid=1; dst=d.0;
            push_op(&mut operands,&mut num,*a)?; push_op(&mut operands,&mut num,*b)?;
        }
        IrOp::VecFma{dst:d,a,b,c,lanes:l} => {
            kind=CIrOpKind::VecFma; lanes=*l; dst_valid=1; dst=d.0;
            push_op(&mut operands,&mut num,*a)?; push_op(&mut operands,&mut num,*b)?; push_op(&mut operands,&mut num,*c)?;
        }
        IrOp::Intrinsic{id,operands:ops,dst:d,..} => {
            kind=CIrOpKind::Intrinsic; intrinsic_id=id.0;
            if let Some(dv)=d { dst_valid=1; dst=dv.0; }
            for o in ops { push_op(&mut operands,&mut num,*o)?; }
        }
    }
    Ok(CIrOp{kind,width,endian,sign_ext,dst_valid,dst,num_operands:num,operands,flag_op,flag_kind,intrinsic_id,lanes})
}
