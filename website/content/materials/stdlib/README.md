# Standalone Dev native stdlib

The native stdlib is a separate C library (ABI v1), with optional Dev module bindings.
It builds without Rust or the Dev compiler. The compiler builds and runs basic
programs without this library. The source interpreter is in `runtime/` and uses
its own intrinsic API. This native library introduces no VM or background threads.

```sh
python stdlib/build.py
# Explicit compiler/backend/archiver or output:
python stdlib/build.py --cc clang --ar llvm-ar --output stdlib/lib/libdevruntime.a
```

Only a C compiler, native archiver and Python standard library are required.
The resulting archive is target-specific. Windows and Linux need separate builds;
rebuild when the C target/ABI changes. Rebuilding replaces the archive only after
all compilation/archive operations succeed, preserving a previous successful build
after a failure.

To consume it from Dev, select both the module directory and built library:

```sh
devc run examples/stdlib/main.dev \
  --module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a
```

`std` is a user-selected namespace here, not a compiler builtin. Another alias
works too: `--module-dir myrt=stdlib/modules` with `use "myrt/io"`.
`check` needs just `--module-dir`; `emit` creates only application/module C,
which is subsequently linked to this archive. Runtime C/header modifications
require an explicit `python stdlib/build.py` rebuild; application Dev objects
remain cached and the changed archive causes relinking.

```dev
use "std/strings"
use "std/io"
fn main() {
    let text = strings.new("Hello Dev")
    if text == null { return 1 }
    io.writeln(strings.view(text))
    strings.free(text)
}

main()
```

Modules: `memory` provides alloc/zero-filled arrays/resize/free/overlap-safe copy;
`strings` provides owned dynamic byte strings, clone/append/equality/borrowed
views; `io` provides UTF-8 paths, files and console; `time` provides monotonic
clocks and OS sleep. The C API is in `include/dev_runtime.h`.

Memory, strings and files use explicit ownership. Check allocations/open for null.
Free allocations with `memory.free`, strings with `strings.free`, and close files
with `io.close`. Failed resize retains the old allocation; failed string append
retains the old content. A string view is valid only until mutation/free. Byte
lengths are not Unicode character counts; ordinary text output stops at NUL.
Raw pointers require valid bounds. These hosted OS modules do not supply board
drivers or a freestanding embedded runtime.

See the repository's `docs/stdlib.md` for the complete API contracts and examples.
Packaged runtimes include that document as `API.md`. Runtime tests:

```sh
python stdlib/tests/test_native.py --cc clang
python scripts/smoke_stdlib.py --compiler target/release/devc.exe
# Linux heap/undefined-behavior diagnostics:
python3 stdlib/tests/test_native.py --sanitize
```
