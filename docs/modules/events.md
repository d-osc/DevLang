# std/events

ส่ง event ด้วย callback และ listener IDs

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/events"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `events.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct EventEmitter { id i64 }
fn createEmitter() EventEmitter
fn EventEmitter.on(self EventEmitter, event str, listener fn(str) void) i64
fn EventEmitter.once(self EventEmitter, event str, listener fn(str) void) i64
fn EventEmitter.off(self EventEmitter, event str, listener_id i64) bool
fn EventEmitter.emit(self EventEmitter, event str, payload str) bool
fn EventEmitter.listenerCount(self EventEmitter, event str) i64
fn EventEmitter.removeAllListeners(self EventEmitter, event str) i64
fn EventEmitter.eventNames(self EventEmitter) Vec<str>
fn EventEmitter.close(self EventEmitter) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/events"

fn main() {
    let emitter = events.createEmitter()
    emitter.once("hello", fn(text str) { print(text) })
    emitter.emit("hello", "Dev")
    emitter.close()
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/events/main.dev
d examples/modules-api/events/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

The current source runtime supports `std/net`, `std/path`, `std/os`, `std/stream`,
`std/url`, `std/module`, `std/process`, `std/events`, `std/buffer`, `std/dgram`.
Run with `d FILE.dev` or `d run FILE.dev` without a C compiler. Both auto/AST
engines and editor signature checking are supported. These modules are
runtime-only and require an updated source build; older installers may lack them.

These are typed APIs inspired by Node, not JavaScript/Node compatibility.
Arguments, encodings and timeouts are explicit. Node properties such as
os.platform and module.builtinModules are functions here. Failures produce
source-located runtime errors. Recover runtime callback errors with `std/result`; JavaScript Promises are not implemented.

## Resource ownership and callbacks

Buffers, emitters, query parameters, streams and sockets have runtime-owned
handles. Close them explicitly. Handles belong to their interpreter thread;
capturing them in another task produces an error. Create resources inside the
worker that uses them. At most 256 core handles exist per interpreter, separate
from HTTP handles. Do not change handle IDs or snapshot metadata.

TCP/UDP and HTTP callbacks are serviced after main statements finish successfully.
Listeners and sockets with receive callbacks keep the runtime alive; close them
to finish. Callbacks run serially and blocking work delays other callbacks.
A blocking read in main from a server in the same runtime prevents the server
from servicing it: use callbacks in this case. Writes are synchronous and do not
expose Node backpressure, timers, error events or Promise scheduling.

## events

`createEmitter()` returns events.EventEmitter. Listeners are `fn(str) void` and
receive one string payload; serialize complex payloads as JSON or capture them.

| Method | Result |
| --- | --- |
| `on(event str, listener fn(str) void)` | `i64`, listener token |
| `once(event str, listener fn(str) void)` | `i64`, listener token |
| `off(event str, listener_id i64)` | `bool` |
| `emit(event str, payload str)` | `bool`, whether listeners ran |
| `listenerCount(event str)` | `i64` |
| `eventNames()` | `Vec<str>`, first-registration order |
| `removeAllListeners(event str)` | `i64`, removed count |
| `close()` | `void` |

Emission is synchronous, in registration order, with a listener snapshot.
Changes during emit affect the next emit. Once listeners are removed before
invocation, including reentrant emits. An unhandled error event raises a runtime
error. Callback errors abort emission. Max 1024 listeners per emitter.
No arbitrary payload types, prependListener or Promise-based events.once.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../node-core.md)
- [Source ตัวอย่าง](../../examples/modules-api/events/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
