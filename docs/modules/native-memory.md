# std/memory — Native

Native C stdlib: Memory

[กลับหน้ารวม module](index.md) · **โหมด:** ต้อง build/link C stdlib; signature ต่างจาก source runtime ดู [Native stdlib](../stdlib.md)

## การ import และเรียกใช้งาน

```dev
use "std/memory"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `memory.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn alloc(bytes usize) *void
fn alloc_array(count usize, size usize) *void
fn resize(data *void, bytes usize) *void
fn free(data *void) void
fn copy(dest *void, source *void, bytes usize) void
fn fill(dest *void, value u8, bytes usize) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/memory"

fn main() {
    unsafe {
        let data = memory.alloc(16 as usize)
        if data == null { return 1 }
        memory.fill(data, 0 as u8, 16 as usize)
        memory.free(data)
        print("released")
    }
    return 0
}
return main()
```

Build จาก repository root แล้วเรียก executable ใน out/ (Windows ใช้ .exe):

```sh
python stdlib/build.py
d build examples/modules-api/native-memory/main.dev --release --module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a -o out/native-memory
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

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../stdlib.md)
- [Source ตัวอย่าง](../../examples/modules-api/native-memory/main.dev)
- [Native bindings](../../stdlib/modules/memory.dev)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
