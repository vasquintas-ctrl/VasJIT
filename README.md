# VasJIT

Hybrid **Rust + C++** JIT for Nintendo Wii (IBM PowerPC **Broadway**) and Switch (Tegra X1 **ARMv8-A**).

```
guest bytes → decode/lower → IR → opt → linear-scan → FFI → x86-64
                 Rust                              C++
```

## Build

```
cargo test
cargo run -- caps
cargo run -- decode-ppc 3800002A 4E800020
cargo run -- smoke
```

`FORCE_BASELINE_CODEGEN=1` forces the SSE2 floor.

Guest memory is a 4 GiB (or fallback) `base+offset` map. Broadway `rA=0` is the constant 0 in D-form addressing.
