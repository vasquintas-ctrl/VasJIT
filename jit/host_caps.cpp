#include "jit_abi.h"
#include <cpuid.h>
extern "C" CHostCaps jit_detect_host_caps(void) {
  CHostCaps c; c.bits = CHOSTCAPS_SSE2;
  unsigned eax=0,ebx=0,ecx=0,edx=0;
  if (!__get_cpuid(1,&eax,&ebx,&ecx,&edx)) return c;
  if (ecx & (1u<<19)) c.bits |= CHOSTCAPS_SSE4_1;
  if (ecx & (1u<<28)) c.bits |= CHOSTCAPS_AVX;
  if (ecx & (1u<<12)) c.bits |= CHOSTCAPS_FMA;
  if (ecx & (1u<<22)) c.bits |= CHOSTCAPS_MOVBE;
  unsigned max_leaf=0,b2=0,c2=0,d2=0;
  if (__get_cpuid(0,&max_leaf,&b2,&c2,&d2) && max_leaf>=7) {
    unsigned a7=0,b7=0,c7=0,d7=0;
    __cpuid_count(7,0,a7,b7,c7,d7);
    if (b7 & (1u<<5)) c.bits |= CHOSTCAPS_AVX2;
    if (b7 & (1u<<8)) c.bits |= CHOSTCAPS_BMI2;
    if (b7 & (1u<<16)) c.bits |= CHOSTCAPS_AVX512F;
  }
  return c;
}
