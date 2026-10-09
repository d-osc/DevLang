# Dev Lang v0.2 optimization results

These are historical v0.2 measurements. The measured compiler binaries are
preserved in `dist/baseline-v0.2/`; the current v0.3 distribution additionally
includes [native runtime modules](runtime.md).

v0.2 adds a fast development backend, stronger release optimization, CPU-specific
builds, modular-width lowering and executable caching. It improves measured
workloads substantially. It does not outperform every language in every metric:
Rust still wins the masked-sum kernel, Go builds optimized applications faster,
and ordinary GCC/Clang development builds show no consistent overall improvement.

## Changes

- `--fast` uses a separate TinyCC subprocess for hosted development executables.
  The Linux distribution includes a pinned backend, matching source and LGPL
  license. Static libraries, freestanding targets and optimized builds use
  Clang/GCC. Windows TinyCC builds have not been validated or bundled.
- `--release` now uses `-O3`, retaining signed wrapping through `-fwrapv`.
  `--native` adds `-march=native`; CPU identity/features join the cache key.
  Native executables can require instructions unavailable on another machine.
- Unsigned 64-bit arithmetic followed by `& 0xffffffff` can lower `+`, `-`, `*`
  and bitwise operations to unsigned 32-bit arithmetic. Arithmetic modulo
  2^32 preserves these results. Division, shifts, casts and function calls keep
  their full-width evaluation. A generated macro selects this path only when
  the backend advertises SSE4.1, which provides 32-bit SIMD multiplication;
  other targets retain the original expression. Calls execute once.
- Single-module executables use one C-driver invocation for compilation and
  linking. A content-checked executable cache restores missing/modified outputs
  without recompilation. Failed compilation preserves the previous executable.
  External C/assembly inputs or custom backend flags conservatively rebuild.
- Multi-module projects retain parallel object compilation and incremental
  body/signature invalidation. Unchanged projects create no idle worker threads.

The width transformation is a general compiler optimization, with no benchmark
function names or input sizes recognized. C can express equivalent narrowed
arithmetic too; the results below compare the committed implementations.

## Before and after in the same session

Intel Core i5-14400F, Ubuntu/WSL2, native Linux temporary filesystem. Runtime:
15 randomized samples with 2 warmups per profile; builds: 5 fresh-cache samples.
Runtime inputs and independent checksums prevent constant-input specialization.
Times include process startup, I/O and allocation. Lower is faster.

| Measurement | v0.1 | v0.2 | Change |
| --- | ---: | ---: | ---: |
| Development full build | 288.225 ms, GCC | 5.352 ms, TinyCC `--fast` | 53.9x faster |
| Masked sum | 49.952 ms, GCC `-O2` | 19.020 ms, GCC `-O3 --native` | 2.63x faster |
| Array update | 20.762 ms, GCC `-O2` | 11.606 ms, GCC `-O3 --native` | 1.79x faster |
| Xorshift64 | 45.890 ms, GCC `-O2` | 46.042 ms, GCC `-O3 --native` | Essentially unchanged |

TinyCC's build advantage is a backend/profile choice, not an inherent advantage
over C; C can use TinyCC. `--fast` produces unoptimized runtime code. Runtime
rows use separate optimized executables. Build-time ranges and all GCC/Clang,
portable/native alternatives are preserved in
[optimization-profiles.json](optimization-profiles.json).

Portable GCC `-O3` remains useful: masked sum 42.399 ms and array update
19.164 ms in that same session. Clang 19 native improves masked sum further to
16.051 ms, but its array update takes 20.805 ms. Select a backend/profile using
the application's own workload; `--native` is optional.

## Final four-language comparison

11 randomized runtime samples, 2 warmups, pinned logical CPU 0. GCC 13.3 for
Dev/C, Rust 1.98.1, Go 1.22.2. Dev/C: `-O3 -march=native`; Rust: opt-level 3,
target-cpu native; Go: normal optimization with GOAMD64=v3. All checksums match.

| Language | Masked sum | Xorshift64 | Array update |
| --- | ---: | ---: | ---: |
| Dev Lang | 18.211 ms | 43.974 ms | 12.025 ms |
| C | 18.676 ms | 43.419 ms | 27.352 ms |
| Rust | 15.527 ms | 42.740 ms | 20.734 ms |
| Go | 48.326 ms | 44.252 ms | 28.194 ms |

