# std/json

JSON values, object literals และ typed accessors

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/json"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `json.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Value เป็น managed data ที่สร้างผ่าน API ไม่ใช่ empty struct ที่ใช้แทน JSON/DON โดยตรง

```text
struct Value {}
fn parse(text str) Value
fn get(value Value, key str) Value
fn at(value Value, index i64) Value
fn object() Value
fn array() Value
fn null_value() Value
fn remove(value Value, key str) Value
```

JSON มี dynamic intrinsics เพิ่มจาก shared declarations ข้างต้น เช่น value/stringify/pretty/set/push และ scalar accessors รายการ API เต็มพร้อมชนิดผลลัพธ์อยู่ในตารางด้านล่าง

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/json"

fn main() {
    let user = { name: "Dev", age: 18 }
    print(user.name)
    print(json.int(json.get(user, "age")))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/json/main.dev
d examples/modules-api/json/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

`use "std/json"` provides managed JSON values without a C library or compiler.
This is a new source-runtime API, available after building the current repository.
The existing v0.4.0 installers do not include it. Native `d check`/`d build` do not
implement this module; native JSON bindings remain separate future work.

```dev
use "std/json"
fn main() {
    let data = json.parse("{\"name\":\"Dev\",\"age\":18}")
    print(json.string(json.get(data, "name")))
    data = json.set(data, "age", 21)
    print(json.stringify(data))
}
main()
```

Run `target/release/d.exe examples/json/main.dev` after
`cargo build --release -p dev-runtime -p dev-cli`. On Linux use
`target/release/d` and the same `.dev` source. The launcher must resolve the newly
built sibling `devrun`; older installed copies cannot supply the new module.
The complete example creates/overwrites `devlang-json-example.json` in cwd.

## Object literals and member access

```dev
use "std/json"
let user = { name: "Dev", age: 18, address: { city: "Bangkok" }, tags: [1, "two", null] }
let original = user
user.age += 1
user.address.city = "Chiang Mai"
user.active = true
print(user.name)
print(user["age"])
print(json.stringify(user))
print(original.age)
```

Object literals work without an import; import `std/json` to call its API or
name the `json.Value` type. Bare and quoted keys, empty objects, trailing commas,
and nested objects are supported. Arrays inside literals or assigned JSON members
may be empty or heterogeneous. Ordinary Dev arrays keep their existing rules.
Duplicate literal keys are errors. Keys containing punctuation require brackets.

Dot/bracket reads unwrap strings, booleans, i64/u64 integers and finite floating
numbers into Dev scalars; null becomes `null`. Nested objects/arrays remain JSON.
Numbers beyond these ranges stay JSON to preserve precision. `get`/`at` always
return JSON values, so use them with strict accessors such as `json.int`.
These reads also work on parsed JSON. Missing keys and invalid indices are errors.

Member assignment can insert keys or change their type; compound assignment
requires an existing compatible scalar. Nested parents must already exist and
array indices must be in bounds. Index expressions run once per assignment.
Updates preserve other copies of the original object and clone the JSON tree;
this is not an in-place shared mutable object or a constant-time update.
Assignment currently requires a local JSON root (for example `user.address.city`);
extract JSON held inside a struct/Vec, update it, then assign it back.
Object syntax currently runs in the source runtime; native compilation is unsupported.
See `examples/json/literals.dev` for a complete program.

## API

| Function | Result / behavior |
| --- | --- |
| `parse(text str)` | `json.Value`; malformed JSON is a located runtime error |
| `valid(text str)` | `bool`; syntax and nesting validity without throwing |
| `value(dev_value)` | JSON copy of supported Dev data |
| `stringify(value)` | compact JSON `str` from JSON or supported Dev data |
| `pretty(value)` | JSON `str` with indentation |
| `object()`, `array()`, `null_value()` | empty object, empty array, JSON null |
| `kind(value)` | `str`: `null`, `bool`, `number`, `string`, `array`, `object` |
| `is_null(value)` | `bool` |
| `get(object, key str)` | JSON member; missing keys are errors |
| `has(object, key str)` | `bool`; distinguishes missing keys from JSON null |
| `at(array, index integer)` | JSON element; negative/out-of-range indices are errors |
| `len(object_or_array)` | `usize`; other kinds are errors |
| `keys(object)` | sorted `Vec<str>` |
| `string(value)` | `str`; requires a JSON string |
| `bool(value)` | `bool`; requires a JSON boolean |
| `int(value)` | `i64`; requires an integer representable in i64 |
| `uint(value)` | `u64`; requires a nonnegative integer representable in u64 |
| `float(value)` | finite `f64`; accepts numbers representable in f64, may round |
| `set(object, key str, value)` | new JSON object with a member inserted/replaced |
| `remove(object, key str)` | new JSON object; absent key is a no-op |
| `push(array, value)` | new JSON array with an appended element |

Accessors accept JSON values, not an implicit Dev struct or Map. Convert first
with `value`. `set`/`push` accept supported Dev values for their new elements.
`json.Value` can be used in typed parameters/returns and managed collections.
Use an explicit `json.Value` annotation when capturing results of dynamic
construction APIs such as `set` in a closure. There is no automatic conversion
from a JSON object into a user struct; read and validate fields explicitly.

## Data and ownership

Supported conversion: bool, str, signed/unsigned integers, finite floats,
fixed arrays, Vec, Slice, structs (field names become object keys), Ref to a
supported value, Map with string keys, existing JSON, and raw null as JSON null.
Non-null pointers, function values, tasks, enum values and non-string Map keys
are rejected. Managed data remains owned; no manual JSON free is necessary.

Copies share immutable JSON storage. Updates are functional: always assign the
returned value. They copy the affected JSON subtree; large repeated updates can
be expensive. Fetching nested values copies their subtree. To edit a nested
object, update the child and set that child back into its parent.

JSON strings retain UTF-8, escapes and embedded NUL bytes; serialization escapes
control characters. Surrogate pairs are decoded, invalid lone surrogates are
rejected. Never pass an embedded-NUL string to C FFI string arguments.
Object keys serialize in sorted order. Duplicate parsed keys keep the last value.
Integer/decimal JSON number text supports arbitrary precision during parse and
serialization; Dev scalar accessors enforce their own representable ranges.
Formatting/whitespace is regenerated rather than preserving the source text.
NaN/Infinity from Dev floats cannot be serialized. Nesting is bounded to protect
recursive parsing/conversion; the parser uses serde_json's default recursion
limit, and constructed trees reject depth greater than 128.

Errors include source locations. Use `std/result` callbacks to recover runtime failures; there is no try/catch syntax.
Use `valid`, `has`, `kind` and `len` before parsing/accessing untrusted data when
appropriate. These APIs do not provide application schema validation.

Validation: `python scripts/smoke_json.py --runtime target/release/devrun.exe`
exercises both engines with no compiler in PATH, including malformed input,
Unicode, exact large numbers, copies, typed values, tasks and bounds/type errors.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../json.md)
- [Source ตัวอย่าง](../../examples/modules-api/json/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
