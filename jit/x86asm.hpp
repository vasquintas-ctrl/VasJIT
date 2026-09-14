#pragma once
#include <cstdint>
#include <vector>
struct X86Buf {
  std::vector<uint8_t> bytes;
  size_t size() const { return bytes.size(); }
  uint8_t* data() { return bytes.data(); }
  void emit8(uint8_t v){ bytes.push_back(v); }
  void emit32(uint32_t v){ emit8(v); emit8(v>>8); emit8(v>>16); emit8(v>>24); }
  void emit64(uint64_t v){ emit32((uint32_t)v); emit32((uint32_t)(v>>32)); }
  void rex(bool w,uint8_t r,uint8_t x,uint8_t b){
    uint8_t p=0x40; if(w)p|=8; if(r&8)p|=4; if(x&8)p|=2; if(b&8)p|=1;
    if(p!=0x40||(r&8)||(b&8)||w) emit8(p);
  }
  void modrm(uint8_t mod,uint8_t reg,uint8_t rm){ emit8((mod<<6)|((reg&7)<<3)|(rm&7)); }
  void mov_rr(uint8_t d,uint8_t s,bool w=true){ rex(w,s,0,d); emit8(0x89); modrm(3,s,d); }
  void mov_rr32(uint8_t d,uint8_t s){ mov_rr(d,s,false); }
  void add_rr(uint8_t d,uint8_t s,bool w=true){ rex(w,s,0,d); emit8(0x01); modrm(3,s,d); }
  void sub_rr(uint8_t d,uint8_t s,bool w=true){ rex(w,s,0,d); emit8(0x29); modrm(3,s,d); }
  void and_rr(uint8_t d,uint8_t s,bool w=true){ rex(w,s,0,d); emit8(0x21); modrm(3,s,d); }
  void or_rr(uint8_t d,uint8_t s,bool w=true){ rex(w,s,0,d); emit8(0x09); modrm(3,s,d); }
  void xor_rr(uint8_t d,uint8_t s,bool w=true){ rex(w,s,0,d); emit8(0x31); modrm(3,s,d); }
  void test_rr(uint8_t d,uint8_t s,bool w=true){ rex(w,s,0,d); emit8(0x85); modrm(3,s,d); }
  void imul_rr(uint8_t d,uint8_t s,bool w=true){ rex(w,d,0,s); emit8(0x0F); emit8(0xAF); modrm(3,d,s); }
  void shl_cl(uint8_t d,bool w=true){ rex(w,0,0,d); emit8(0xD3); modrm(3,4,d); }
  void shr_cl(uint8_t d,bool w=true){ rex(w,0,0,d); emit8(0xD3); modrm(3,5,d); }
  void sar_cl(uint8_t d,bool w=true){ rex(w,0,0,d); emit8(0xD3); modrm(3,7,d); }
  void shl_imm(uint8_t d,uint8_t i,bool w=true){ rex(w,0,0,d); emit8(0xC1); modrm(3,4,d); emit8(i); }
  void shr_imm(uint8_t d,uint8_t i,bool w=true){ rex(w,0,0,d); emit8(0xC1); modrm(3,5,d); emit8(i); }
  void sar_imm(uint8_t d,uint8_t i,bool w=true){ rex(w,0,0,d); emit8(0xC1); modrm(3,7,d); emit8(i); }
  void movzx_r8(uint8_t d,uint8_t s){ rex(true,d,0,s); emit8(0x0F); emit8(0xB6); modrm(3,d,s); }
  void bswap32(uint8_t r){ rex(false,0,0,r); emit8(0x0F); emit8(0xC8+(r&7)); }
  void movabs(uint8_t d,uint64_t imm){ rex(true,0,0,d); emit8(0xB8+(d&7)); emit64(imm); }
  void mov_ri32(uint8_t d,uint32_t imm){ rex(false,0,0,d); emit8(0xB8+(d&7)); emit32(imm); }
  void alu_imm(uint8_t d,uint32_t imm,uint8_t op,bool w=true){ rex(w,0,0,d); emit8(0x81); modrm(3,op,d); emit32(imm); }
  void add_ri(uint8_t d,int32_t i,bool w=true){ alu_imm(d,(uint32_t)i,0,w); }
  void or_ri(uint8_t d,int32_t i,bool w=true){ alu_imm(d,(uint32_t)i,1,w); }
  void and_ri(uint8_t d,int32_t i,bool w=true){ alu_imm(d,(uint32_t)i,4,w); }
  void sub_ri(uint8_t d,int32_t i,bool w=true){ alu_imm(d,(uint32_t)i,5,w); }
  void xor_ri(uint8_t d,int32_t i,bool w=true){ alu_imm(d,(uint32_t)i,6,w); }
  void modrm_r15(uint8_t reg,int32_t disp){ modrm(2,reg,7); emit32((uint32_t)disp); }
  void mov_load_r15(uint8_t d,int32_t disp,bool w=true){ rex(w,d,0,15); emit8(0x8B); modrm_r15(d,disp); }
  void mov_store_r15(uint8_t s,int32_t disp,bool w=true){ rex(w,s,0,15); emit8(0x89); modrm_r15(s,disp); }
  void mov_load8_r15(uint8_t d,int32_t disp){ rex(true,d,0,15); emit8(0x0F); emit8(0xB6); modrm_r15(d,disp); }
  void mov_store8_r15(uint8_t s,int32_t disp){ rex(false,s,0,15); emit8(0x88); modrm_r15(s,disp); }
  void emit_mem(uint8_t reg,uint8_t base,int32_t disp){
    uint8_t rm=base&7;
    if(rm==4){ modrm(2,reg,4); emit8(0x24); emit32((uint32_t)disp); return; }
    modrm(2,reg,rm); emit32((uint32_t)disp);
  }
  void mov_load(uint8_t d,uint8_t base,int32_t disp,bool w=true){ rex(w,d,0,base); emit8(0x8B); emit_mem(d,base,disp); }
  void mov_store(uint8_t s,uint8_t base,int32_t disp,bool w=true){ rex(w,s,0,base); emit8(0x89); emit_mem(s,base,disp); }
  void movzx8_load(uint8_t d,uint8_t base,int32_t disp){ rex(true,d,0,base); emit8(0x0F); emit8(0xB6); emit_mem(d,base,disp); }
  void movzx16_load(uint8_t d,uint8_t base,int32_t disp){ rex(true,d,0,base); emit8(0x0F); emit8(0xB7); emit_mem(d,base,disp); }
  void movsx8_load(uint8_t d,uint8_t base,int32_t disp){ rex(true,d,0,base); emit8(0x0F); emit8(0xBE); emit_mem(d,base,disp); }
  void movsx16_load(uint8_t d,uint8_t base,int32_t disp){ rex(true,d,0,base); emit8(0x0F); emit8(0xBF); emit_mem(d,base,disp); }
  void load32(uint8_t d,uint8_t base,int32_t disp){ rex(false,d,0,base); emit8(0x8B); emit_mem(d,base,disp); }
  void store8(uint8_t s,uint8_t base,int32_t disp){ rex(false,s,0,base); emit8(0x88); emit_mem(s,base,disp); }
  void store16(uint8_t s,uint8_t base,int32_t disp){ emit8(0x66); rex(false,s,0,base); emit8(0x89); emit_mem(s,base,disp); }
  void store32(uint8_t s,uint8_t base,int32_t disp){ rex(false,s,0,base); emit8(0x89); emit_mem(s,base,disp); }
  void push(uint8_t r){ if(r&8)emit8(0x41); emit8(0x50+(r&7)); }
  void pop(uint8_t r){ if(r&8)emit8(0x41); emit8(0x58+(r&7)); }
  void ret(){ emit8(0xC3); }
  size_t jmp_rel32(){ emit8(0xE9); size_t o=size(); emit32(0); return o; }
  size_t jz_rel32(){ emit8(0x0F); emit8(0x84); size_t o=size(); emit32(0); return o; }
  void patch_rel32(size_t off,size_t target){
    int32_t rel=(int32_t)((int64_t)target-(int64_t)(off+4));
    bytes[off]=rel; bytes[off+1]=rel>>8; bytes[off+2]=rel>>16; bytes[off+3]=rel>>24;
  }
  void setz(uint8_t r){ rex(false,0,0,r); emit8(0x0F); emit8(0x94); modrm(3,0,r); movzx_r8(r,r); }
  void sets(uint8_t r){ rex(false,0,0,r); emit8(0x0F); emit8(0x98); modrm(3,0,r); movzx_r8(r,r); }
  void setc(uint8_t r){ rex(false,0,0,r); emit8(0x0F); emit8(0x92); modrm(3,0,r); movzx_r8(r,r); }
  void seto(uint8_t r){ rex(false,0,0,r); emit8(0x0F); emit8(0x90); modrm(3,0,r); movzx_r8(r,r); }
  void movq_xmm_r(uint8_t x,uint8_t r){ emit8(0x66); rex(true,x,0,r); emit8(0x0F); emit8(0x6E); modrm(3,x,r); }
  void movq_r_xmm(uint8_t r,uint8_t x){ emit8(0x66); rex(true,x,0,r); emit8(0x0F); emit8(0x7E); modrm(3,x,r); }
  void addps(uint8_t d,uint8_t s){ rex(false,d,0,s); emit8(0x0F); emit8(0x58); modrm(3,d,s); }
  void mulps(uint8_t d,uint8_t s){ rex(false,d,0,s); emit8(0x0F); emit8(0x59); modrm(3,d,s); }
  void movzx32(uint8_t d,uint8_t s){ mov_rr32(d,s); }
};
