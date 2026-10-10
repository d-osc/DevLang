# std/yaml

แปลง YAML subset กับ JSON value model

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/yaml"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `yaml.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn parse(text str) json.Value
fn valid(text str) bool
fn stringify<T>(value T) str
fn toJSON(text str) str
fn fromJSON(text str) str
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/yaml"

fn main() {
    print(yaml.toJSON("name: Dev\nactive: true\n"))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/yaml/main.dev
d examples/modules-api/yaml/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

These are interpreter APIs; run with `d file.dev`. Use `std/result` to catch failures. General text/model/output limits are 8 MiB; this is not a guarantee that total process memory stays below 8 MiB.

## std/toml and std/yaml

Both expose `parse(text) json.Value`, `valid(text) bool`, `stringify<T>(value) str`, `toJSON(text) str` and `fromJSON(text) str`. They use Dev's JSON value model and support serializable structs and object literals. `valid` checks this supported subset and its budgets, rather than every feature of the format specification.

TOML serialization requires an object root. TOML has no null value; nulls, nonfinite numbers and integers outside i64 are rejected. Date/time values parse as strings; serializing them again does not restore a date/time type automatically.

YAML accepts a single document, strict boolean spellings, and string object keys. Duplicate keys, merge keys (`<<`), unsupported tags, nonfinite numbers, and filesystem includes are rejected. Simple anchors/aliases work within limits: 64 anchors, 64 aliases, 4096 replay events, 16 replays per anchor, replay depth 32; overall depth 128, 65536 nodes and 131072 events. Comments and custom tags are not preserved. There is no environment substitution or file inclusion.

```dev
use "std/yaml"
use "std/toml"
fn main() {
    print(yaml.toJSON("name: Dev\nactive: true\n"))
    print(toml.toJSON("name = 'Dev'\nactive = true\n"))
}
main()
```

See [the executable example](../../examples/storage-libs/main.dev). `scripts/smoke_storage_libs.py` checks typed SQLite rows, SQL binding, transactions, budgets, and Python SQLite/CSV/TOML interoperability.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../storage-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/yaml/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
