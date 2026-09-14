#include "strategy.hpp"
CodegenStrategy resolve_strategy(CHostCaps caps, uint8_t force_baseline) {
  CodegenStrategy s{};
  if (force_baseline) return s;
  s.use_movbe = (caps.bits & CHOSTCAPS_MOVBE) != 0;
  s.use_avx2  = (caps.bits & CHOSTCAPS_AVX2) != 0;
  s.use_fma   = (caps.bits & CHOSTCAPS_FMA) != 0;
  s.use_bmi2  = (caps.bits & CHOSTCAPS_BMI2) != 0;
  return s;
}
