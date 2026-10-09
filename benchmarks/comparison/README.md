# Four-language native benchmark

`bench.dev`, `bench.c`, `bench.rs` and `bench.go` implement the same three unsigned-64 kernels. Every executable reads four decimal values from stdin:

```text
mode n seed rounds
```

- Mode 1 sums `i & 255` for `0 <= i < n`.
- Mode 2 applies xorshift64 (`<<13`, `>>7`, `<<17`) `n` times to `seed`.
- Mode 3 allocates `n` 64-bit values, initializes `(seed + i) & 0xffffffff`, performs `rounds` updates using `(value * 1664525 + 1013904223) & 0xffffffff`, and sums the final array.

The inputs used by the harness stay within the valid arithmetic/allocation limits. The harness checks nine small/boundary fixtures per language in both development and optimized profiles and every timed result against independent Python references (closed-form masked sum, exponentiated GF(2) xorshift transform, and exponentiated affine LCG transform). Reference tests compare these fast formulas against straightforward loops.

Run with installed native toolchains:

```sh
python3 scripts/test_benchmark_compare.py
python3 scripts/benchmark_compare.py --compiler dist/compiler/linux-x86_64/devc --runs 11 --build-runs 5 --output docs/comparison-linux.json --markdown docs/comparison-linux.md
```

Pass `--cc`, `--rustc`, or `--go` to select explicit tools. `--keep-workdir` preserves the generated executables, application files and caches for assembly inspection. Linux runtime processes inherit affinity to the first permitted logical CPU; `--no-pin` disables this. This changes only the benchmark process and its children.

The compiler binary and all application sources/cache files are copied to the OS temporary filesystem. Runtime languages execute in a deterministic shuffled order, with two warmups and eleven timed runs by default. Durations include startup, input parsing, allocation and output; only optimized executables are used for runtime comparisons.

The build table records five complete application rebuilds per profile/language. Dev gets a new object cache for each rebuild; C and Rust use their direct drivers without configured project caches; Go gets a source edit that invalidates its main-package cache. The harness verifies with `go build -n` that a main-source edit requires compilation. Go standard-library dependencies are shared after a separately reported first build with an empty private GOCACHE. No global Go cache is cleared. C/Rust distributions provide precompiled standard libraries.

Dev v0.2/C optimize with `-O3`; Rust uses `-C opt-level=3` with arithmetic overflow checks disabled to match the unsigned operations; Go uses its default optimizer. `--native` adds `-march=native` for Dev/C, `-C target-cpu=native` for Rust and `GOAMD64=v3` for Go on this AVX2-capable x86-64 machine. Rust/Go retain their ordinary safe array access. Dev/C allocate via raw pointers. Full flags, compiler versions, source/compiler SHA-256 hashes, input parameters, shuffled orders and every raw timing sample are saved in JSON. Results are workload/toolchain-specific.

`--fast-tcc /path/to/tcc` selects TinyCC only for Dev development build timings. This is a backend/profile comparison: C could use TinyCC as well, so do not treat that result as an intrinsic language advantage. Optimized Dev/C still share the same backend. `--c-opt 2 --rust-opt 2` with the old v0.1 compiler reproduces the original optimization settings.

`scripts/benchmark_profiles.py` compares v0.1 and v0.2 Dev compilers and GCC/Clang profiles in the same randomized runtime session. `docs/optimization-profiles.json` preserves all its timing samples. `scripts/benchmark.py --fast-tcc /path/to/tcc` additionally measures a 33-module project, unchanged builds, and editing one module body; its older fixed-input runtime kernel is diagnostic, while this four-language suite uses runtime inputs.
