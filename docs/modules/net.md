# std/net

TCP clients, servers และ IP validation

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/net"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `net.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct Socket { id i64 }
struct Server { id i64 }
struct Address { address str, port i64, family str }
fn isIP(input str) i64
fn isIPv4(input str) bool
fn isIPv6(input str) bool
fn connect(port i64, host str, timeout_ms i64) Socket
fn createConnection(port i64, host str, timeout_ms i64) Socket
fn createServer(handler fn(Socket) void) Server
fn Server.listen(self Server, port i64) void
fn Server.listenOn(self Server, port i64, host str) void
fn Server.address(self Server) Address
fn Server.close(self Server) void
fn Server.on(self Server, event str, handler fn(Socket) void) void
fn Socket.on(self Socket, event str, handler fn(Vec<u8>) void) void
fn Socket.read(self Socket, size i64) Vec<u8>
fn Socket.write(self Socket, bytes Vec<u8>) i64
fn Socket.end(self Socket, bytes Vec<u8>) void
fn Socket.destroy(self Socket) void
fn Socket.setTimeout(self Socket, timeout_ms i64) void
fn Socket.setNoDelay(self Socket, enable bool) void
fn Socket.address(self Socket) Address
fn Socket.remoteAddress(self Socket) Address
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้ bind loopback port ที่ OS เลือก แล้วปิด handle ทันที ทดสอบการสร้าง resource โดยไม่ต้องมี server ภายนอก; ไม่ได้ทดสอบรับส่งข้อมูล

```dev
use "std/net"

fn main() {
    let server = net.createServer(fn(socket net.Socket) { socket.destroy() })
    server.listenOn(0, "127.0.0.1")
    print(server.address().port > 0)
    server.close()
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/net/main.dev
d examples/modules-api/net/main.dev --engine ast
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

## net

`isIP(input str) i64` returns 0, 4 or 6. isIPv4/isIPv6 return bool.
`connect(port i64, host str, timeout_ms i64)` and alias createConnection return
net.Socket synchronously. Connect timeout is 1..300000 ms and does not bound OS
DNS resolution.

`createServer(handler fn(net.Socket) void)` returns net.Server.
Server methods: `listen(port i64)` (all IPv4 interfaces),
`listenOn(port i64, host str)`, `address()`, `close()`,
`on("connection", handler fn(net.Socket) void)` (replaces connection handler).
Port zero selects a free port; address returns net.Address with address str,
port i64 and family str (IPv4/IPv6). Closing the listener leaves existing sockets.

| Socket method | Result |
| --- | --- |
| `read(size i64)` | `Vec<u8>`, one blocking read; empty on EOF/size zero |
| `write(bytes Vec<u8>)` | `i64`, writes all |
| `end(bytes Vec<u8>)` | `void`, writes then shuts down sending |
| `destroy()` | `void`, closes |
| `setTimeout(timeout_ms i64)` | `void`, read/write timeout; zero disables |
| `setNoDelay(enable bool)` | `void`, TCP_NODELAY |
| `address()`, `remoteAddress()` | `net.Address` |
| `on(event str, handler fn(Vec<u8>) void)` | `void`, adds data/end callback |

Read/write chunks max 8 MiB; callback chunks max 64 KiB. TCP is a byte stream,
so a callback/read is not necessarily one message. End callbacks receive empty
Vec, and callback sockets auto-close on EOF. Blocking read cannot be combined
with data listeners. Default blocking timeout: 10000 ms; setTimeout does not
schedule Node timeout events. No TLS, IPC, keepalive, error/drain events or reconnect.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../node-core.md)
- [Source ตัวอย่าง](../../examples/modules-api/net/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
