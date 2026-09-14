#include "jit_abi.h"
#include "strategy.hpp"
#include "x86asm.hpp"
#include <sys/mman.h>
#include <unistd.h>
#include <cstring>
#include <new>
#include <vector>

static const uint8_t kMap[13]={0,3,1,2,6,7,8,9,10,11,12,13,14};
static constexpr uint8_t RAX=0,RCX=1,RDX=2,RBX=3,RDI=7,R15=15;

struct AllocHdr { size_t total, code_off, code_len; uint32_t n_patch; };
struct Loc { bool in_reg; uint8_t gpr; uint32_t spill; };

static Loc lookup(uint32_t vreg, const CAllocEntry* allocs, uint32_t n) {
  for (uint32_t i=0;i<n;i++) if (allocs[i].vreg==vreg) {
    Loc l{};
    if (allocs[i].loc.kind==CLocationKind_Reg){ l.in_reg=true; l.gpr=kMap[(int)allocs[i].loc.gpr]; }
    else { l.in_reg=false; l.spill=allocs[i].loc.offset; }
    return l;
  }
  Loc m{}; m.in_reg=true; m.gpr=RAX; return m;
}

struct Emitter {
  X86Buf b; CodegenStrategy st; const CIrBlock* block; std::vector<CPatchSite> patches;
  Emitter(const CIrBlock* blk, CodegenStrategy s): st(s), block(blk) {}
  bool w64(CWidth w) const { return w==CWidth_W64||w==CWidth_W128; }
  void load_loc(uint8_t d, Loc l){
    if(l.in_reg){ if(d!=l.gpr) b.mov_rr(d,l.gpr,true); }
    else b.mov_load_r15(d,(int32_t)(CPUSTATE_OFF_SPILL+l.spill),true);
  }
  void store_loc(Loc l, uint8_t s){
    if(l.in_reg){ if(s!=l.gpr) b.mov_rr(l.gpr,s,true); }
    else b.mov_store_r15(s,(int32_t)(CPUSTATE_OFF_SPILL+l.spill),true);
  }
  uint8_t scratch_not(uint8_t a){ return a==RAX?RCX:RAX; }
  void park(uint8_t r,int w){ b.mov_store_r15(r,w?CPUSTATE_OFF_JIT_TMP1:CPUSTATE_OFF_JIT_TMP0,true); }
  void unpark(uint8_t r,int w){ b.mov_load_r15(r,w?CPUSTATE_OFF_JIT_TMP1:CPUSTATE_OFF_JIT_TMP0,true); }
  void materialize(const COperand& op, uint8_t d, bool wide){
    if(op.kind==COperandKind_Imm){ if(wide) b.movabs(d,(uint64_t)op.imm); else b.mov_ri32(d,(uint32_t)op.imm); }
    else { Loc l=lookup(op.reg,block->allocs,block->num_allocs); load_loc(d,l); if(!wide) b.movzx32(d,d); }
  }
  uint8_t dest_reg(const CIrOp& op){
    if(!op.dst_valid) return RAX;
    Loc l=lookup(op.dst,block->allocs,block->num_allocs);
    return l.in_reg?l.gpr:RAX;
  }
  void commit_dst(const CIrOp& op, uint8_t s){
    if(!op.dst_valid) return;
    store_loc(lookup(op.dst,block->allocs,block->num_allocs),s);
  }
  void emit_binop(const CIrOp& op, int alu){
    bool wide=w64(op.width); uint8_t d=dest_reg(op);
    materialize(op.operands[0],d,wide);
    const COperand& bop=op.operands[1];
    if(bop.kind==COperandKind_Imm && bop.imm==(int32_t)bop.imm) b.alu_imm(d,(uint32_t)bop.imm,(uint8_t)alu,wide);
    else {
      uint8_t s=scratch_not(d); park(s,0); materialize(bop,s,wide);
      if(alu==0) b.add_rr(d,s,wide); else if(alu==1) b.or_rr(d,s,wide);
      else if(alu==4) b.and_rr(d,s,wide); else if(alu==5) b.sub_rr(d,s,wide);
      else b.xor_rr(d,s,wide);
      unpark(s,0);
    }
    if(!wide) b.movzx32(d,d); commit_dst(op,d);
  }
  void emit_shift(const CIrOp& op, int kind){
    bool wide=w64(op.width); uint8_t d=dest_reg(op);
    materialize(op.operands[0],d,wide);
    const COperand& amt=op.operands[1];
    if(amt.kind==COperandKind_Imm){
      uint8_t n=(uint8_t)(amt.imm&63);
      if(kind==0) b.shl_imm(d,n,wide); else if(kind==1) b.shr_imm(d,n,wide); else b.sar_imm(d,n,wide);
    } else {
      park(RCX,0); uint8_t work=d;
      if(d==RCX){ work=RAX; park(RAX,1); b.mov_rr(RAX,RCX,true); }
      materialize(amt,RCX,false);
      if(kind==0) b.shl_cl(work,wide); else if(kind==1) b.shr_cl(work,wide); else b.sar_cl(work,wide);
      if(d==RCX){ b.mov_rr(RCX,work,true); unpark(RAX,1); } else unpark(RCX,0);
    }
    if(!wide) b.movzx32(d,d); commit_dst(op,d);
  }
  void emit_load(const CIrOp& op){
    uint8_t d=dest_reg(op); materialize(op.operands[0],d,true); b.movzx32(d,d);
    uint8_t s=scratch_not(d); park(s,0); b.mov_load_r15(s,CPUSTATE_OFF_MEM_BASE,true); b.add_rr(d,s,true); unpark(s,0);
    bool be=op.endian==CEndian_Big;
    switch(op.width){
    case CWidth_W8: if(op.sign_ext) b.movsx8_load(d,d,0); else b.movzx8_load(d,d,0); break;
    case CWidth_W16:
      if(op.sign_ext) b.movsx16_load(d,d,0); else b.movzx16_load(d,d,0);
      if(be){ b.bswap32(d); b.shr_imm(d,16,false); }
      break;
    case CWidth_W32: b.load32(d,d,0); if(be) b.bswap32(d); b.movzx32(d,d); break;
    default: b.mov_load(d,d,0,true); break;
    }
    commit_dst(op,d);
  }
  void emit_store(const CIrOp& op){
    materialize(op.operands[0],RAX,true); b.movzx32(RAX,RAX);
    materialize(op.operands[1],RCX,true);
    b.mov_load_r15(RDX,CPUSTATE_OFF_MEM_BASE,true); b.add_rr(RAX,RDX,true);
    bool be=op.endian==CEndian_Big;
    switch(op.width){
    case CWidth_W8: b.store8(RCX,RAX,0); break;
    case CWidth_W16:
      if(be){ b.emit8(0x66); b.emit8(0xC1); b.modrm(3,1,RCX); b.emit8(8); }
      b.store16(RCX,RAX,0); break;
    case CWidth_W32: if(be) b.bswap32(RCX); b.store32(RCX,RAX,0); break;
    default: b.mov_store(RCX,RAX,0,true); break;
    }
  }
  void emit_setflags(const CIrOp& op){
    bool wide=w64(op.width);
    materialize(op.operands[0],RAX,wide); materialize(op.operands[1],RCX,wide);
    switch(op.flag_op){
    case CFlagOp_AddOp: b.add_rr(RAX,RCX,wide); break;
    case CFlagOp_SubOp: b.sub_rr(RAX,RCX,wide); break;
    case CFlagOp_AndOp: b.and_rr(RAX,RCX,wide); break;
    case CFlagOp_OrOp:  b.or_rr(RAX,RCX,wide); break;
    case CFlagOp_XorOp: b.xor_rr(RAX,RCX,wide); break;
    default: b.test_rr(RAX,RAX,wide); break;
    }
    b.setz(RDX); b.mov_store8_r15(RDX,CPUSTATE_OFF_FLAGS_Z);
    b.sets(RDX); b.mov_store8_r15(RDX,CPUSTATE_OFF_FLAGS_N);
    b.setc(RDX); b.mov_store8_r15(RDX,CPUSTATE_OFF_FLAGS_C);
    b.seto(RDX); b.mov_store8_r15(RDX,CPUSTATE_OFF_FLAGS_O);
  }
  void emit_readflag(const CIrOp& op){
    uint8_t d=dest_reg(op); int32_t off=CPUSTATE_OFF_FLAGS_Z;
    if(op.flag_kind==CFlagKind_Carry) off=CPUSTATE_OFF_FLAGS_C;
    else if(op.flag_kind==CFlagKind_Overflow) off=CPUSTATE_OFF_FLAGS_O;
    else if(op.flag_kind==CFlagKind_Negative) off=CPUSTATE_OFF_FLAGS_N;
    b.mov_load8_r15(d,off); commit_dst(op,d);
  }
  void emit_branch(const CIrOp& op){
    int n=(int)op.num_operands; if(n<1) return;
    COperand taken{}, fall{}; bool has_cond=false; COperand cond{};
    if(n>=2 && op.operands[n-1].kind==COperandKind_Imm && op.operands[n-2].kind==COperandKind_Imm){
      taken=op.operands[n-2]; fall=op.operands[n-1];
      if(n==3){ cond=op.operands[0]; has_cond=true; }
    } else taken=op.operands[0];
    auto set_pc=[&](int64_t pc){ b.movabs(RAX,(uint64_t)pc); b.mov_store_r15(RAX,CPUSTATE_OFF_PC,true); };
    if(has_cond){
      materialize(cond,RAX,true); b.test_rr(RAX,RAX,true);
      size_t jz=b.jz_rel32(); set_pc(taken.imm); size_t jt=b.jmp_rel32();
      patches.push_back(CPatchSite{(uint32_t)jt,(uint64_t)taken.imm});
      b.patch_rel32(jz,b.size()); set_pc(fall.imm); size_t jf=b.jmp_rel32();
      patches.push_back(CPatchSite{(uint32_t)jf,(uint64_t)fall.imm});
    } else {
      set_pc(taken.imm); size_t jt=b.jmp_rel32();
      patches.push_back(CPatchSite{(uint32_t)jt,(uint64_t)taken.imm});
    }
  }
  void emit_indirect(const CIrOp& op){
    materialize(op.operands[0],RAX,true); b.mov_store_r15(RAX,CPUSTATE_OFF_PC,true);
    size_t j=b.jmp_rel32(); patches.push_back(CPatchSite{(uint32_t)j,0});
  }
  void emit_intrinsic(const CIrOp& op){
    auto og=[](int64_t n){return (int32_t)(CPUSTATE_OFF_GPR+n*8);};
    auto of=[](int64_t n){return (int32_t)(CPUSTATE_OFF_FPR+n*8);};
    auto ov=[](int64_t n){return (int32_t)(CPUSTATE_OFF_VEC+n*16);};
    switch(op.intrinsic_id){
    case INTRIN_GET_GPR:{ uint8_t d=dest_reg(op); b.mov_load_r15(d,og(op.operands[0].imm),true); commit_dst(op,d); break; }
    case INTRIN_SET_GPR:{ materialize(op.operands[1],RAX,true); b.mov_store_r15(RAX,og(op.operands[0].imm),true); break; }
    case INTRIN_GET_FPR:{ uint8_t d=dest_reg(op); b.mov_load_r15(d,of(op.operands[0].imm),true); commit_dst(op,d); break; }
    case INTRIN_SET_FPR:{ materialize(op.operands[1],RAX,true); b.mov_store_r15(RAX,of(op.operands[0].imm),true); break; }
    case INTRIN_GET_VEC:{ uint8_t d=dest_reg(op); b.mov_load_r15(d,ov(op.operands[0].imm),true); commit_dst(op,d); break; }
    case INTRIN_SET_VEC:{ materialize(op.operands[1],RAX,true); b.mov_store_r15(RAX,ov(op.operands[0].imm),true); break; }
    case INTRIN_GET_CR:{ uint8_t d=dest_reg(op); b.mov_load_r15(d,CPUSTATE_OFF_CR,true); commit_dst(op,d); break; }
    case INTRIN_SET_CR:{ materialize(op.operands[0],RAX,true); b.mov_store_r15(RAX,CPUSTATE_OFF_CR,true); break; }
    case INTRIN_GET_LR:{ uint8_t d=dest_reg(op); b.mov_load_r15(d,CPUSTATE_OFF_LR,true); commit_dst(op,d); break; }
    case INTRIN_SET_LR:{ materialize(op.operands[0],RAX,true); b.mov_store_r15(RAX,CPUSTATE_OFF_LR,true); break; }
    case INTRIN_GET_CTR:{ uint8_t d=dest_reg(op); b.mov_load_r15(d,CPUSTATE_OFF_CTR,true); commit_dst(op,d); break; }
    case INTRIN_SET_CTR:{ materialize(op.operands[0],RAX,true); b.mov_store_r15(RAX,CPUSTATE_OFF_CTR,true); break; }
    case INTRIN_GET_XER:{ uint8_t d=dest_reg(op); b.mov_load_r15(d,CPUSTATE_OFF_XER,true); commit_dst(op,d); break; }
    case INTRIN_SET_XER:{ materialize(op.operands[0],RAX,true); b.mov_store_r15(RAX,CPUSTATE_OFF_XER,true); break; }
    case INTRIN_SET_PC:{ materialize(op.operands[0],RAX,true); b.mov_store_r15(RAX,CPUSTATE_OFF_PC,true); break; }
    case INTRIN_GET_CR_BIT:{
      uint8_t d=dest_reg(op); int bit=(int)op.operands[0].imm;
      b.mov_load_r15(d,CPUSTATE_OFF_CR,true); int host=31-bit;
      if(host>0) b.shr_imm(d,(uint8_t)host,true); b.and_ri(d,1,true); commit_dst(op,d); break;
    }
    case INTRIN_SET_CR_FIELD:{
      int field=(int)op.operands[0].imm; materialize(op.operands[1],RCX,true); b.and_ri(RCX,0xf,true);
      int shift=(7-field)*4; if(shift) b.shl_imm(RCX,(uint8_t)shift,true);
      b.mov_load_r15(RAX,CPUSTATE_OFF_CR,true); b.and_ri(RAX,(int32_t)~(0xf<<shift),true);
      b.or_rr(RAX,RCX,true); b.mov_store_r15(RAX,CPUSTATE_OFF_CR,true); break;
    }
    default:
      if(op.dst_valid){ uint8_t d=dest_reg(op); b.xor_rr(d,d,true); commit_dst(op,d); }
      break;
    }
  }
  void emit_vec(const CIrOp& op, bool mul){
    uint8_t d=dest_reg(op); materialize(op.operands[0],d,true); materialize(op.operands[1],RCX,true);
    b.movq_xmm_r(0,d); b.movq_xmm_r(1,RCX);
    if(mul) b.mulps(0,1); else b.addps(0,1);
    b.movq_r_xmm(d,0); commit_dst(op,d);
  }
  void emit_op(const CIrOp& op){
    switch(op.kind){
    case CIrOpKind_Add: emit_binop(op,0); break;
    case CIrOpKind_Sub: emit_binop(op,5); break;
    case CIrOpKind_And: emit_binop(op,4); break;
    case CIrOpKind_Or:  emit_binop(op,1); break;
    case CIrOpKind_Xor: emit_binop(op,6); break;
    case CIrOpKind_Mul:{
      bool wide=w64(op.width); uint8_t d=dest_reg(op); materialize(op.operands[0],d,wide);
      uint8_t s=scratch_not(d); park(s,0); materialize(op.operands[1],s,wide);
      b.imul_rr(d,s,wide); unpark(s,0); if(!wide) b.movzx32(d,d); commit_dst(op,d); break;
    }
    case CIrOpKind_Shl: emit_shift(op,0); break;
    case CIrOpKind_Shr: emit_shift(op,1); break;
    case CIrOpKind_Sar: emit_shift(op,2); break;
    case CIrOpKind_Load: emit_load(op); break;
    case CIrOpKind_Store: emit_store(op); break;
    case CIrOpKind_SetFlags: emit_setflags(op); break;
    case CIrOpKind_ReadFlag: emit_readflag(op); break;
    case CIrOpKind_Branch: emit_branch(op); break;
    case CIrOpKind_IndirectBranch: case CIrOpKind_Call: emit_indirect(op); break;
    case CIrOpKind_Return: break;
    case CIrOpKind_VecAdd: emit_vec(op,false); break;
    case CIrOpKind_VecMul: emit_vec(op,true); break;
    case CIrOpKind_VecFma:{
      emit_vec(op,true); uint8_t d=dest_reg(op); materialize(op.operands[2],RCX,true);
      b.movq_xmm_r(0,d); b.movq_xmm_r(1,RCX); b.addps(0,1); b.movq_r_xmm(d,0); commit_dst(op,d); break;
    }
    case CIrOpKind_Intrinsic: emit_intrinsic(op); break;
    }
  }
};

