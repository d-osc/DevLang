# Source runtime compute optimization

Numeric workloads use typed register instructions built in memory, with AST fallback for unsupported code. No native JIT, C translation, compiler executable or generated files are involved in pure Dev execution. This improves the interpreter; it does not establish native-speed parity.

These measurements identify the numeric-plan implementation snapshot before the later struct/enum/generic frontend extension; binary hashes record the measured versions.

Measured on 2026-10-09, comparing the saved pre-change release runtime with the new release runtime. Each workload has one warmup and seven measured samples per binary, alternating order, with every output checked. Timings include process startup, parsing, plan creation, computation and output. Source files are on OS-local temporary filesystems; PATH is empty and C backend variables name a missing compiler. Windows and WSL runs were performed sequentially. CPU affinity was not pinned; medians, ranges, hashes and raw samples are recorded.

| Workload | Windows before → after (ms) | Speedup | WSL Linux before → after (ms) | Speedup |
| --- | ---: | ---: | ---: | ---: |
| integer_loop | 274.823 → 24.639 | 11.15× | 217.697 → 11.496 | 18.94× |
| float_loop | 159.606 → 22.123 | 7.21× | 131.917 → 8.175 | 16.14× |
| array_loop | 105.278 → 14.554 | 7.23× | 108.241 → 1.807 | 59.90× |
| function_loop | 75.364 → 24.728 | 3.05× | 38.222 → 8.532 | 4.48× |

Workloads: 500,000 integer iterations; 300,000 float iterations; 20,000 indexed updates/reads of a 256-element array; and 30,000 user-function calls. These fixed kernels are executed as interpreter instructions, without loop elimination. Small Windows workloads include noticeable process-startup overhead. Results are specific to these workloads and this machine; they do not imply a universal speedup.

Raw evidence: [Windows](runtime-compute-windows.json), [WSL Linux](runtime-compute-linux.json). Baseline binaries are local snapshots under `target/` in each validation workspace; their SHA256 identities are in the reports.

Changes: resolve local names to slots once, parse constants once, use direct jumps for control flow, read indexed elements without copying whole arrays, index function definitions without cloning their bodies, and reuse at most one workspace per function. Array assignment and calls retain value-copy semantics. Bounds, integer divide/shift checks, wrapping, source diagnostics and recursion limits remain.

Validation: 101 differential numeric cases run in both auto and AST modes, comparing output, exit status and exact source-located errors, including recursion, scopes, short-circuiting, array copies, all integer widths, rounding-sensitive casts, NaN/Infinity, module calls, lazy errors and seeded generated loops. Existing runtime, explicit-entry, CLI, native FFI and automatic C dependency suites also pass on Windows and Linux. Windows Clippy and workspace Rust tests pass; Linux workspace Rust tests pass.

```powershell
.\target\release\d.exe examples/runtime/compute.dev --timings
python scripts/smoke_numeric.py --runtime target/release/devrun.exe
python scripts/benchmark_runtime.py --runtime target/release/devrun.exe --before target/devrun-before-compute.exe --runs 7 --output docs/runtime-compute-local.json
```

The example runs 100 million integer iterations and prints `12750000000`. A single sanity run took about 2.25 seconds of execution on Windows and 2.46 seconds on WSL, excluding load; these are single samples, not comparative benchmark medians.

Use `--engine ast` for reference execution. Mixed string/pointer/intrinsic-heavy functions can fall back to AST; put the numerical kernel in a separate function to use a plan even when its caller handles I/O. There is no native JIT, SIMD backend or parallel execution. For maximum native throughput, the independent `d build` path remains available.
