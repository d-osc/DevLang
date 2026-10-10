# std/time — Native

Native C stdlib: Time

[กลับหน้ารวม module](index.md) · **โหมด:** ต้อง build/link C stdlib; signature ต่างจาก source runtime ดู [Native stdlib](../stdlib.md)

## การ import และเรียกใช้งาน

```dev
use "std/time"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `time.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn now_ns() u64
fn now_ms() u64
fn sleep_ms(milliseconds u64) bool
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/time"

fn main() {
    let start = time.now_ns()
    time.sleep_ms(1 as u64)
    print(time.now_ns() >= start)
    return 0
}
return main()
```

Build จาก repository root แล้วเรียก executable ใน out/ (Windows ใช้ .exe):

```sh
python stdlib/build.py
d build examples/modules-api/native-time/main.dev --release --module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a -o out/native-time
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

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../stdlib.md)
- [Source ตัวอย่าง](../../examples/modules-api/native-time/main.dev)
- [Native bindings](../../stdlib/modules/time.dev)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
