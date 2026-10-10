# Interpreter API

`std/don` reads and writes Dev Object Notation (`.don`), a JSON-compatible format
with comments, bare keys and newline separators. See [DON](don.md).

Node-style core modules are also available under `std/`: `net`, `path`, `os`,
`stream`, `url`, `module`, `process`, `events`, `buffer`, `dgram`.
See [Core runtime APIs and contracts](node-core.md).

Filesystem and HTTP/HTTPS clients are available through `std/fs` and `std/http`
in the current source runtime. See [Filesystem and HTTP](fs-http.md) for APIs,
examples, errors and limits. These modules currently support runtime execution only.

JSON is available in the current source runtime through `use "std/json"`.
See [JSON API and contracts](json.md) and `examples/json/main.dev` for parsing,
serialization, typed access, objects/arrays and file I/O. Build the updated runtime
from source; the existing v0.4.0 release binaries do not contain this addition.

Run `devrun FILE.dev [-- arguments]` or `devrun -e 'statements'`.
`--eval` is an alias for `-e`; `--timings` prints load/execution times to stderr.
`--engine auto` (default) uses numeric plans where supported; `--engine ast`
selects the reference AST interpreter. With `d`, put this option after the source filename.
`-h`/`--help` and `-v`/`-V`/`--version` show help and version.
Runtime errors include source path, line and column. File-level statements execute
in order; declarations never invoke `main` automatically. Write `main()` explicitly
to call it. `main` defaults to return type `i32`, with a fallthrough value of zero.
Other non-void functions must return a value. A file-level `return` sets the process
exit code; `return main()` propagates the function's result. A bare `main()` ignores
that result, like any expression statement. Imported files contain declarations only.

Use `use "std/io"` (or `as alias`). Built-in modules need no files or libraries:

| Module | Functions |
| --- | --- |
| `std/io` | `write(str) bool`, `writeln(str) bool`, `read_line() str`, `read_file(path str) str`, `write_file(path str, text str) bool` |
| `std/strings` | `len(str) usize`, `concat(str, str) str`, `equal(str, str) bool` |
| `std/time` | `now_ns() u64`, `now_ms() u64`, `sleep_ms(integer) bool` |
| `std/args` | `len() usize`, `get(integer) str` |

I/O failures produce runtime errors. Text files and strings use UTF-8. String
length and indexing count bytes; indexing produces `u8`. `read_line` removes
the final newline. Arguments include only values following `--`, without the
source filename. Time measures elapsed monotonic time since runtime startup.

`print(value)` writes a line; string `+` concatenates. Fixed-width integer
arithmetic wraps, division by zero and invalid shifts produce errors. Locals
and parameters enforce declared types. Explicit numeric casts use `as`.
Boolean operators short-circuit. Array indices are checked. Unexecuted code is
not type-checked: use `devc check` for native programs, noting that runtime
intrinsics and the native stdlib have different APIs.

The runtime evaluates Dev source without C translation or native code generation.
Only automatically prepared C dependencies use a native-library cache. No networking, async, JIT or JavaScript compatibility
is provided in this release.

## Native C FFI

`extern fn` uses the platform's default C ABI. `--ffi-lib PATH` loads an existing
shared library (.dll on Windows, .so on Linux), and is repeatable. Put the option
after the entry filename when using `d`. Paths resolve from the working directory.
Explicit libraries are searched in command-line order, followed by system CRTs.
If a symbol remains unresolved, the runtime discovers shared libraries beside
the source files containing `extern fn` declarations. This search is nonrecursive,
uses only library extensions for the current OS, and sorts paths deterministically.
Each canonical library path is attempted once. Imported modules contribute their
own directories; unrelated working directories are not searched. Library loading
can run native initialization code. A missing symbol reports adjacent load errors.
Standard symbols such as `puts`, `malloc` and `free` need no explicit library.
Function pointers and call interfaces are cached and the loaded libraries remain
alive throughout execution.

