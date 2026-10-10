# std/tls

เชื่อมต่อและเปิด server TLS พร้อมตรวจ certificate

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/tls"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `tls.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct Options { serverName str, caFile str, timeoutMs i64 }
struct Socket { id i64 }
struct Server { id i64 }
struct Address { address str, port i64, family str }
fn options() Options
fn connect(port i64, host str, options Options) Socket
fn createServer(certFile str, keyFile str, callback fn(Socket) void) Server
fn Socket.read(self Socket, size i64) Vec<u8>
fn Socket.write(self Socket, data Vec<u8>) i64
fn Socket.setTimeout(self Socket, timeoutMs i64) void
fn Socket.close(self Socket) void
fn Server.listen(self Server, port i64) void
fn Server.listenOn(self Server, port i64, host str) void
fn Server.address(self Server) Address
fn Server.on(self Server, event str, callback fn(Socket) void) void
fn Server.close(self Server) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างเริ่มต้นนี้แสดง options โดยไม่ติดต่อเครือข่ายหรือเริ่ม process ดูตัวอย่างใช้งานจริงในสัญญา API ด้านล่าง

```dev
use "std/tls"

fn main() {
    let options = tls.options()
    print(options.timeoutMs)
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/tls/main.dev
d examples/modules-api/tls/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

These modules run in the interpreter. Calls perform blocking network I/O on their owner thread; this is not an async I/O scheduler. TLS uses rustls with bundled public CA roots plus an optional custom CA PEM. Certificate chain and host name verification remain enabled. TLS 1.2/1.3 are supported.

## std/tls

`options()` returns `Options(serverName, caFile, timeoutMs)` with defaults `""`, `""`, 10000 ms. `connect(port, host, options)` returns `Socket`. Empty `serverName` uses the connection host. `caFile` adds trusted certificates; server files must contain PEM certificates and a matching private key.

`Socket.read(size)` returns up to that many bytes (an empty vector means EOF). `write(Vec<u8>)` writes and flushes the entire buffer, returning the byte count. `setTimeout(ms)` changes read/write timeouts. `close()` removes the handle and sends close-notify.

`createServer(certFile, keyFile, fn(Socket) void)` returns `Server`. Call `listen(port)` or `listenOn(port, host)`; port zero selects an ephemeral port. `address()` returns `Address(address, port, family)`. `on("connection", callback)` replaces the handler, and `close()` stops listening. Handlers run through the common runtime event pump after the entry call, or through `timers.run()`.

```dev
use "std/tls"
use "std/encoding"
fn main() {
    let s = tls.connect(443, "example.com", tls.options())
    s.write(encoding.encode("GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n", "utf8"))
    print(encoding.decode(s.read(4096), "utf8"))
    s.close()
}
main()
```

## Limits and failure behavior

Timeouts accept 1–300000 ms. Connect attempts share the deadline among resolved addresses. OS DNS resolution may block beyond it, and read/write timeouts apply per I/O operation, not to an entire conversation or handshake. Slow peers can therefore occupy the synchronous server loop. Accepted handshakes default to 10 seconds. Invalid handshakes are discarded; handler errors propagate.

PEM files are capped at 64 KiB, TLS read/write buffers and WebSocket messages/frames at 8 MiB. Limits are 32 TLS sockets, 32 WebSocket sockets and 16 combined TLS/WS servers per interpreter, also subject to the global runtime handle cap. There is no mTLS, ALPN configuration, proxy support, custom TLS-option integration with the HTTP client, or HTTP-server upgrade API yet. These are independent modules: `std/http` retains its existing HTTPS client, while its server and `std/net` sockets have not gained TLS through these additions.

`scripts/smoke_secure_network.py` generates a local test CA and checks TLS/WS/WSS clients and servers against independent Python peers, including certificate rejection and WebSocket masking/ping/close behavior. It requires OpenSSL only for generating test certificates; the Dev runtime does not require OpenSSL.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../secure-network.md)
- [Source ตัวอย่าง](../../examples/modules-api/tls/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
