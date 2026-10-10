# Node-style core runtime modules

The current source runtime supports `std/net`, `std/path`, `std/os`, `std/stream`,
`std/url`, `std/module`, `std/process`, `std/events`, `std/buffer`, `std/dgram`.
Run with `d FILE.dev` or `d run FILE.dev` without a C compiler. Both auto/AST
engines and editor signature checking are supported. These modules are
runtime-only and require an updated source build; older installers may lack them.

These are typed APIs inspired by Node, not JavaScript/Node compatibility.
Arguments, encodings and timeouts are explicit. Node properties such as
os.platform and module.builtinModules are functions here. Failures produce
source-located runtime errors. There is no catch API or JavaScript Promise.

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

## path

Lexical operations use host separators without filesystem access or symlink
resolution. They do not sandbox paths. No posix/win32 namespaces or varargs.
Windows drive-relative/case edge cases can differ from Node.

| Function | Result |
| --- | --- |
| `join(base str, child str)` | `str`, join and normalize; leading child separator does not discard base |
| `joinMany(parts Vec<str>)` | `str` |
| `resolve(base str, child str)` | `str`, absolute/cwd-relative; absolute child replaces base |
| `normalize(path str)` | `str`, empty becomes `.` |
| `dirname(path str)`, `basename(path str)`, `extname(path str)` | `str` |
| `isAbsolute(path str)` | `bool` |
| `relative(from str, to str)` | `str`, equal paths produce empty text |
| `parse(path str)` | `path.Parts { root str, dir str, base str, ext str, name str }` |
| `format(parts path.Parts)` | `str`, dir/base take precedence over root/name/ext |
| `sep()`, `delimiter()` | `str`, path separator / PATH delimiter |

## os

All functions take no arguments. `platform`, `arch`, `hostname`, `homedir`,
`tmpdir`, `release`, `type`, `endianness`, `EOL` return str.
`availableParallelism`, `uptime`, `totalmem`, `freemem` return i64.
OS uptime is seconds; memory is bytes. Platforms include win32/linux/darwin;
architectures include x64/ia32/arm64. Endianness is LE or BE.
Home uses USERPROFILE on Windows or HOME elsewhere; unavailable values error.
CPU details, load averages, network interfaces and priorities are absent.

## process

| Function | Result |
| --- | --- |
| `cwd()` | `str` |
| `chdir(path str)` | `bool`, changes cwd process-wide, including other tasks |
| `pid()` | `i64` |
| `argv()` | `Vec<str>`: runtime executable, entry file, then args after `--` |
| `execPath()` | `str`, actual runtime executable |
| `env()` | `Map<str,str>`, Unicode environment snapshot |
| `getenv(name str)` | `str`, empty if absent/non-Unicode |
| `hasEnv(name str)` | `bool`, distinguishes missing from empty |
| `platform()`, `arch()` | `str` |
| `uptime()` | `f64`, elapsed seconds of this interpreter instance |
| `hrtime()` | `i64`, elapsed monotonic nanoseconds of this instance |

No environment setter, signals, exit() or child launcher. Use DevLang's file-level
return for exit status. Changing the env map does not change OS environment.

## module

| Function | Result |
| --- | --- |
| `builtinModules()` | `Vec<str>`, implemented canonical std/... names |
| `isBuiltin(name str)` | `bool`, accepts full or short names |
| `resolve(specifier str, from_file str)` | `str`, canonical builtin or existing absolute file |
| `loaded()` | `Vec<str>`, sorted loaded module paths/specifiers |
| `entry()` | `str`, entry file |

Uses DevLang .dev imports and configured namespace directories. Imports remain
static: `use "std/path"`. No require, node: imports, CommonJS/ESM, Node packages,
compile cache or loader hooks.

## url

`parse(input str)` and `resolve(base str, relative str)` return url.URL using
WHATWG parsing. Fields (all str): href, protocol, host, hostname, port, pathname,
search, hash, origin, username, password. Protocol includes `:`, nonempty
search/hash include `?`/`#`, absent port is empty. Fields are record snapshots,
not live URL setters. `URL.toString()` returns canonical href.
`format(value url.URL) str` rebuilds a hierarchical URL from protocol, hostname,
port, pathname, search, hash and credentials, using href as its initial parser
base; it ignores snapshot host/origin. Opaque URLs may not support format.

