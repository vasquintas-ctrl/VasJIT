use crate::core::cache::CodeCache;
use crate::core::ir::{IrBuilder, IrOp, VOperand};
use crate::core::jit_ffi::CompiledBlock;
use crate::core::memory::GuestMemory;
use crate::core::{passes, regalloc};
use crate::ffi::CHostCaps;
use crate::frontends::Frontend;

#[repr(C)]
#[derive(Clone)]
pub struct CpuState {
    pub gpr: [u64;32],
    pub fpr: [f64;32],
    pub vec: [u128;32],
    pub pc: u64,
    pub flags_zero: bool,
    pub flags_carry: bool,
    pub flags_overflow: bool,
    pub flags_negative: bool,
    pub spill: [u64;128],
    pub cr: u64,
    pub lr: u64,
    pub ctr: u64,
    pub xer: u64,
    pub msr: u64,
    pub fpscr: u64,
    pub gqr: [u64;8],
    pub mem_base: u64,
    pub jit_tmp0: u64,
    pub jit_tmp1: u64,
}
impl CpuState {
    pub fn new()->Self { unsafe { std::mem::zeroed() } }
}
impl Default for CpuState { fn default()->Self { Self::new() } }

pub struct GuestSystem<F: Frontend> {
    pub cpu: CpuState,
    pub cache: CodeCache,
    pub frontend: F,
    pub host_caps: CHostCaps,
    pub force_baseline: bool,
    pub memory: GuestMemory,
    pub halted: bool,
}
impl<F: Frontend> GuestSystem<F> {
    pub fn new(frontend:F, host_caps:CHostCaps, force_baseline:bool)->std::io::Result<Self> {
        let memory=GuestMemory::new()?;
        let mut cpu=CpuState::new();
        cpu.mem_base=memory.base_ptr() as u64;
        Ok(Self{cpu,cache:CodeCache::new(32*1024*1024),frontend,host_caps,force_baseline,memory,halted:false})
    }
    pub fn write_guest(&mut self, addr:u32, bytes:&[u8]) { self.memory.write_bytes(addr,bytes); }
    pub fn run_steps(&mut self, max_blocks:usize) {
        for _ in 0..max_blocks {
            if self.halted { break; }
            let pc=self.cpu.pc;
            if self.cache.lookup(pc).is_none() { self.translate_block_at(pc); }
            self.cache.touch(pc);
            let block=self.cache.lookup(pc).expect("translated");
            self.cpu.pc=unsafe{Self::execute_block(block,&mut self.cpu)};
        }
    }
    pub fn translate_block_at(&mut self, guest_pc:u64) {
        let mut builder=IrBuilder::new();
        builder.endian=self.frontend.endianness();
        builder.insn_len=4;
        let block_id=builder.new_block(guest_pc);
        let mut cursor_pc=guest_pc;
        for _ in 0..64 {
            let raw=self.memory.fetch_guest_bytes(cursor_pc);
            let (decoded,len)=self.frontend.decode(&raw,cursor_pc);
            builder.current_pc=cursor_pc;
            builder.insn_len=len as u64;
            self.frontend.lower_to_ir(&decoded,&mut builder,block_id);
            let term=self.frontend.is_block_terminator(&decoded);
            cursor_pc=cursor_pc.wrapping_add(len as u64);
            if term { break; }
        }
        let last_is_cf=builder.blocks.get(block_id.0 as usize).and_then(|b|b.ops.last()).map(IrOp::is_control_flow).unwrap_or(false);
        if !last_is_cf {
            builder.flush_guest_state(block_id);
            builder.emit_set_pc(block_id, VOperand::Imm(cursor_pc as i64));
            builder.push(block_id, IrOp::Return);
        }
        let mut ir_block=builder.blocks.pop().expect("block");
        passes::run_all(&mut ir_block);
        let allocation=regalloc::linear_scan(&ir_block);
        let compiled=match crate::core::jit_ffi::emit_block(&ir_block,&allocation,self.host_caps,self.force_baseline) {
            Ok(b)=>b, Err(st)=>panic!("jit_emit_block failed at {guest_pc:#x}: {st:?}"),
        };
        self.cache.insert(guest_pc, cursor_pc, compiled);
    }
    unsafe fn execute_block(block:&CompiledBlock, cpu:&mut CpuState)->u64 {
        let f: extern "C" fn(*mut CpuState)->u64 = std::mem::transmute(block.code_ptr);
        f(cpu as *mut CpuState)
    }
}
