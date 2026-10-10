# std/uuid

สร้างและตรวจ UUID

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/uuid"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `uuid.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn v4() str
fn parse(text str) str
fn isValid(text str) bool
fn version(text str) i64
fn nil() str
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/uuid"

fn main() {
    let id = uuid.v4()
    print(uuid.isValid(id) && uuid.version(id) == 4)
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/uuid/main.dev
d examples/modules-api/uuid/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

Six additional runtime modules: `std/regex`, `std/encoding`, `std/crypto`,
`std/compression`, `std/archive`, `std/uuid`. They require the current source build;
native stdlib and existing installers do not provide them. Arguments are required
and signatures are shared with editor checking. Binary APIs use `Vec<u8>`;
`encoding.encode` or `buffer.toBytes` bridges text/Buffer to binary input.

## UUID

| API | Contract |
| --- | --- |
| `v4() str` | Random RFC UUID version 4, using OS entropy |
| `parse(text) str` | Normalize a parseable UUID to lowercase hyphenated form |
| `isValid(text) bool` | Whether the UUID crate accepts the representation |
| `version(text) i64` | Version nibble (nil is 0) |
| `nil() str` | All-zero UUID |

UUIDs are represented as str; generation is limited to v4 in this release.
Parsing does not require the UUID to be v4. There is no v7 generator yet.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../data-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/uuid/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
