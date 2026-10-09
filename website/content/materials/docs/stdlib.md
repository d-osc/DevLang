# Standalone Dev runtime v1 / compiler v0.4

An optional native library for hosted Windows and Linux programs. It introduces
no VM, garbage collector, background threads or automatic application heap.
The C runtime builds independently with `python stdlib/build.py`, without Rust
or a Dev compiler. Dev bindings are ordinary modules and use the typed
scalar/pointer C ABI. Consumers explicitly provide the module directory and
prebuilt library; compiler objects remain cached when a rebuilt archive relinks.

```dev
use "std/strings"
use "std/io"
use "std/time"

fn main() {
    unsafe {
    let message = strings.new("Hello")
    if message == null { return 1 }
    if not strings.append(message, " ภาษาไทย") {
        strings.free(message)
        return 1
    }
    io.writeln(strings.view(message))
    print(strings.len(message))
    strings.free(message)
    }
}

main()
```

Build the runtime and consume it explicitly:

```sh
python stdlib/build.py
devc run examples/stdlib/main.dev --module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a
```

Add `--release --native` for optimized application code or `--fast` for TinyCC.
`std` is a chosen namespace, not a compiler builtin. Any namespace works with
`--module-dir NAME=DIR`. There is no runtime auto-discovery or `DEV_RUNTIME`.
Compiler and runtime are distributed in separate `dist/compiler/<platform>`
and `dist/stdlib/<platform>` directories; each may be relocated independently.
Existing relative imports remain available.

## Memory

| Function | Contract |
| --- | --- |
| `memory.alloc(bytes) *void` | Uninitialized allocation; at least one byte for zero size; null on failure |
| `memory.alloc_array(count, size) *void` | Zero-filled allocation; rejects multiplication overflow; at least one byte for a zero product |
| `memory.resize(data, bytes) *void` | Reallocate; failure preserves the old allocation; zero size frees it and returns null |
| `memory.free(data)` | Release allocation; null is accepted |
| `memory.copy(dest, source, bytes)` | Overlap-safe byte copy |
| `memory.fill(dest, value u8, bytes)` | Fill a region with a byte |

Sizes/counts use `usize`. Callers provide valid pointers and allocated bounds.
Copy/fill do nothing for zero length and accept null in that case. Do not
overwrite the only owner with a failed resize:

```dev
let next = memory.resize(data, 128)
if next != null { data = next }
```

## Strings

Owned strings use opaque `*void` handles. Check `strings.new(text)` and
`strings.clone(handle)` for null. Release each successful allocation exactly
once with `strings.free(handle)`; null is accepted. A clone owns independent
storage. Null handles have length zero and an empty view; mutating null fails.

| Function | Contract |
| --- | --- |
| `strings.new(text str)` | Copy a NUL-terminated UTF-8 byte string; null text creates an empty string |
| `strings.clone(handle)` | Copy all bytes, including embedded NUL |
| `strings.len(handle) usize` | Byte length, not Unicode character count |
| `strings.view(handle) str` | Borrowed NUL-terminated view; valid until mutation or free |
| `strings.append(handle, text) bool` | Append text; supports appending the handle's own view |
| `strings.append_bytes(handle, data *u8, bytes usize) bool` | Append bytes, including NUL; valid bounded source required |
| `strings.equal(a, b) bool` | Compare byte lengths/content; two null handles compare equal |
| `strings.clear(handle)` | Empty string while retaining its allocation |
| `strings.free(handle)` | Release handle and byte storage |

Storage grows geometrically and checks size overflow. Failed append preserves
the old content/ownership. UTF-8 validity is not enforced. Embedded NUL counts
in `len`/`clone`/`equal`, but ordinary C-string output of `view` stops there.
These handles are not reference-counted or safe for concurrent mutation.

## File and console I/O

`io.open(path str, mode str) *void` returns an owned file handle or null.
Paths use UTF-8; Windows converts to UTF-16 for `_wfopen_s`/`_wfopen`.
Supported modes are `r/w/a`, their binary forms `rb/wb/ab`, and update forms
with `+`. Other mode strings fail. Use binary mode for exact byte preservation.

