# std/don

Dev Object Notation พร้อม references และ object access

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/don"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `don.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Value เป็น managed data ที่สร้างผ่าน API ไม่ใช่ empty struct ที่ใช้แทน JSON/DON โดยตรง

```text
struct Value {}
fn parse(text str) Value
fn valid(text str) bool
fn stringify<T>(value T) str
fn toJSON<T>(value T) str
fn fromJSON(text str) Value
fn get(value Value, key str) Value
fn at(value Value, index i64) Value
fn has(value Value, key str) bool
fn string(value Value) str
fn int(value Value) i64
fn bool(value Value) bool
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/don"

fn main() {
    let config = don.parse("version: 'v1.0.0'\nrev: @version")
    print(config.rev)
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/don/main.dev
d examples/modules-api/don/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

DON is a data format for DevLang, with the `.don` extension. It stores JSON-compatible
objects, arrays, strings, booleans, null and decimal numbers. It does not execute
DevLang, expand environment variables, fetch imports or evaluate expressions.
It supports explicit `@key` references to values in the same document.

```don
# package.don
package: {
  name: 'my_app'
  entry: 'src/main.dev'
  modules: 'src'
}
dependencies: {
  math: { path: '../math' }
}
```

## Syntax

- Outer object braces are optional. Nested objects require `{ ... }`.
- Keys may be identifiers (`name`, `server-port`, Unicode names, `_name`, `$name`)
  or quoted strings (`"@app"`, `"key.with.dots"`). Dots do not expand nested keys.
- Use either `:` or `=` between a key and value.
- Separate object entries and array elements with a newline, comma or semicolon.
  Trailing commas/semicolons are allowed. Spaces alone do not separate entries.
- Comments: `# line`, `// line`, `/* block */`. Block comments do not nest.
- Single or double quoted strings support `\n`, `\r`, `\t`, `\b`, `\f`, escaped
  quotes, backslash, slash and JSON Unicode escapes (including surrogate pairs).
- Triple double quotes preserve raw multiline text; no escapes or interpolation.
- Strings must be quoted, including paths, URLs, versions and dates.
- Booleans and null are lowercase: `true`, `false`, `null`.
- Numbers follow JSON decimal syntax, with optional `_` between digits:
  `1_000`, `-12.5`, `1.2e3`. Hex, Infinity and NaN are unsupported.
- Duplicate object keys are errors. Parsing reports line and character column.
- Valid JSON is valid DON. An empty/comment-only document is an empty object.
- Maximum UTF-8 input: 8 MiB; nesting limit: 128 levels. Large integer text is
  preserved by the underlying arbitrary-precision JSON representation.

```don
name = 'Demo'
server: {
  host: "127.0.0.1"
  port: 3_000
  enabled: true
}
tags: ["dev", "don",]
description: """A multiline
description with no escapes."""
```

## DevLang runtime API

### References

```don
version: 'v1.0.0'
package: { name: 'my_app', entry: 'src/main.dev', modules: 'src' }
dependencies: {
  utils: {
    git: 'https://github.com/example/utils.git'
    rev: @version
  }
}
```

`@version` references a root key. `@server.port` follows object fields from the
root. Each path segment must be a bare identifier; quoted keys/array indices are
not supported in reference paths. Forward references are supported. Values retain
their types, including numbers, booleans, arrays and objects, and are copied into
the result. `'@version'` remains a literal string; `rev: version` is invalid.
Missing targets and direct/indirect cycles are errors. Reference expansion has
an 8 MiB data budget and a 128-level traversal limit to reject expansion bombs.
Serialization and `d pkg add/remove` write resolved values, not original references.

```dev-runtime
use "std/don"
fn main() {
    let config = don.parse("name: 'Demo'\nport: 3_000")
    print(config.name)
    print(config.port)
    config.port = 8080
    print(don.toJSON(config))
}
main()
```

| API | Behavior |
| --- | --- |
| `don.parse(text)` | Parse into a managed `don.Value`; invalid data raises an error |
| `don.valid(text)` | Return whether the document parses |
| `don.stringify(value)` | Write DON with bare keys where possible, indentation and newline separators |
| `don.toJSON(value)` | Write compact, standard JSON |
| `don.fromJSON(text)` | Parse strict JSON into a `don.Value` |
| `don.get(value,key)` / `don.at(value,index)` | Return managed child values |
| `don.has(value,key)` | Check whether an object contains a key |
| `don.string(value)` / `don.int(value)` / `don.bool(value)` | Strict scalar accessors |

Use dot/bracket access, nested assignment and heterogeneous arrays with the same
managed data behavior as JSON. Missing keys raise errors; check `has` first.
`stringify` and `toJSON` also accept supported Dev scalar/collection/record values.
Parsed values preserve data, not comments, formatting or original key order.
Objects are serialized in key order. These runtime APIs require the updated
`devrun`; native compilation of `std/don` is unsupported.

Read/write files with `std/fs`:

```dev-runtime
use "std/don"
use "std/fs"
fn main() {
    let value = don.parse("name: 'Demo'")
    fs.write_text("demo.don", don.stringify(value))
    let loaded = don.parse(fs.read_text("demo.don"))
    print(loaded.name)
    fs.remove_file("demo.don")
}
main()
```

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../don.md)
- [Source ตัวอย่าง](../../examples/modules-api/don/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
