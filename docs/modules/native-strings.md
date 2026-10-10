# std/strings — Native

Native C stdlib: Strings

[กลับหน้ารวม module](index.md) · **โหมด:** ต้อง build/link C stdlib; signature ต่างจาก source runtime ดู [Native stdlib](../stdlib.md)

## การ import และเรียกใช้งาน

```dev
use "std/strings"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `strings.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn new(text str) *void
fn clone(handle *void) *void
fn len(handle *void) usize
fn view(handle *void) str
fn append(handle *void, text str) bool
fn append_bytes(handle *void, data *u8, bytes usize) bool
fn equal(a *void, b *void) bool
fn clear(handle *void) void
fn free(handle *void) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/strings"

fn main() {
    unsafe {
        let text = strings.new("Hello")
        if text == null { return 1 }
        print(strings.view(text))
        strings.free(text)
    }
    return 0
}
return main()
```

Build จาก repository root แล้วเรียก executable ใน out/ (Windows ใช้ .exe):

```sh
python stdlib/build.py
d build examples/modules-api/native-strings/main.dev --release --module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a -o out/native-strings
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

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../stdlib.md)
- [Source ตัวอย่าง](../../examples/modules-api/native-strings/main.dev)
- [Native bindings](../../stdlib/modules/strings.dev)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