If no library supplies the symbol, adjacent C sources containing that identifier
trigger automatic preparation through sibling `devc native-build`. This compiles
all `.c` files in that declaration module's directory into a shared library;
Dev source remains interpreted. The first run needs Clang/GCC or `DEV_CC`.
The compiler helper stays in `compiler/`; the interpreter has no compiler-crate
dependency. Cache keys include platform, directory, declared symbols, backend
selection, C contents and recursively collected local `.h` contents. Cached
library bytes are checked before reuse, and warm runs need no C backend.
Preparation failures preserve previous successful cache entries.
Cache files are in `.dev-cache/native/`. External system-header/toolchain changes
require removing this directory. Only adjacent C files are compiled, with up to
512 tracked files and 8 MiB per file; custom include/link flags are not supported
by this automatic path. Use a prebuilt library for more complex native projects.
Explicit and adjacent prebuilt libraries take precedence, so remove stale
prebuilt libraries when switching to C source preparation.

```dev
extern fn puts(message str) i32
extern fn device_add(a i32, b i32) i32

fn main() {
    puts("Calling C directly")
    print(device_add(19, 23))
}

main()
```

```powershell
target\release\d.exe examples\ffi\main.dev
```

This command prepares the C dependency automatically on the first run and
reuses it thereafter. Existing compatible shared libraries need no preparation. Supported ABI types:
`i8/i16/i32/i64`, `u8/u16/u32/u64`, `isize/usize`, `bool` (C `_Bool`), `f32/f64`,
`str`, pointers, C-compatible structs, payload-free enums and `void` returns. Variadic calls apply C default argument promotions; scalar callbacks with captured values use libffi closures. Payload unions, top-level by-value arrays and custom calling conventions require C wrappers.

`str` arguments are temporary, read-only NUL-terminated UTF-8 buffers valid only
for the duration of the call. Embedded NUL bytes are rejected. Native code must
not retain or free them. A returned `str` must point to readable, NUL-terminated
UTF-8 data and is copied before argument buffers are released. A null or invalid
UTF-8 string result is an error. Use `*u8` for nullable string pointers.

Live foreign pointers support unsafe scalar/struct/array reads and writes,
field mutation, pointer arithmetic and numeric/bool volatile access. Null,
alignment and address overflow are checked in runtime access; allocation bounds
and foreign-memory lifetimes are the caller's responsibility. Address-of
interpreter locals remains unsupported. Use the allocating library's free function.

Native library architecture, signatures, calling convention and pointer validity
must match declarations; an incorrect native contract can crash the process.

System CRT streams are flushed after native calls to preserve output order with
runtime I/O. A DLL with its own statically linked CRT must flush its private
streams itself when mixing output with the interpreter.

Missing libraries/symbols, unsupported types, conflicting extern signatures,
argument count/type errors and invalid string results produce runtime errors.

## Numeric execution plans

Functions consisting of supported numeric/bool operations, numeric arrays,
control flow and calls are lowered once during module load to typed register
instructions. Entry scripts can use the same path. Plans stay in memory and
never invoke `devc`, emit C, generate native code or create cache files.
Constants and variable names are resolved outside compute loops. An unsupported
or inconsistent function uses the AST interpreter; fallback functions may call
planned numeric functions. Error checks and source locations remain active.
Array values still copy at assignment/call boundaries, while indexed reads and
writes access individual elements directly. Workspaces are reused across calls;
recursive invocations take independent frames, with at most one cached frame
per function. No SIMD, automatic parallel loops or native JIT is provided. Explicit `spawn`, `Task<T>`, `await(task)`, nonblocking `ready(task)` and `async fn` use OS threads with independent interpreter frames and copy-on-write snapshots.

Use `examples/runtime/compute.dev` for a 100-million-iteration source example.
Measured before/after results and reproduction commands are in
[runtime-compute.md](runtime-compute.md).

## Shared language features

Range `for`, nominal value structs, payload enums with exhaustive `match` and inferred/explicit generics use
the shared frontend described in [language.md](language.md). Type definitions
and specialization arguments are validated during load. Supported numerical
generic functions and range loops retain numeric plans; struct/enum-heavy
functions use AST fallback. No compiler executable is required for these pure
Dev features. Field and array-element mutation checks indices and declared
member types, and struct copies do not alias mutable nested arrays.

Collections, function values, closures, traits, const generics and thread-backed tasks are documented in [advanced.md](advanced.md). Pure Dev execution continues to require no compiler.

`then(task, next)` returns a continuation task without waiting in its caller; its worker still waits on an OS thread. `callback_context(f)` exposes a managed C userdata pair, with userdata last. Captured values are released with the owner; runtime weak owners diagnose expired context invocations. See [advanced.md](advanced.md) for trampoline storage lifetime and unsafe C contracts.