`pathToFileURL(path str) str` accepts absolute/cwd-relative paths;
`fileURLToPath(input str) str` returns a local path. Spaces, Unicode and fragment
characters are escaped. `domainToASCII(domain str) str` performs IDN/punycode
normalization and rejects URL syntax.

`searchParams(query str)` returns url.URLSearchParams:

| Method | Result |
| --- | --- |
| `append(name str, value str)`, `set(name str, value str)` | `void` |
| `get(name str)` | `str`, first value; missing is empty, use has to distinguish |
| `getAll(name str)` | `Vec<str>` |
| `has(name str)` | `bool` |
| `delete(name str)` | `void`, deletes all matching pairs |
| `toString()` | `str`, form encoding, spaces become + |
| `close()` | `void` |

Order/repeated keys are preserved; set replaces first and removes duplicates.
Limits: 4096 pairs, 8 MiB decoded key/value storage. No iterators, sorting,
live URL-search binding or domainToUnicode.

## buffer

buffer.Buffer contains opaque id and fixed `length i64` snapshot. Live buffers
together are capped at 8 MiB per interpreter; close frees their storage.

| Function | Result |
| --- | --- |
| `alloc(size i64, fill u8)` | `Buffer`, initialized |
| `from(text str, encoding str)` | `Buffer` |
| `fromBytes(bytes Vec<u8>)` | `Buffer`, copy |
| `concat(buffers Vec<buffer.Buffer>)` | `Buffer`, copies in order |
| `byteLength(text str, encoding str)` | `i64`, length of decoded bytes |

Encodings: utf8/utf-8, hex, base64, base64url. Hex/base64 decoding is strict.
Base64url output has no padding; input can have trailing padding. UTF-8 output
replaces invalid sequences. toBytes returns independent managed Vec values.

| Buffer method | Result |
| --- | --- |
| `toString(encoding str)` | `str` |
| `toBytes()` | `Vec<u8>`, copy |
| `readUInt8(offset i64)` | `u8` |
| `readUInt16LE/BE(offset i64)` | `u16` |
| `readUInt32LE/BE(offset i64)` | `u32` |
| `writeUInt8(value u8, offset i64)` | `bool` |
| `writeUInt16LE/BE(value u16, offset i64)` | `bool` |
| `writeUInt32LE/BE(value u32, offset i64)` | `bool` |
| `slice(start i64, end i64)` | `Buffer`, copy; clamped negative indices |
| `equals(other buffer.Buffer)` | `bool` |
| `fill(value u8)` | `bool` |
| `copy(target buffer.Buffer, targetStart i64, sourceStart i64, sourceEnd i64)` | `i64`, bytes copied; overlap-safe |
| `close()` | `void` |

Offsets are checked. Slice copies instead of sharing Node Buffer memory. No unsafe
allocation, signed/float/64-bit operations, subarray views or resizing.

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

## stream

Actual file streams: `createReadStream(path str)` returns stream.Readable;
`createWriteStream(path str, append bool)` returns stream.Writable and
creates/truncates or appends. These factories live in std/stream in this version.

| Method | Result |
| --- | --- |
| `Readable.read(size i64)` | `Vec<u8>`, one read, empty at EOF or size zero |
| `Readable.pipe(destination stream.Writable)` | `i64`, total bytes; closes both handles |
| `Readable.close()` | `void` |
| `Writable.write(bytes Vec<u8>)` | `i64`, bytes written |
| `Writable.end()`, `Writable.close()` | `void`, flush/close |

Read/write chunks are capped at 8 MiB. Pipe uses bounded copying and has no
whole-file 8 MiB limit. Operations are synchronous. No generic stream constructors,
PassThrough/Transform/Duplex, async iterators, data events or backpressure.

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

## Examples and validation

```sh
d examples/core-modules/main.dev
d examples/tcp/main.dev
d examples/udp/main.dev
python scripts/smoke_core_modules.py --bin-dir target/release
```

TCP/UDP examples use loopback ephemeral ports and close their own resources.
Smoke tests cover both engines without a C compiler, real sockets, 9 MiB file
streaming, Buffer bounds, URL encoding, event reentrancy and invalid arguments.