- `io.read(file, data *u8, bytes usize)` and `io.write_bytes(...)` return the
  number of bytes transferred; a short transfer is possible. Call `io.eof(file)`
  and `io.error(file)` to inspect the file state.
- `io.write_file(file, text str)` writes NUL-terminated text and returns bytes.
- `io.flush_file(file)` and `io.close(file)` return success as `bool`. Close
  consumes the handle even if it reports a failure; do not close/use it twice.
- `io.write(text)` and `io.writeln(text)` write stdout and return success.
  `io.flush()` flushes stdout. Unicode console rendering depends on the terminal;
  redirected output contains UTF-8 bytes.
- `io.read_line(data *u8, capacity usize) isize` reads stdin into caller storage,
  adds NUL and excludes LF / CRLF. Returns byte length, `-1` for EOF before any
  bytes, `-2` for error/invalid capacity. Capacity must be at least 2. At most
  capacity minus 1 bytes are read; a long line continues on the next call.

File handles are opaque libc FILE handles, not memory allocations. Release with
`io.close`, not `memory.free`. Null read/write returns zero; null close/flush
fails; `io.error(null)` is true. File-system errors are reported by null, byte
counts and file status; an errno/message API is not implemented yet.

## Time

- `time.now_ns() u64`: monotonic OS clock in nanosecond units; zero on clock
  failure. It is for elapsed time, not calendar/Unix time. Units do not guarantee
  nanosecond measurement resolution.
- `time.now_ms() u64`: the same clock divided by 1,000,000.
- `time.sleep_ms(milliseconds u64) bool`: OS sleep; handles interrupted Linux
  sleeps. Scheduling can extend sleep duration.

Windows uses QueryPerformanceCounter / Sleep; Linux uses clock_gettime /
nanosleep. This runtime library is hosted and does not provide a freestanding
board runtime. The compiler treats these bindings as normal external modules;
hardware programs supply platform libraries/drivers matching their target.

## C usage, emission and validation

```sh
python stdlib/build.py --cc clang --ar llvm-ar
devc emit examples/stdlib/main.dev --module-dir std=stdlib/modules -o out/runtime-generated
python scripts/smoke_stdlib.py --compiler target/release/devc.exe
python stdlib/tests/test_native.py --cc clang
# Linux
python3 scripts/smoke_stdlib.py --compiler target/release/devc --tcc /path/to/tcc
python3 stdlib/tests/test_native.py --sanitize
```

The archive exposes `dvr_*` symbols declared in `stdlib/include/dev_runtime.h`.
The ordinary libc and OS dependencies still apply. Emitted C contains only the
application and imported Dev bindings. Link it to `libdevruntime.a` explicitly.
Runtime C sources and headers belong to the independent runtime build, not the
compiler's emission path. Compiler `check` works with module declarations alone.

Tests cover UTF-8/binary data, self-append across reallocations, independent
clones, null handling, allocation overflow, overlapping copy, deterministic
allocation failures, Unicode filenames, file errors/EOF, stdin boundaries,
clock/sleep, cache reuse/invalidation and preserving output after compile failure.
Linux C tests run with AddressSanitizer and UndefinedBehaviorSanitizer.

Validated: 37 native/stdlib/cache scenarios on Windows/Clang, 49 on Linux with
GCC/TinyCC, standalone compiler-only source builds on each OS, emitted-C compilation with explicit
archive linkage, C consumption, a compiler relocated without runtime dependency,
arbitrary module namespaces, the original 35 compiler scenarios on each OS, 14 Rust tests
on each OS, and Windows Clippy with warnings denied. Linux Clippy is unavailable
in the installed toolchain; no Linux Clippy success is claimed.

This first runtime does not include networking, threads/async, sockets, maps,
automatic ownership, garbage collection or a VM. Raw pointers/opaque handles
remain the programmer's responsibility.
