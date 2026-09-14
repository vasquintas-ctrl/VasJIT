#pragma once
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct CHostCaps { uint32_t bits; } CHostCaps;
#define CHOSTCAPS_SSE2 (1u<<0)
#define CHOSTCAPS_SSE4_1 (1u<<1)
#define CHOSTCAPS_AVX (1u<<2)
#define CHOSTCAPS_AVX2 (1u<<3)
#define CHOSTCAPS_FMA (1u<<4)
#define CHOSTCAPS_BMI2 (1u<<5)
#define CHOSTCAPS_MOVBE (1u<<6)
#define CHOSTCAPS_AVX512F (1u<<7)

typedef enum CWidth { CWidth_W8=0,CWidth_W16,CWidth_W32,CWidth_W64,CWidth_W128 } CWidth;
typedef enum CEndian { CEndian_Little=0,CEndian_Big } CEndian;
typedef enum CIrOpKind {
  CIrOpKind_Add=0,CIrOpKind_Sub,CIrOpKind_And,CIrOpKind_Or,CIrOpKind_Xor,CIrOpKind_Mul,
  CIrOpKind_Shl,CIrOpKind_Shr,CIrOpKind_Sar,CIrOpKind_Load,CIrOpKind_Store,
  CIrOpKind_SetFlags,CIrOpKind_ReadFlag,CIrOpKind_Branch,CIrOpKind_IndirectBranch,
  CIrOpKind_Call,CIrOpKind_Return,CIrOpKind_VecAdd,CIrOpKind_VecMul,CIrOpKind_VecFma,CIrOpKind_Intrinsic
} CIrOpKind;
typedef enum CFlagOp { CFlagOp_AddOp=0,CFlagOp_SubOp,CFlagOp_AndOp,CFlagOp_OrOp,CFlagOp_XorOp,CFlagOp_TestOp } CFlagOp;
typedef enum CFlagKind { CFlagKind_Zero=0,CFlagKind_Carry,CFlagKind_Overflow,CFlagKind_Negative } CFlagKind;
typedef enum COperandKind { COperandKind_Reg=0,COperandKind_Imm } COperandKind;
typedef enum CLocationKind { CLocationKind_Reg=0,CLocationKind_Spill } CLocationKind;
typedef enum CHostGpr {
  CHostGpr_Rax=0,CHostGpr_Rbx,CHostGpr_Rcx,CHostGpr_Rdx,CHostGpr_Rsi,CHostGpr_Rdi,
  CHostGpr_R8,CHostGpr_R9,CHostGpr_R10,CHostGpr_R11,CHostGpr_R12,CHostGpr_R13,CHostGpr_R14
} CHostGpr;
typedef enum CJitStatus {
  CJitStatus_Ok=0,CJitStatus_InvalidInput,CJitStatus_OutOfMemory,CJitStatus_EmitFailed,CJitStatus_IntrinsicOverflow
} CJitStatus;

typedef struct COperand { COperandKind kind; uint32_t reg; int64_t imm; } COperand;
#define CIR_MAX_OPERANDS 8
typedef struct CIrOp {
  CIrOpKind kind; CWidth width; CEndian endian; uint8_t sign_ext; uint8_t dst_valid;
  uint32_t dst; uint32_t num_operands; COperand operands[CIR_MAX_OPERANDS];
  CFlagOp flag_op; CFlagKind flag_kind; uint32_t intrinsic_id; uint8_t lanes;
} CIrOp;
typedef struct CLocation { CLocationKind kind; CHostGpr gpr; uint32_t offset; } CLocation;
typedef struct CAllocEntry { uint32_t vreg; CLocation loc; } CAllocEntry;
typedef struct CIrBlock {
  uint64_t guest_pc; const CIrOp* ops; uint32_t num_ops;
  const CAllocEntry* allocs; uint32_t num_allocs;
} CIrBlock;
typedef struct CPatchSite { uint32_t code_offset; uint64_t target_guest_pc; } CPatchSite;
typedef struct CEmitResult {
  uint8_t* code; size_t code_len; CPatchSite* patch_sites; uint32_t num_patch_sites;
} CEmitResult;

#define CPUSTATE_OFF_GPR 0
#define CPUSTATE_OFF_FPR 256
#define CPUSTATE_OFF_VEC 512
#define CPUSTATE_OFF_PC 1024
#define CPUSTATE_OFF_FLAGS_Z 1032
#define CPUSTATE_OFF_FLAGS_C 1033
#define CPUSTATE_OFF_FLAGS_O 1034
#define CPUSTATE_OFF_FLAGS_N 1035
#define CPUSTATE_OFF_SPILL 1040
#define CPUSTATE_OFF_CR 2064
#define CPUSTATE_OFF_LR 2072
#define CPUSTATE_OFF_CTR 2080
#define CPUSTATE_OFF_XER 2088
#define CPUSTATE_OFF_MSR 2096
#define CPUSTATE_OFF_FPSCR 2104
#define CPUSTATE_OFF_GQR 2112
#define CPUSTATE_OFF_MEM_BASE 2176
#define CPUSTATE_OFF_JIT_TMP0 2184
#define CPUSTATE_OFF_JIT_TMP1 2192

#define INTRIN_GET_GPR 1
#define INTRIN_SET_GPR 2
#define INTRIN_GET_FPR 3
#define INTRIN_SET_FPR 4
#define INTRIN_GET_VEC 5
#define INTRIN_SET_VEC 6
#define INTRIN_GET_CR 7
#define INTRIN_SET_CR 8
#define INTRIN_GET_LR 9
#define INTRIN_SET_LR 10
#define INTRIN_GET_CTR 11
#define INTRIN_SET_CTR 12
#define INTRIN_GET_XER 13
#define INTRIN_SET_XER 14
#define INTRIN_GET_MSR 15
#define INTRIN_SET_MSR 16
#define INTRIN_GET_FPSCR 17
#define INTRIN_SET_FPSCR 18
#define INTRIN_SET_PC 19
#define INTRIN_GET_CR_BIT 20
#define INTRIN_SET_CR_FIELD 21
#define INTRIN_GET_SPR 22
#define INTRIN_SET_SPR 23
#define INTRIN_GET_SP 24
#define INTRIN_CACHE_OP 25
#define INTRIN_SYNC 26
#define INTRIN_ISYNC 27
#define INTRIN_DCBZ 28
#define INTRIN_TRAP 29
#define INTRIN_SC 30
#define INTRIN_SVC 31
#define INTRIN_RFI 32
#define INTRIN_GQR_LOAD 33
#define INTRIN_GQR_STORE 34
#define INTRIN_SYSREG 35
#define INTRIN_UNIMPLEMENTED 36

CHostCaps jit_detect_host_caps(void);
CJitStatus jit_emit_block(const CIrBlock* block, CHostCaps caps, uint8_t force_baseline, CEmitResult* out);
void jit_free_code(uint8_t* code);
CJitStatus jit_patch_chain(uint8_t* code, size_t code_len, uint32_t patch_offset, const uint8_t* target_entry);
#ifdef __cplusplus
}
#endif
