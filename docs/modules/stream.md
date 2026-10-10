# std/stream

อ่าน/เขียน file stream แบบ bytes

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/stream"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `stream.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct Readable { id i64 }
struct Writable { id i64 }
fn createReadStream(path str) Readable
fn createWriteStream(path str, append bool) Writable
fn Readable.read(self Readable, size i64) Vec<u8>
fn Readable.pipe(self Readable, destination Writable) i64
fn Readable.close(self Readable) void
fn Writable.write(self Writable, bytes Vec<u8>) i64
fn Writable.end(self Writable) void
fn Writable.close(self Writable) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/stream"
use "std/encoding"
use "std/fs"

fn main() {
    let writer = stream.createWriteStream("stream.txt", false)
    writer.write(encoding.encode("Hello", "utf8"))
    writer.end()
    let reader = stream.createReadStream("stream.txt")
    print(encoding.decode(reader.read(32), "utf8"))
    reader.close()
    fs.remove_file("stream.txt")
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/stream/main.dev
d examples/modules-api/stream/main.dev --engine ast
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

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../node-core.md)
- [Source ตัวอย่าง](../../examples/modules-api/stream/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
