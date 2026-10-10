# std/dgram

UDP sockets และ datagrams

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/dgram"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `dgram.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct Socket { id i64 }
struct Address { address str, port i64, family str }
struct Message { data Vec<u8>, address str, port i64 }
fn createSocket(kind str) Socket
fn Socket.bind(self Socket, port i64, host str) void
fn Socket.send(self Socket, bytes Vec<u8>, port i64, host str) i64
fn Socket.recv(self Socket, size i64, timeout_ms i64) Message
fn Socket.on(self Socket, event str, handler fn(Message) void) void
fn Socket.address(self Socket) Address
fn Socket.setBroadcast(self Socket, enable bool) void
fn Socket.close(self Socket) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้ bind loopback port ที่ OS เลือก แล้วปิด handle ทันที ทดสอบการสร้าง resource โดยไม่ต้องมี server ภายนอก; ไม่ได้ทดสอบรับส่งข้อมูล

```dev
use "std/dgram"

fn main() {
    let socket = dgram.createSocket("udp4")
    socket.bind(0, "127.0.0.1")
    print(socket.address().port > 0)
    socket.close()
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/dgram/main.dev
d examples/modules-api/dgram/main.dev --engine ast
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

## dgram

`createSocket(kind str)` accepts udp4/udp6, returning dgram.Socket. Hosts must
match that family. Sending before bind automatically binds a wildcard ephemeral
address.

| Socket method | Result |
| --- | --- |
| `bind(port i64, host str)` | `void`, port zero selects a free port |
| `send(bytes Vec<u8>, port i64, host str)` | `i64` |
| `recv(size i64, timeout_ms i64)` | `dgram.Message { data Vec<u8>, address str, port i64 }` |
| `on("message", handler fn(dgram.Message) void)` | `void`, replaces receive handler |
| `address()` | `dgram.Address { address str, port i64, family str }` |
| `setBroadcast(enable bool)` | `void`, bound socket required |
| `close()` | `void` |

Send max: 65507 bytes. Receive size: 1..65535; timeout: 1..300000 ms.
Oversized packets are consumed and error instead of silent truncation. Callbacks
receive whole packets, including zero-length ones. Blocking recv cannot be mixed
with a message listener. UDP does not guarantee delivery/order. No multicast,
connect/disconnect or other UDP events.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../node-core.md)
- [Source ตัวอย่าง](../../examples/modules-api/dgram/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
