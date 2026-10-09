# Dev Lang vs C vs Rust vs Go

Measured 2026-10-08T23:39:22+07:00 on Linux-6.6.87.2-microsoft-standard-WSL2-x86_64-with-glibc2.39, Intel(R) Core(TM) i5-14400F.

Runtime: median milliseconds across randomized rounds, including process startup, stdin parsing, allocation and printing. Lower is faster. All results match independent Python references.

| Language | Masked sum | Xorshift64 | Array update |
| --- | ---: | ---: | ---: |
| Dev Lang | 43.998 | 46.430 | 19.351 |
| C | 44.040 | 46.318 | 19.105 |
| Rust | 34.424 | 46.844 | 19.708 |
| Go | 52.361 | 46.344 | 27.421 |

Full application rebuilds, with toolchain/standard-library dependencies already warm. No project cache is reused for Dev; Go main-package source changes every repetition. Optimized Dev and C use the same C backend.

| Language | Build, no application optimization | Build, optimized | Optimized build min–max | Optimized binary KiB |
| --- | ---: | ---: | ---: | ---: |
| Dev Lang | 4.831 ms | 299.582 ms | 266.071–324.259 ms | 16.0 |
| C | 274.848 ms | 328.022 ms | 260.761–698.325 ms | 15.9 |
| Rust | 327.992 ms | 362.657 ms | 341.305–558.351 ms | 4415.2 |
| Go | 229.615 ms | 227.932 ms | 219.468–250.580 ms | 2000.5 |

Build timings vary substantially and their ranges overlap; small median differences do not establish a stable compile-speed ranking.

Dev development builds use TinyCC, while C development builds use the reported GCC/Clang backend. This demonstrates the fast backend/profile, not an inherent advantage over C: C can use TinyCC too. TinyCC produces unoptimized runtime code; optimized runtime measurements below use GCC/Clang.

Unchanged optimized builds (only drivers with caches configured in this experiment):

| Driver | Median |
| --- | ---: |
| Dev Lang | 1.924 ms |
| Go | 182.481 ms |

The first Go build with an empty private GOCACHE took 2538.280 ms, including compiling standard-library dependencies. This is one sample and is separate from the application rebuild table.

## Parameters

- Runtime repetitions: 11; warmups per workload/language: 2; build repetitions: 5.
- Runtime CPU affinity: 0; Go runtime GOMAXPROCS=1, GOGC=100, CGO_ENABLED=0, GOAMD64=v1.
- Inputs are supplied through stdin after compilation; no fixed-workload constant is available to the optimizer.
- Dev compiler executable, benchmark sources, binaries and caches are staged on the local temporary filesystem (not /mnt/c on Linux).
- Unsigned 64-bit operations; xorshift wraps at 64 bits; array elements wrap at 32 bits.
- masked_sum: mode/n/seed/rounds = (1, 200000003, 0, 0); checksum = 25500000003.
- xorshift64: mode/n/seed/rounds = (2, 30000000, 88172645463325252, 0); checksum = 2535504059865855682.
- array_update: mode/n/seed/rounds = (3, 262144, 123456789, 256); checksum = 562952374714368.

## Toolchains and flags

- Dev Lang: `devc 0.2.0`; optimized flags: `--release / -O3`.
- C: `cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0`; optimized flags: `-std=c11 -O3 -fwrapv`.
- Rust: `rustc 1.98.1 (48a229cea 2026-09-01)`; optimized flags: `-C opt-level=3 -C overflow-checks=off -C debuginfo=0`.
- Go: `go version go1.22.2 linux/amd64`; optimized flags: `go build (default optimized)`.
- Development backend: `tcc version 0.9.28rc 2026-10-03 HEAD@43c7708 (x86_64 Linux)`; Dev flags: `--fast`.

Dev/C use raw pointers and malloc/free for the array; Rust uses a safe Vec and Go uses a checked slice. Rust/Go initialize their allocation to zero before the same explicit fill loop. This compares these ordinary implementations, including memory-safety/runtime costs; the compiler may eliminate redundant checks or writes.

Binary sizes are as linked, without a separate stripping step. Rust/Go include more runtime/standard-library code; Dev/C dynamically link libc. Equal optimization-level numbers do not imply equal optimization passes. These results are specific to these kernels/toolchains/WSL and do not establish a universal language ranking.

References: [Rust codegen flags](https://doc.rust-lang.org/rustc/codegen-options/), [Go build/cache flags](https://pkg.go.dev/cmd/go), [C optimization levels](https://clang.llvm.org/docs/CommandGuide/clang.html).
