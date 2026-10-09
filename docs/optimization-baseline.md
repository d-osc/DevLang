# Dev Lang vs C vs Rust vs Go

Measured 2026-10-08T23:14:06+07:00 on Linux-6.6.87.2-microsoft-standard-WSL2-x86_64-with-glibc2.39, Intel(R) Core(TM) i5-14400F.

Runtime: median milliseconds across randomized rounds, including process startup, stdin parsing, allocation and printing. Lower is faster. All results match independent Python references.

| Language | Masked sum | Xorshift64 | Array update |
| --- | ---: | ---: | ---: |
| Dev Lang | 50.057 | 43.244 | 20.875 |
| C | 50.568 | 45.398 | 21.006 |
| Rust | 34.460 | 45.037 | 20.384 |
| Go | 50.325 | 46.744 | 27.989 |

Full application rebuilds, with toolchain/standard-library dependencies already warm. No project cache is reused for Dev; Go main-package source changes every repetition. Both Dev and C use the same C backend.

| Language | Build, no application optimization | Build, optimized | Optimized build min–max | Optimized binary KiB |
| --- | ---: | ---: | ---: | ---: |
| Dev Lang | 278.278 ms | 276.336 ms | 271.819–331.873 ms | 16.0 |
| C | 274.926 ms | 279.107 ms | 246.829–292.590 ms | 15.9 |
| Rust | 330.601 ms | 400.519 ms | 352.342–422.922 ms | 4415.2 |
| Go | 243.223 ms | 235.059 ms | 204.092–471.185 ms | 2000.5 |

Build timings vary substantially and their ranges overlap; small median differences do not establish a stable compile-speed ranking.

Unchanged optimized builds (only drivers with caches configured in this experiment):

| Driver | Median |
| --- | ---: |
| Dev Lang | 1.677 ms |
| Go | 169.299 ms |

The first Go build with an empty private GOCACHE took 2632.291 ms, including compiling standard-library dependencies. This is one sample and is separate from the application rebuild table.

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

- Dev Lang: `devc 0.1.0`; optimized flags: `--release / -O2`.
- C: `cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0`; optimized flags: `-std=c11 -O2 -fwrapv`.
- Rust: `rustc 1.98.1 (48a229cea 2026-09-01)`; optimized flags: `-C opt-level=2 -C overflow-checks=off -C debuginfo=0`.
- Go: `go version go1.22.2 linux/amd64`; optimized flags: `go build (default optimized)`.

Dev/C use raw pointers and malloc/free for the array; Rust uses a safe Vec and Go uses a checked slice. Rust/Go initialize their allocation to zero before the same explicit fill loop. This compares these ordinary implementations, including memory-safety/runtime costs; the compiler may eliminate redundant checks or writes.

Binary sizes are as linked, without a separate stripping step. Rust/Go include more runtime/standard-library code; Dev/C dynamically link libc. Equal optimization-level numbers do not imply equal optimization passes. These results are specific to these kernels/toolchains/WSL and do not establish a universal language ranking.

References: [Rust codegen flags](https://doc.rust-lang.org/rustc/codegen-options/), [Go build/cache flags](https://pkg.go.dev/cmd/go), [C optimization levels](https://clang.llvm.org/docs/CommandGuide/clang.html).
