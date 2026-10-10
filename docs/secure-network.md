# TLS and WebSocket

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

## std/websocket

`connect(url, timeoutMs, caFile)` supports `ws://` and `wss://`. Credentials and fragments in URLs are rejected. `caFile` applies to WSS. `createServer(fn(Socket) void)` creates a WS listener; `createSecureServer(certFile, keyFile, callback)` creates WSS. Server methods match TLS: `listen`, `listenOn`, `address`, `on("connection", callback)`, `close`.

`Socket.sendText(str)`, `sendBytes(Vec<u8>)`, `receive()`, `setTimeout(ms)` and `close()` send and receive messages. `Message` has `kind`, `text`, `data`, `code`: text messages use kind `"text"`, binary messages `"binary"`, and close messages `"close"` with reason and status code. Ping is answered automatically; Pong is consumed internally. Once a peer closes, further application receive/send calls fail; call `close()` to release the handle.

Accepted sockets expose `path()` (including query) and `origin()`. Applications must enforce their own authentication, path rules and Origin allowlists. There is no built-in subprotocol selection or compression negotiation.

## Limits and failure behavior

Timeouts accept 1–300000 ms. Connect attempts share the deadline among resolved addresses. OS DNS resolution may block beyond it, and read/write timeouts apply per I/O operation, not to an entire conversation or handshake. Slow peers can therefore occupy the synchronous server loop. Accepted handshakes default to 10 seconds. Invalid handshakes are discarded; handler errors propagate.

PEM files are capped at 64 KiB, TLS read/write buffers and WebSocket messages/frames at 8 MiB. Limits are 32 TLS sockets, 32 WebSocket sockets and 16 combined TLS/WS servers per interpreter, also subject to the global runtime handle cap. There is no mTLS, ALPN configuration, proxy support, custom TLS-option integration with the HTTP client, or HTTP-server upgrade API yet. These are independent modules: `std/http` retains its existing HTTPS client, while its server and `std/net` sockets have not gained TLS through these additions.

`scripts/smoke_secure_network.py` generates a local test CA and checks TLS/WS/WSS clients and servers against independent Python peers, including certificate rejection and WebSocket masking/ping/close behavior. It requires OpenSSL only for generating test certificates; the Dev runtime does not require OpenSSL.
