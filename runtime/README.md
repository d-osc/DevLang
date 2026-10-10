# Dev source runtime

`std/dns` and `std/cli` provide OS host resolution and schema-based argument
parsing. See [API contracts](../docs/system-libs.md).

`regex`, `encoding`, `crypto`, `compression`, `archive`, `uuid` add text matching,
byte conversion, hashing/AEAD, codecs, ZIP/TAR and random UUIDs. See
[data library contracts](../docs/data-libs.md).

Foundational `math`, `random`, `datetime`, `test`, `log` and expanded `strings`
are built-in source-runtime modules. See [API contracts](../docs/basic-libs.md).

`std/don` supports [Dev Object Notation](../docs/don.md): flexible data/config
syntax, managed objects and JSON conversion, without a C compiler.

Additional `std/net`, `std/path`, `std/os`, `std/stream`, `std/url`, `std/module`,
`std/process`, `std/events`, `std/buffer`, `std/dgram` provide TCP/UDP, file
streams, events, byte buffers and system utilities. See
[core APIs](../docs/node-core.md) for signatures, ownership and differences from
Node. These modules are runtime-only.

Source runtime modules `std/fs` and `std/http` provide filesystem operations and
a blocking HTTP/HTTPS client without a C compiler. Node-style filesystem Sync APIs,
`std/fs/promises` with Task results and `http.createServer` are also available.
See [API documentation](../docs/fs-http.md) for the supported subset and limits.

`devrun` interprets `.dev` source directly. Pure Dev execution needs no compiler or build artifacts. Source-only C FFI
dependencies are prepared automatically by the separate `devc` executable.
Compiler and runtime are independent executables; only the syntax crate is shared.

```sh
cargo build --release -p dev-runtime
./target/release/devrun examples/hello.dev
./target/release/devrun examples/modules/main.dev
./target/release/devrun examples/runtime/main.dev -- hello
./target/release/devrun -e 'print(40 + 2)'
./target/release/devrun --eval 'print(40 + 2)' --timings
```

Windows uses `devrun.exe`. Distribute the binary alone; users do not need Rust
or the compiler for pure Dev or prebuilt-library FFI execution. Automatic
C dependency preparation needs sibling `devc` and a C backend on the first run. Source builds need Rust, native C build tools for
bundled libffi, and the sibling `syntax/` crate, but not `compiler/` or `stdlib/`.

Supports functions, relative imports (including cycles), local scopes, typed
numbers, strings, arrays, casts, conditions, while/range-for loops, break/continue,
value structs, payload enums with exhaustive `match`, inferred/explicit function/struct/enum generics and print.
The runtime executes entry-file statements in source order. Function declarations
do not execute their bodies. Call `main()` explicitly if you define it:

```dev
fn main() {
    print("Hello")
}

main()
```

`-e` evaluates the supplied source directly. Top-level `print(42)` works without
any function. An expression such as `main()` discards its return value; use
`return main()` at file level to propagate it as the process exit code.
Imported modules contain declarations only and do not invoke `main`.
File-level variables are local to the entry script; pass values as arguments
to functions instead of capturing script locals.
Strings and arrays are owned automatically; arrays use value copies.

The default `auto` engine builds typed numeric instruction plans in memory
for supported functions and entry scripts, with AST fallback for other code.
It has no JIT and does not claim native performance. `extern fn`
calls work through libffi and loaded shared libraries. Unsafe raw pointer reads/writes
and volatile scalar access work on live foreign memory; address-of interpreter locals remains native-only. Field and array-element assignments support nested local values. Recursion is limited to 128 calls.

See `docs/runtime.md` for intrinsic APIs and `stdlib/` for the separate native
C library. Validate with `python scripts/smoke_runtime.py --runtime target/release/devrun`.

Native FFI:

```powershell
.\target\release\d.exe examples/ffi/main.dev
```

Missing native symbols trigger automatic discovery
of shared libraries beside modules declaring extern functions. `--ffi-lib`
selects libraries explicitly or loads libraries from other directories;
standard C symbols such as `puts` resolve from the system CRT. Native code is
prepared automatically from adjacent C files when no library supplies the symbol.
The runtime delegates this work to sibling `devc`; the first run needs Clang/GCC
or `DEV_CC`. Cached libraries live in `.dev-cache/native/`; C and local header
changes invalidate them. Warm execution needs no C backend. Prebuilt libraries
take precedence. Dev source itself remains interpreted. Supports
integer/size/bool/float, UTF-8 `str`, void returns, C structs, variadic functions, scalar callbacks and unsafe direct foreign memory.
See `docs/runtime.md` for ABI, ownership and unsupported features. Validate with
`python scripts/smoke_ffi.py --bin-dir target/release`.

Validate automatic preparation with `python scripts/smoke_native_auto.py --bin-dir target/release`.

## Compute runtime

Numeric plans use indexed local slots, pre-parsed constants and explicit jumps
instead of repeated AST walks and name lookups. Integer/float/bool operations,
numeric arrays, branches, loops, casts and function calls are supported. Reading
an array element does not copy the entire array; array assignment and parameters
still preserve value-copy semantics. Repeated calls reuse one workspace per
function, with independent frames for recursion.

```powershell
.\target\release\d.exe examples/runtime/compute.dev --timings
.\target\release\d.exe examples/runtime/compute.dev --engine ast
```

Runtime APIs now include `std/result`, `std/timers`, `std/child_process`,
`std/sqlite`, `std/csv`, `std/toml`, `std/yaml`, `std/tls`, `std/websocket`.
See [control libraries](../docs/control-libs.md), [storage libraries](../docs/storage-libs.md)
and [secure networking](../docs/secure-network.md) for contracts and limits.
`std/sync` adds shared channels, mutex updates and cancellation tokens; see
[sync contracts](../docs/sync.md) and `examples/sync/main.dev`.

`--engine auto` is the default; `--engine ast` selects the reference interpreter.
Both modes run without `devc`, a native toolchain or generated files for pure
Dev. These plans are runtime-internal instructions, not native compilation.
Unsupported or statically inconsistent plans fall back to AST execution;
unexecuted invalid code retains the existing lazy error behavior. Bounds checks,
integer divide/shift checks, numeric wrapping and the 128-call recursion limit
remain active. Strings, pointers and intrinsic-heavy functions generally use
AST fallback, but called numeric functions can still use plans.

See [measured results](../docs/runtime-compute.md). Validate with
`python scripts/smoke_numeric.py --runtime target/release/devrun`.

Use `examples/features/main.dev` for structs, enums, ranges and explicit generics. See `docs/language.md` for syntax and limits; validate with `python scripts/smoke_features.py --bin-dir target/release`.

Recursive `*T`/`Ref<T>` layouts are accepted. Pure Dev uses immutable `ref(value)`
and `deref(reference)`; live foreign pointers can be read/written in lexical unsafe blocks.
See `examples/features/payload.dev` and `docs/language.md` for match syntax and
reference lifetime differences between runtime and native compilation.

See `docs/advanced.md` for reference-counted collections, function values/closures, constrained/const generics and thread-backed async tasks.
