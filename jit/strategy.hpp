#pragma once
#include "jit_abi.h"
struct CodegenStrategy { bool use_movbe, use_avx2, use_fma, use_bmi2; };
CodegenStrategy resolve_strategy(CHostCaps caps, uint8_t force_baseline);
