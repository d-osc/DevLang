# std/regex

ค้นหาและแทนที่ข้อความด้วย regular expressions

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/regex"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `regex.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct Regex { id i64 }
struct Match { matched bool, text str, start i64, end i64 }
fn compile(pattern str, flags str) Regex
fn escape(text str) str
fn Regex.test(self Regex, text str) bool
fn Regex.find(self Regex, text str) Match
fn Regex.findAll(self Regex, text str) Vec<str>
fn Regex.captures(self Regex, text str) Vec<str>
fn Regex.split(self Regex, text str) Vec<str>
fn Regex.replace(self Regex, text str, replacement str) str
fn Regex.replaceAll(self Regex, text str, replacement str) str
fn Regex.close(self Regex) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/regex"

fn main() {
    let pattern = regex.compile("[0-9]+", "")
    print(pattern.find("port=3000").text)
    pattern.close()
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/regex/main.dev
d examples/modules-api/regex/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

Six additional runtime modules: `std/regex`, `std/encoding`, `std/crypto`,
`std/compression`, `std/archive`, `std/uuid`. They require the current source build;
native stdlib and existing installers do not provide them. Arguments are required
and signatures are shared with editor checking. Binary APIs use `Vec<u8>`;
`encoding.encode` or `buffer.toBytes` bridges text/Buffer to binary input.

## Regex

```dev-runtime
use "std/regex"
let pattern = regex.compile("[0-9]+", "")
print(pattern.find("port=3000").text)
print(pattern.replaceAll("a1 b2", "#"))
pattern.close()
```

| API | Contract |
| --- | --- |
| `compile(pattern,flags) Regex` | Compile once; flags i/m/s/U or empty |
| `escape(text) str` | Escape regex metacharacters for literal matching |
| `Regex.test(text) bool` | Whether a match exists anywhere |
| `Regex.find(text) Match` | First match; matched/text/start/end fields |
| `Regex.findAll(text) Vec<str>` | All non-overlapping matches |
| `Regex.captures(text) Vec<str>` | First match: group 0 followed by numbered groups |
| `Regex.split(text) Vec<str>` | Split by pattern; preserves empty entries |
| `Regex.replace(text,replacement) str` | Replace first match |
| `Regex.replaceAll(text,replacement) str` | Replace all matches |
| `Regex.close()` | Release compiled handle |

`i` ignores case, `m` enables line anchors, `s` lets dot match newline, `U` swaps
greediness. Duplicate/unknown flags fail. There is no `g` flag: use `findAll` or
`replaceAll`. Unicode matching is enabled. Look-around and backreferences are not
supported. In Dev source, backslashes must be escaped: `"\\d+"`.

Match offsets are UTF-8 **bytes**, end-exclusive. No match returns matched=false,
text="", start/end=-1. Captures returns an empty vector for no match and empty
strings for unmatched optional groups. Replacement supports `$1`, `$name`,
`${name}`, `${1}` and `$$`; unknown groups expand to empty. Use braces before a
suffix (`${1}suffix`). There are no replacement callbacks.

Limits: 32 regex handles per interpreter (also counted in the 256 core handle
limit), 64 KiB pattern/replacement, 2 MiB compiled regex/DFA cache limits, 8 MiB
input/output, 4096 result items. Captured text and replacements are checked before
copying expanded data. Handles belong to their creating interpreter/thread.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../data-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/regex/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
