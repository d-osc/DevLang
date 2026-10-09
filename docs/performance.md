# Performance and validation

This document preserves the v0.1 baseline, including its `-O2` release profile. For v0.2's `-O3`, TinyCC development builds, executable caching, CPU-specific profiles and new before/after measurements, see [optimization.md](optimization.md). The old JSON files below remain unchanged as historical evidence.

The expanded **Dev / C / Rust / Go** comparison uses runtime inputs, three kernels, randomized execution order and independent checksum references. See [the comparison report](comparison-linux.md), [raw samples](comparison-linux.json) and `scripts/benchmark_compare.py`. The original fixed-input Dev/C microbenchmark below remains a historical measurement; use the expanded report for the four-language comparison.

Measured on 2026-10-08 on this machine using the release build of `devc 0.1.0`. Linux is Ubuntu in WSL2, x86-64, GCC 13.3; Windows is Windows 11, x86-64, Clang 18.1.7. Each environment reports 16 logical CPUs. These results describe these workloads and toolchains, not a universal performance guarantee.

The benchmark records elapsed wall-clock time around each process. Build figures include process startup, source loading, native compilation/linking where required, and cache checking. The frontend phase is separately measured inside the compiler and does **not** represent total compile latency. Sources and caches are created on each OS's local temporary filesystem; Linux measurements do not use `/mnt/c`. Compiler startup and executable output are included in the runtime comparison.

## Native build latency

Medians of 7 runs, in milliseconds. The 33-module cold build has one sample and is explicitly marked.

| Workload | Windows / Clang | Linux WSL2 / GCC |
| --- | ---: | ---: |
| Hello program, frontend phase only | 0.347 | 0.064 |
| Hello program, `check` process | 12.800 | 0.785 |
| Hello program, cold native build | 281.846 | 268.695 |
| Hello program, unchanged build | 16.989 | 1.461 |
| 33 modules / 257 functions, cold build, one sample | 772.536 | 474.235 |
| 33 modules / 257 functions, unchanged build | 26.472 | 3.368 |
| 33 modules, one implementation changed | 270.150 | 263.620 |
| 33 modules, frontend phase | 6.667 | 1.315 |

An unchanged build compiles zero objects and skips linking. In the 33-module implementation edit test, exactly one object recompiles and 32 remain cached. The tests additionally verify that a function signature change recompiles the declaring module and its direct consumers, while unrelated modules remain cached.

Cold builds still spend most of their time in Clang/GCC and the native linker. `--release` uses `-O2` and can take longer than the default `-O0`. Optimizations and backend choice should follow the actual application's needs.

## Measured cache improvement

Before the cache optimization, every build spawned the C compiler's `--version` command and wrote all generated C/header files. The compiler now checks the resolved executable's path/size/mtime, caches its version, and writes generated files only if objects need compiling.

| Unchanged workload | Windows before → after | Linux before → after |
| --- | ---: | ---: |
| Single module | 59.497 → 16.989 ms | 2.358 → 1.461 ms |
| 33 modules | 96.266 → 26.472 ms | 6.192 → 3.368 ms |

The native regression suite uses a compiler wrapper to verify that `--version` runs only once for unchanged builds and that modifying the compiler executable invalidates the version/object/link caches. These are separate benchmark sessions, so differences in CPU load and filesystem cache also contribute; no cold-build speedup is inferred from them.

## Release runtime comparison

The workload sums `i & 255` over 100,000,000 iterations and prints `12750000000`. Both programs compile with the same backend, `-O2` and `-fwrapv`. Dev and C executables alternate for 7 runs.

| Program | Windows median | Linux median |
| --- | ---: | ---: |
| Dev Lang release | 26.412 ms | 23.968 ms |
| Equivalent C source | 26.640 ms | 38.460 ms |

This confirms that the Dev program is a native executable and produces the same result. It does not establish that Dev Lang is faster than C. Generated expression structure can affect the optimizer, and scheduling/frequency/cache effects influence these short runs. Windows also showed occasional ~170 ms outliers; the JSON reports preserve minimum and maximum measurements.

## Validation

- 14 Rust tests passed on Windows and Linux, including a table of 37 invalid-program cases.
- 35 native/compiler scenarios passed on both Windows/Clang and Linux/GCC: real executable output, Unicode and string escaping, fixed-width integers, pointers/volatile reads and writes, arrays, control flow, recursion, cyclic module imports, C FFI, exported static libraries, generated C, cache reuse/invalidation/corruption recovery, preservation of successful output after a link failure, compiler version caching, error diagnostics and process exit codes.
- Windows `cargo clippy --all-targets -- -D warnings` passed.
- The Cortex-M4 example compiled with Clang targeting `arm-none-eabi`; `llvm-readobj` confirmed ELF32 ARM. With `--release`, `llvm-objdump` showed `ldr r0, [r0]` for `register_read` and `str r1, [r0]` for `register_write`. There was no physical board test.

## Reproduce

```sh
cargo build --release
cargo test
python3 scripts/smoke.py --compiler target/release/devc
python3 scripts/benchmark.py --compiler target/release/devc --output docs/benchmarks-local.json
```

On Windows use `python` and `target/release/devc.exe`. `--cc clang` or `--cc gcc` selects an explicit backend. `--runs N` controls the number of repeated measurements, with a minimum of three. The WSL staging script `scripts/validate-linux.sh` prints the Linux workspace it validated and leaves it available for inspection.

Raw reports: [Windows](benchmarks-windows.json), [Linux](benchmarks-linux.json), [Windows before optimization](benchmarks-windows-before-cache-optimization.json), [Linux before optimization](benchmarks-linux-before-cache-optimization.json).