static void emit_prologue(X86Buf& b){
  b.push(RBX); b.push(5); b.push(12); b.push(13); b.push(14); b.push(R15);
  b.mov_rr(R15,RDI,true);
}
static void emit_epilogue(X86Buf& b){
  b.mov_load_r15(RAX,CPUSTATE_OFF_PC,true);
  b.pop(R15); b.pop(14); b.pop(13); b.pop(12); b.pop(5); b.pop(RBX); b.ret();
}

extern "C" CJitStatus jit_emit_block(const CIrBlock* block, CHostCaps caps, uint8_t force_baseline, CEmitResult* out){
  if(!block||!out) return CJitStatus_InvalidInput;
  std::memset(out,0,sizeof(*out));
  CodegenStrategy st=resolve_strategy(caps,force_baseline);
  Emitter e(block,st); emit_prologue(e.b);
  std::vector<size_t> local_jumps;
  for(uint32_t i=0;i<block->num_ops;i++){
    if(block->ops[i].kind==CIrOpKind_Return){ local_jumps.push_back(e.b.jmp_rel32()); continue; }
    e.emit_op(block->ops[i]);
  }
  local_jumps.push_back(e.b.jmp_rel32());
  size_t epi=e.b.size(); emit_epilogue(e.b);
  for(size_t off: local_jumps) e.b.patch_rel32(off,epi);
  for(auto& p: e.patches) e.b.patch_rel32(p.code_offset,epi);

  size_t code_len=e.b.size(); size_t n_patch=e.patches.size();
  size_t hdr=sizeof(AllocHdr); size_t patch_bytes=n_patch*sizeof(CPatchSite);
  size_t patch_off=(hdr+code_len+7)&~size_t(7);
  size_t total=patch_off+patch_bytes;
  long page=sysconf(_SC_PAGESIZE); total=(total+page-1)&~(size_t)(page-1);
  void* mem=mmap(nullptr,total,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
  if(mem==MAP_FAILED) return CJitStatus_OutOfMemory;
  auto* h=new (mem) AllocHdr(); h->total=total; h->code_off=hdr; h->code_len=code_len; h->n_patch=(uint32_t)n_patch;
  uint8_t* code=(uint8_t*)mem+hdr; std::memcpy(code,e.b.data(),code_len);
  CPatchSite* ps=(CPatchSite*)((uint8_t*)mem+patch_off);
  if(n_patch) std::memcpy(ps,e.patches.data(),patch_bytes);
  if(mprotect(mem,total,PROT_READ|PROT_EXEC)!=0){ munmap(mem,total); return CJitStatus_EmitFailed; }
  out->code=code; out->code_len=code_len; out->num_patch_sites=(uint32_t)n_patch; out->patch_sites=ps;
  return CJitStatus_Ok;
}
extern "C" void jit_free_code(uint8_t* code){
  if(!code) return; AllocHdr* h=(AllocHdr*)(code-sizeof(AllocHdr)); munmap(h,h->total);
}
extern "C" CJitStatus jit_patch_chain(uint8_t* code, size_t code_len, uint32_t patch_offset, const uint8_t* target_entry){
  if(!code||patch_offset+4>code_len) return CJitStatus_InvalidInput;
  AllocHdr* h=(AllocHdr*)(code-sizeof(AllocHdr));
  if(mprotect(h,h->total,PROT_READ|PROT_WRITE)!=0) return CJitStatus_EmitFailed;
  int32_t rel=(int32_t)((intptr_t)target_entry-(intptr_t)(code+patch_offset+4));
  code[patch_offset]=(uint8_t)rel; code[patch_offset+1]=(uint8_t)(rel>>8);
  code[patch_offset+2]=(uint8_t)(rel>>16); code[patch_offset+3]=(uint8_t)(rel>>24);
  if(mprotect(h,h->total,PROT_READ|PROT_EXEC)!=0) return CJitStatus_EmitFailed;
  return CJitStatus_Ok;
}
