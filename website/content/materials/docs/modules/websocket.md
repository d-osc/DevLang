# std/websocket

ส่งข้อความ WS/WSS แบบ text และ binary

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/websocket"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `websocket.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct Socket { id i64 }
struct Server { id i64 }
struct Address { address str, port i64, family str }
struct Message { kind str, text str, data Vec<u8>, code i64 }
fn connect(url str, timeoutMs i64, caFile str) Socket
fn createServer(callback fn(Socket) void) Server
fn createSecureServer(certFile str, keyFile str, callback fn(Socket) void) Server
fn Socket.sendText(self Socket, text str) void
fn Socket.sendBytes(self Socket, data Vec<u8>) void
fn Socket.receive(self Socket) Message
fn Socket.setTimeout(self Socket, timeoutMs i64) void
fn Socket.path(self Socket) str
fn Socket.origin(self Socket) str
fn Socket.close(self Socket) void
fn Server.listen(self Server, port i64) void
fn Server.listenOn(self Server, port i64, host str) void
fn Server.address(self Server) Address
fn Server.on(self Server, event str, callback fn(Socket) void) void
fn Server.close(self Server) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้ bind loopback port ที่ OS เลือก แล้วปิด handle ทันที ทดสอบการสร้าง resource โดยไม่ต้องมี server ภายนอก; ไม่ได้ทดสอบรับส่งข้อมูล

```dev
use "std/websocket"

fn main() {
    let server = websocket.createServer(fn(socket websocket.Socket) { socket.close() })
    server.listenOn(0, "127.0.0.1")
    print(server.address().port > 0)
    server.close()
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/websocket/main.dev
d examples/modules-api/websocket/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

These modules run in the interpreter. Calls perform blocking network I/O on their owner thread; this is not an async I/O scheduler. TLS uses rustls with bundled public CA roots plus an optional custom CA PEM. Certificate chain and host name verification remain enabled. TLS 1.2/1.3 are supported.

## std/websocket

`connect(url, timeoutMs, caFile)` supports `ws://` and `wss://`. Credentials and fragments in URLs are rejected. `caFile` applies to WSS. `createServer(fn(Socket) void)` creates a WS listener; `createSecureServer(certFile, keyFile, callback)` creates WSS. Server methods match TLS: `listen`, `listenOn`, `address`, `on("connection", callback)`, `close`.

`Socket.sendText(str)`, `sendBytes(Vec<u8>)`, `receive()`, `setTimeout(ms)` and `close()` send and receive messages. `Message` has `kind`, `text`, `data`, `code`: text messages use kind `"text"`, binary messages `"binary"`, and close messages `"close"` with reason and status code. Ping is answered automatically; Pong is consumed internally. Once a peer closes, further application receive/send calls fail; call `close()` to release the handle.

Accepted sockets expose `path()` (including query) and `origin()`. Applications must enforce their own authentication, path rules and Origin allowlists. There is no built-in subprotocol selection or compression negotiation.

## Limits and failure behavior

Timeouts accept 1–300000 ms. Connect attempts share the deadline among resolved addresses. OS DNS resolution may block beyond it, and read/write timeouts apply per I/O operation, not to an entire conversation or handshake. Slow peers can therefore occupy the synchronous server loop. Accepted handshakes default to 10 seconds. Invalid handshakes are discarded; handler errors propagate.

PEM files are capped at 64 KiB, TLS read/write buffers and WebSocket messages/frames at 8 MiB. Limits are 32 TLS sockets, 32 WebSocket sockets and 16 combined TLS/WS servers per interpreter, also subject to the global runtime handle cap. There is no mTLS, ALPN configuration, proxy support, custom TLS-option integration with the HTTP client, or HTTP-server upgrade API yet. These are independent modules: `std/http` retains its existing HTTPS client, while its server and `std/net` sockets have not gained TLS through these additions.

`scripts/smoke_secure_network.py` generates a local test CA and checks TLS/WS/WSS clients and servers against independent Python peers, including certificate rejection and WebSocket masking/ping/close behavior. It requires OpenSSL only for generating test certificates; the Dev runtime does not require OpenSSL.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../secure-network.md)
- [Source ตัวอย่าง](../../examples/modules-api/websocket/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
