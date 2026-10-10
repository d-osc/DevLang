# std/buffer

จัดการ binary buffer และอ่าน/เขียน integers

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/buffer"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `buffer.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct Buffer { id i64, length i64 }
fn alloc(size i64, fill u8) Buffer
fn from(text str, encoding str) Buffer
fn fromBytes(bytes Vec<u8>) Buffer
fn concat(buffers Vec<Buffer>) Buffer
fn byteLength(text str, encoding str) i64
fn Buffer.toString(self Buffer, encoding str) str
fn Buffer.toBytes(self Buffer) Vec<u8>
fn Buffer.readUInt8(self Buffer, offset i64) u8
fn Buffer.writeUInt8(self Buffer, value u8, offset i64) bool
fn Buffer.readUInt16LE(self Buffer, offset i64) u16
fn Buffer.readUInt16BE(self Buffer, offset i64) u16
fn Buffer.readUInt32LE(self Buffer, offset i64) u32
fn Buffer.readUInt32BE(self Buffer, offset i64) u32
fn Buffer.writeUInt16LE(self Buffer, value u16, offset i64) bool
fn Buffer.writeUInt16BE(self Buffer, value u16, offset i64) bool
fn Buffer.writeUInt32LE(self Buffer, value u32, offset i64) bool
fn Buffer.writeUInt32BE(self Buffer, value u32, offset i64) bool
fn Buffer.slice(self Buffer, start i64, end i64) Buffer
fn Buffer.equals(self Buffer, other Buffer) bool
fn Buffer.fill(self Buffer, value u8) bool
fn Buffer.copy(self Buffer, target Buffer, targetStart i64, sourceStart i64, sourceEnd i64) i64
fn Buffer.close(self Buffer) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/buffer"

fn main() {
    let bytes = buffer.from("Dev", "utf8")
    print(bytes.toString("utf8"))
    bytes.close()
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/buffer/main.dev
d examples/modules-api/buffer/main.dev --engine ast
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

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../node-core.md)
- [Source ตัวอย่าง](../../examples/modules-api/buffer/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
