# std/log

เขียน log ตามระดับไปยัง stderr หรือไฟล์

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/log"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `log.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn setLevel(level str) void
fn toFile(path str) void
fn toStderr() void
fn debug(message str) void
fn info(message str) void
fn warn(message str) void
fn error(message str) void
fn flush() void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/log"
use "std/fs"

fn main() {
    log.toFile("module.log")
    log.info("Hello DevLang")
    log.flush()
    log.toStderr()
    print(fs.exists("module.log"))
    fs.remove_file("module.log")
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/log/main.dev
d examples/modules-api/log/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

The current source runtime adds `std/math`, `std/random`, `std/datetime`,
`std/test`, `std/log`, and expands `std/strings`. Import with `use "std/math"`.
These APIs run without a C compiler. They are not yet provided by native stdlib
or existing release installers. Shared signatures let the editor check calls.
Arguments are required; there are no optional/variadic arguments.

## Logging

| API | Meaning |
| --- | --- |
| `setLevel(level str)` | debug/info/warn/error/off; default info |
| `debug(message)`, `info(message)`, `warn(message)`, `error(message)` | Emit at the selected level |
| `toFile(path str)` | Open append-only log output; replace this interpreter's destination |
| `toStderr()` | Flush/close file output and switch to stderr |
| `flush()` | Flush current output |

Lines contain UTC RFC3339 timestamp, level and message. CR/LF in a message are
escaped to keep each message on one line. Messages are limited to 8 MiB; writes
are synchronous and flushed. I/O errors propagate. Files close when switched or
when the interpreter exits. Task interpreters have separate log settings; writes
from multiple processes/tasks are not globally coordinated. Rotation, structured
JSON logging, remote sinks and automatic log retention are not implemented.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../basic-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/log/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
