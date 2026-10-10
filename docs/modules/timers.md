# std/timers

ตั้งเวลา callback และควบคุม event pump

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/timers"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `timers.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct Timer { id i64 }
fn setTimeout(callback fn(Timer) void, delayMs i64) Timer
fn setInterval(callback fn(Timer) void, delayMs i64) Timer
fn setImmediate(callback fn(Timer) void) Timer
fn clearTimeout(timer Timer) bool
fn clearInterval(timer Timer) bool
fn Timer.cancel(self Timer) bool
fn Timer.ticks(self Timer) i64
fn Timer.ref(self Timer) void
fn Timer.unref(self Timer) void
fn Timer.hasRef(self Timer) bool
fn run() void
fn sleep(delayMs i64) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/timers"

fn main() {
    timers.setInterval(fn(t timers.Timer) {
        print(t.ticks())
        if t.ticks() >= 2 { t.cancel() }
    }, 1)
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/timers/main.dev
d examples/modules-api/timers/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

These modules run in `d file.dev` / `d run file.dev`, without a native compiler. Native builds do not yet implement these APIs.

## std/timers

`setTimeout(fn(Timer) void, delayMs)`, `setInterval(fn(Timer) void, delayMs)` and `setImmediate(fn(Timer) void)` return a `Timer`. Unlike Node's timer callback, the callback receives its handle. `Timer.ticks()` counts invocations; `cancel()` returns whether an active timer was removed. `clearTimeout` and `clearInterval` also cancel. `ref`, `unref` and `hasRef` control whether the timer keeps the event pump alive.

Timers run after a successful entry call, or when `timers.run()` explicitly pumps timers and network servers. Delays use a monotonic clock. Timeouts allow 0–300000 ms; intervals allow 1–300000 ms. There are at most 64 active timers per interpreter. Interval delays start after each callback finishes; missed intervals do not accumulate. Callbacks execute sequentially on the interpreter thread. A failed callback removes that timer and propagates its error. Nested event pumps are rejected.

```dev
use "std/timers"
fn main() {
    timers.setInterval(fn(t timers.Timer) {
        print(t.ticks())
        if t.ticks() >= 3 { t.cancel() }
    }, 1)
}
main()
```

Unreferenced timers may run while another referenced timer/server keeps the pump alive. Worker tasks need an explicit pump. `sleep(ms)` blocks the current thread; these APIs are not coroutine-based async I/O. Closures capture values, rather than sharing mutable local counters automatically.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../control-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/timers/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
