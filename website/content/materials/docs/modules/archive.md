# std/archive

สร้างและอ่าน ZIP/TAR ในหน่วยความจำ

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/archive"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `archive.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
struct Entry { name str, data Vec<u8> }
fn writeZIP(entries Vec<Entry>) Vec<u8>
fn listZIP(data Vec<u8>) Vec<str>
fn readZIP(data Vec<u8>, name str) Vec<u8>
fn writeTAR(entries Vec<Entry>) Vec<u8>
fn listTAR(data Vec<u8>) Vec<str>
fn readTAR(data Vec<u8>, name str) Vec<u8>
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/archive"
use "std/encoding"

fn main() {
    let entries = Vec<archive.Entry>()
    entries.push(archive.Entry("hello.txt", encoding.encode("Hello", "utf8")))
    let bytes = archive.writeZIP(entries)
    print(encoding.decode(archive.readZIP(bytes, "hello.txt"), "utf8"))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/archive/main.dev
d examples/modules-api/archive/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

Six additional runtime modules: `std/regex`, `std/encoding`, `std/crypto`,
`std/compression`, `std/archive`, `std/uuid`. They require the current source build;
native stdlib and existing installers do not provide them. Arguments are required
and signatures are shared with editor checking. Binary APIs use `Vec<u8>`;
`encoding.encode` or `buffer.toBytes` bridges text/Buffer to binary input.

## Archive

`archive.Entry(name str,data Vec<u8>)` represents a file. Build a
`Vec<archive.Entry>` and call writeZIP/writeTAR.

| API | Contract |
| --- | --- |
| `writeZIP(entries)` / `writeTAR(entries)` | Create archive bytes |
| `listZIP(bytes)` / `listTAR(bytes)` | List regular-file names |
| `readZIP(bytes,name)` / `readTAR(bytes,name)` | Read one named file as Vec<u8> |

```dev-runtime
use "std/archive"
use "std/encoding"
let entries = Vec<archive.Entry>()
entries.push(archive.Entry("hello.txt", encoding.encode("hello", "utf8")))
let packed = archive.writeZIP(entries)
print(encoding.decode(archive.readZIP(packed, "hello.txt"), "utf8"))
```

ZIP creation uses DEFLATE; reading supports stored/DEFLATE files. TAR creates
regular GNU-format headers with deterministic timestamps/permissions. To combine
TAR and GZIP, pass writeTAR output into compression.gzip. To save/read archives
as files, use `fs.write_bytes` / `fs.read_bytes`.

Names must be UTF-8 relative slash-separated paths, without empty/dot/parent
components, backslashes, drive colons or NUL; maximum 1024 bytes. Duplicate names,
links, special files and encrypted members are rejected. Directory members are
validated but omitted from listing. Missing files raise errors. Maximum 4096
entries, 8 MiB encoded input/output and 8 MiB total declared file data.
Read operations also bound actual expanded member data. ZIP member CRC is checked
when that member is read; listing is metadata inspection, not a full integrity
scan of every payload. There is **no filesystem extraction** or folder-recursion
API, and no password-protected ZIP, RAR or 7z support.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../data-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/archive/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