Dev wins this array implementation by 2.27x versus C, 1.72x versus Rust and
2.34x versus Go. Rust wins masked sum; xorshift results remain close. Dev's
optimized full-build median is 357.781 ms versus Go's 229.975 ms. Native Dev
binaries grow from about 16 KiB to 20 KiB; better runtime performance has a
code-size/build-time cost. Source implementations include raw-pointer allocation
for Dev/C and ordinary safe Vec/slice allocation/access for Rust/Go. These are
three kernels on one CPU and installed toolchain versions, not a language ranking.

Full methods, flags, source/compiler hashes, commands, ranges and raw samples:
[native report](optimization-native.md), [native JSON](optimization-native.json),
[portable report](optimization-portable.md),
[portable JSON](optimization-portable.json). Historical v0.1 results remain in
[comparison-linux.md](comparison-linux.md) and [performance.md](performance.md).

## Modules and build cache

The 7-run Linux cache/project benchmark uses a compiler copied to the Linux
temporary filesystem and a Linux-local TinyCC backend:

| v0.2 development measurement | Time |
| --- | ---: |
| Hello, full TinyCC build | 4.097 ms median |
| Hello, unchanged cached build | 1.059 ms median |
| 33 modules / 257 functions, full build | 18.696 ms, one sample |
| 33 modules, unchanged | 2.859 ms median |
| One module body changed, 32 objects cached | 8.132 ms median |

Raw reports: [Linux](optimization-cache-linux.json),
[Windows/Clang](optimization-cache-windows.json). Windows uses Clang, with no
TinyCC bundle; these Linux `--fast` timings do not describe Windows builds.
WSL projects, compiler and backend under `/mnt/c` can take much longer because
of filesystem/process overhead. Stage them on Linux's filesystem to reproduce
the small millisecond figures. No global Windows settings are changed.

## Usage and reproduction

```sh
# Linux bundle: development speed
./dist/linux-x86_64/devc run examples/modules/main.dev --fast

# Optimized program for this CPU; GCC gave the fastest array result here
./dist/linux-x86_64/devc build main.dev --release --native --cc gcc

# Portable optimized program
./dist/linux-x86_64/devc build main.dev --release

# Build the pinned development backend from matching upstream source
bash scripts/build-fast-backend.sh

python3 scripts/benchmark_compare.py --compiler dist/linux-x86_64/devc \
  --cc cc --fast-tcc /path/on/linux/filesystem/tcc --native \
  --output docs/native-local.json --markdown docs/native-local.md

python3 scripts/benchmark_profiles.py \
  --before dist/baseline-v0.1/linux-x86_64/devc \
  --after dist/linux-x86_64/devc --cc cc --clang clang-19 \
  --tcc /path/on/linux/filesystem/tcc --output docs/profiles-local.json

python3 scripts/benchmark.py --compiler dist/linux-x86_64/devc \
  --fast-tcc /path/on/linux/filesystem/tcc --output docs/cache-local.json
```

## Verification

- 14 Rust tests pass on Windows and Linux. Windows Clippy with warnings denied
  passes; the Linux toolchain lacks its Clippy component, so Linux Clippy was
  not completed. Three independent Python reference tests pass.
- 35 existing native/compiler scenarios pass on Windows/Clang and Linux/GCC.
- New cache/profile regression scenarios: 9 on Windows/Clang, 23 on Linux with
  GCC/TinyCC, and 9 on Linux/Clang 19. The modular-width scenarios compare 279
  results per profile against independent Python arithmetic, covering large
  values, wrapping, subtraction, full-width division/right shifts, commuted and
  nested masks, signed arithmetic and a stateful call evaluated once.
- Both development and optimized four-language binaries pass 9 boundary/input
  fixtures per language; every measured runtime result is independently checked.
- Relocated bundled TinyCC successfully builds/runs the three-file module example.
- Optimized ARM Cortex-M4 freestanding archive disassembles to `ldr`/`str` volatile
  access with no hosted runtime calls. Physical-board execution is unverified.
- Windows/Linux v0.2 compiler binaries and SHA256 manifests are refreshed in `dist`.

Backend references: [GCC optimization options](https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html),
[GCC x86 target options](https://gcc.gnu.org/onlinedocs/gcc/x86-Options.html),
[TinyCC documentation](https://bellard.org/tcc/tcc-doc.html).
