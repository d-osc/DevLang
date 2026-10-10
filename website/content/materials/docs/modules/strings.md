# std/strings

จัดการข้อความ Unicode, ค้นหา, แยก และแทนที่ข้อความ

[กลับหน้ารวม module](index.md) · **โหมด:** Source runtime builtin; native stdlib มีชื่อเดียวกันแต่ API/ownership ต่างกัน ดู [native counterpart](native-strings.md)

## การ import และเรียกใช้งาน

```dev
use "std/strings"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `strings.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn len(text str) usize
fn charLength(text str) i64
fn concat(a str, b str) str
fn equal(a str, b str) bool
fn trim(text str) str
fn trimStart(text str) str
fn trimEnd(text str) str
fn toLowerCase(text str) str
fn toUpperCase(text str) str
fn contains(text str, pattern str) bool
fn startsWith(text str, pattern str) bool
fn endsWith(text str, pattern str) bool
fn indexOf(text str, pattern str) i64
fn lastIndexOf(text str, pattern str) i64
fn split(text str, separator str) Vec<str>
fn join(parts Vec<str>, separator str) str
fn replace(text str, pattern str, replacement str) str
fn replaceAll(text str, pattern str, replacement str) str
fn repeat(text str, count i64) str
fn substring(text str, start i64, end i64) str
fn charAt(text str, index i64) str
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/strings"

fn main() {
    print(strings.charLength("Dev 🚀"))
    print(strings.toUpperCase("dev"))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/strings/main.dev
d examples/modules-api/strings/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

The current source runtime adds `std/math`, `std/random`, `std/datetime`,
`std/test`, `std/log`, and expands `std/strings`. Import with `use "std/math"`.
These APIs run without a C compiler. They are not yet provided by native stdlib
or existing release installers. Shared signatures let the editor check calls.
Arguments are required; there are no optional/variadic arguments.

## Strings

| API | Result |
| --- | --- |
| `len(text)` | UTF-8 **byte** length as usize (existing behavior) |
| `charLength(text)` | Unicode scalar count as i64 |
| `charAt(text,index)` | One Unicode scalar as str |
| `substring(text,start,end)` | Unicode scalar range `[start,end)` |
| `concat(a,b)`, `equal(a,b)` | Existing text operations |
| `trim(text)`, `trimStart(text)`, `trimEnd(text)` | Unicode whitespace removal |
| `toLowerCase(text)`, `toUpperCase(text)` | Unicode casing, without locale tailoring |
| `contains(text,pattern)`, `startsWith(text,pattern)`, `endsWith(text,pattern)` | bool |
| `indexOf(text,pattern)`, `lastIndexOf(text,pattern)` | UTF-8 **byte** offset, or -1 |
| `split(text,separator)` | Vec<str>; literal separator, keeps empty entries |
| `join(parts Vec<str>,separator)` | str |
| `replace(text,pattern,replacement)` | Replace first literal occurrence |
| `replaceAll(text,pattern,replacement)` | Replace all literal occurrences |
| `repeat(text,count)` | Repeated str |

An empty split separator returns Unicode scalars, with no extra empty entries.
Unicode scalars are not grapheme clusters (Thai marks and combined emoji can
contain multiple scalars). Negative/out-of-range character indices fail;
substring does not clamp or swap indices. An empty replacement pattern follows
UTF-8 character boundaries. Generated text is limited to 8 MiB and split to 65,536
parts / 8 MiB of text. Use std/regex for regular expressions; Unicode normalization is not implemented.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../basic-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/strings/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
