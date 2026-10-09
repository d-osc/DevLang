# Dev Lang vs C vs Rust vs Go

Measured 2026-10-08T23:40:39+07:00 on Linux-6.6.87.2-microsoft-standard-WSL2-x86_64-with-glibc2.39, Intel(R) Core(TM) i5-14400F.

Runtime: median milliseconds across randomized rounds, including process startup, stdin parsing, allocation and printing. Lower is faster. All results match independent Python references.

| Language | Masked sum | Xorshift64 | Array update |
| --- | ---: | ---: | ---: |
| Dev Lang | 18.211 | 43.974 | 12.025 |
| C | 18.676 | 43.419 | 27.352 |
| Rust | 15.527 | 42.740 | 20.734 |
| Go | 48.326 | 44.252 | 28.194 |

Full application rebuilds, with toolchain/standard-library dependencies already warm. No project cache is reused for Dev; Go main-package source changes every repetition. Optimized Dev and C use the same C backend.

| Language | Build, no application optimization | Build, optimized | Optimized build min–max | Optimized binary KiB |
| --- | ---: | ---: | ---: | ---: |
| Dev Lang | 4.977 ms | 357.781 ms | 274.349–384.229 ms | 20.0 |
| C | 264.230 ms | 277.515 ms | 263.682–362.771 ms | 15.9 |
| Rust | 286.812 ms | 362.225 ms | 302.280–418.902 ms | 4416.0 |
| Go | 230.952 ms | 229.975 ms | 199.847–324.783 ms | 1999.9 |

Build timings vary substantially and their ranges overlap; small median differences do not establish a stable compile-speed ranking.

Dev development builds use TinyCC, while C development builds use the reported GCC/Clang backend. This demonstrates the fast backend/profile, not an inherent advantage over C: C can use TinyCC too. TinyCC produces unoptimized runtime code; optimized runtime measurements below use GCC/Clang.

Unchanged optimized builds (only drivers with caches configured in this experiment):

| Driver | Median |
| --- | ---: |
| Dev Lang | 1.727 ms |
| Go | 186.539 ms |

The first Go build with an empty private GOCACHE took 2278.892 ms, including compiling standard-library dependencies. This is one sample and is separate from the application rebuild table.

## Parameters

- Runtime repetitions: 11; warmups per workload/language: 2; build repetitions: 5.
- Runtime CPU affinity: 0; Go runtime GOMAXPROCS=1, GOGC=100, CGO_ENABLED=0, GOAMD64=v3.
- Inputs are supplied through stdin after compilation; no fixed-workload constant is available to the optimizer.
- Dev compiler executable, benchmark sources, binaries and caches are staged on the local temporary filesystem (not /mnt/c on Linux).
- Unsigned 64-bit operations; xorshift wraps at 64 bits; array elements wrap at 32 bits.
- masked_sum: mode/n/seed/rounds = (1, 200000003, 0, 0); checksum = 25500000003.
- xorshift64: mode/n/seed/rounds = (2, 30000000, 88172645463325252, 0); checksum = 2535504059865855682.
- array_update: mode/n/seed/rounds = (3, 262144, 123456789, 256); checksum = 562952374714368.

## Toolchains and flags

- Dev Lang: `devc 0.2.0`; optimized flags: `--release / -O3 --native / -march=native`.
- C: `cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0`; optimized flags: `-std=c11 -O3 -fwrapv -march=native`.
- Rust: `rustc 1.98.1 (48a229cea 2026-09-01)`; optimized flags: `-C opt-level=3 -C overflow-checks=off -C debuginfo=0 -C target-cpu=native`.
- Go: `go version go1.22.2 linux/amd64`; optimized flags: `go build (default optimized) GOAMD64=v3`.
- Development backend: `tcc version 0.9.28rc 2026-10-03 HEAD@43c7708 (x86_64 Linux)`; Dev flags: `--fast`.

Dev/C use raw pointers and malloc/free for the array; Rust uses a safe Vec and Go uses a checked slice. Rust/Go initialize their allocation to zero before the same explicit fill loop. This compares these ordinary implementations, including memory-safety/runtime costs; the compiler may eliminate redundant checks or writes.

Binary sizes are as linked, without a separate stripping step. Rust/Go include more runtime/standard-library code; Dev/C dynamically link libc. Equal optimization-level numbers do not imply equal optimization passes. These results are specific to these kernels/toolchains/WSL and do not establish a universal language ranking.

References: [Rust codegen flags](https://doc.rust-lang.org/rustc/codegen-options/), [Go build/cache flags](https://pkg.go.dev/cmd/go), [C optimization levels](https://clang.llvm.org/docs/CommandGuide/clang.html).
