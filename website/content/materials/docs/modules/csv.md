# std/csv

อ่านและเขียนตาราง CSV

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/csv"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `csv.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
struct Table { headers Vec<str>, rows Vec<Vec<str>> }
fn parse(text str, delimiter str, hasHeaders bool) Table
fn stringify(table Table, delimiter str) str
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/csv"

fn main() {
    let table = csv.parse("name,age\nDev,18\n", ",", true)
    print(table.headers[0])
    print(table.rows[0][0])
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/csv/main.dev
d examples/modules-api/csv/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

These are interpreter APIs; run with `d file.dev`. Use `std/result` to catch failures. General text/model/output limits are 8 MiB; this is not a guarantee that total process memory stays below 8 MiB.

## std/csv

`parse(text, delimiter, hasHeaders)` returns `Table(headers Vec<str>, rows Vec<Vec<str>>)`. `stringify(table, delimiter)` emits CSV. Delimiters are a single ASCII byte other than NUL, quote, CR or LF. Quoting, escaped quotes, embedded newlines, Unicode, CRLF and tab delimiters are supported. Every value is a string; duplicate header names are preserved.

Rows must have consistent widths. Limits are 4096 data rows and 256 columns. Blank lines are ignored. The CSV parser follows the `csv` crate's permissive quoting behavior; it is not a strict validator for every malformed RFC 4180 quote sequence. NUL-containing input/cells are rejected.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../storage-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/csv/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
