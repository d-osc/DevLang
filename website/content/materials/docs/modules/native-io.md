# std/io — Native

Native C stdlib: File and console I/O

[กลับหน้ารวม module](index.md) · **โหมด:** ต้อง build/link C stdlib; signature ต่างจาก source runtime ดู [Native stdlib](../stdlib.md)

## การ import และเรียกใช้งาน

```dev
use "std/io"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `io.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn open(path str, mode str) *void
fn read(file *void, data *u8, bytes usize) usize
fn write_bytes(file *void, data *u8, bytes usize) usize
fn write_file(file *void, text str) usize
fn flush_file(file *void) bool
fn close(file *void) bool
fn error(file *void) bool
fn eof(file *void) bool
fn write(text str) bool
fn writeln(text str) bool
fn flush() bool
fn read_line(data *u8, capacity usize) isize
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/io"

fn main() {
    io.writeln("Hello native")
    return 0
}
return main()
```

Build จาก repository root แล้วเรียก executable ใน out/ (Windows ใช้ .exe):

```sh
python stdlib/build.py
d build examples/modules-api/native-io/main.dev --release --module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a -o out/native-io
```

## พฤติกรรม, errors และข้อจำกัด

Individual bindings: [memory](native-memory.md), [strings](native-strings.md),
[io](native-io.md), [time](native-time.md). Their APIs differ from source-runtime builtins.

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

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../stdlib.md)
- [Source ตัวอย่าง](../../examples/modules-api/native-io/main.dev)
- [Native bindings](../../stdlib/modules/io.dev)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
