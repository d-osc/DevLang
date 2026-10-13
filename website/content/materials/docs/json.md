# JSON in DevLang

`use "std/json"` provides JSON values in the source runtime and hosted native
compiler. Native data support is available in source builds after v0.5.1;
existing v0.5.1 release installers predate it. Source execution requires no C
compiler. Native builds require a C11 backend such as Clang/GCC.

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

## DON-style object literals and member access

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
Updates preserve other copies of the original object. Source execution clones
the JSON tree; native execution shares immutable nodes and copies the update path;
this is not an in-place shared mutable object or a constant-time update.
Assignment currently requires a local JSON root (for example `user.address.city`);
extract JSON held inside a struct/Vec, update it, then assign it back.
The same notation and member operations now work in hosted native builds.
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

## DON-style object literals in DevLang

```dev-runtime
let name: str = 'Dev'
let user = {
    name = name
    age: 18
    greeting: "Hello ${name}"
    address: { city = 'Bangkok' }
}
print(user.name)
print(user.greeting)
```

Inside `.dev`, object literals use DON-style notation: `:` or `=`, newline/comma/
semicolon separators, single-quoted literal strings, double-quoted interpolation,
triple-quoted multiline strings and block comments. Braces remain required.
Values are ordinary DevLang expressions referencing lexical variables; they are
not document-root/forward references as in a `.don` file. Existing JSON-style
objects and `json.stringify` remain supported for data exchange. These object
forms now run in both source engines and the hosted native compiler, including
nested objects, empty/heterogeneous arrays, dot/bracket reads, insertion, member
assignment, compound assignment, snapshots and passing objects across files.
Native builds require a C11 backend and retain data allocations until process
exit; freestanding builds cannot use this allocation-backed representation.
This change does not embed the DON document parser in native executables:
`don.parse`/`don.valid`, document-root references and forward references remain
source-runtime facilities. Available in source builds after v0.5.1.


## Object destructuring / แยก field เป็นตัวแปร

```dev
fn main() {
    let user = { name: "test", age: 10 }
    let {name, age} = user
    print(name)
    print(age)
    let {name: userName, age: userAge} = user
    print(userName)
    print(userAge)
}
main()
```

ใช้ชื่อ field เดิมเป็นตัวแปร หรือใช้ `field: variable` เพื่อเปลี่ยนชื่อ
ค่าทางขวาประเมินครั้งเดียว ตัวแปรที่แยกเป็นสำเนาที่แก้ไขได้ โดยไม่แก้ object เดิม
Bindings use the surrounding scope. The right-hand expression is evaluated once;
changing a binding does not change the original object. Supports runtime and
hosted native builds from current source after v0.5.1. Defaults, rest bindings
and nested binding patterns are not supported. Missing fields report an error.

ดู [ตัวอย่างเต็ม / Full example](#/examples/don-destructuring).

## Native builds

```sh
cargo build --workspace --release --locked
./target/release/d run examples/json/literals.dev
./target/release/d build examples/json/literals.dev --release -o out/objects
./out/objects
```

On Windows use `d.exe` and `out/objects.exe`. `std/json` is a compiler builtin
for hosted native builds; no external C library or module mapping is required.
The documented constructors, parse/valid, accessors, queries, keys, persistent
updates, value/stringify/pretty are supported. Structs, fixed arrays, Vec, Slice,
str-keyed Map and nonrecursive Ref values can be encoded. Dynamic scalar reads
retain the language's numeric type rules: use an explicit `as i32` to narrow an
i64 field; `json.get`/`json.at` remain boxed and require typed accessors.

Native nodes and generated strings are retained until process exit. Repeated
construction/updates can grow memory; this backend currently has no garbage
collector. Native strings follow the C null-terminated representation; embedded
NUL data requires the source runtime (native JSON parse reports an error rather
than truncating it). Float serialization uses `%.17g`, so text can differ from
Rust's shortest-number formatting. Native DON encoding quotes non-ASCII keys
conservatively. Recursive Ref encoding and runtime intrinsic function values
require source execution; native calls can be wrapped in ordinary Dev functions.
`don.stringify` and `don.toJSON` work natively, but `don.parse`/`don.valid` still
require the source runtime. `.dev` literals use lexical variables rather than
DON document-root/forward references.


## Object methods and this

```dev
fn main() {
    let user = {
        name: "Dev"
        greet: fn(): str {
            return "Hello ${this.name}"
        }
    }
    print(user.greet())
    user.name = "Alice"
    print(user.greet())
}
main()
```

Object literals with `fn` fields create an anonymous typed record. Methods receive
an implicit `this` containing a read-only snapshot of the object used for that
call. Methods can call other methods through `this.method()`. Parameter and
return annotations use ordinary function syntax. Both runtime and native builds
support this form in current source after v0.5.1.

`this` อ่านค่าของ object ที่เรียก method ณ เวลานั้น และเรียก method อื่นได้
แต่ยังไม่รองรับการแก้ object เดิมผ่าน `this` หรือการ capture ตัวแปรนอก method
ให้เก็บค่าที่ต้องใช้เป็น field แล้วอ่าน `this.field` แทน

Data fields have fixed inferred types; adding new fields or replacing methods
is not supported on method objects. Fixed array data fields are supported.
Detached method values are not supported yet. `json.stringify` serializes data
fields only. A `.don` data document cannot contain functions or `this`.
See [ตัวอย่างเต็ม / Full example](#/examples/object-methods).

## Computed data field access on method objects

```dev
fn main() {
    let person = {
        name: "Dev"
        age: 18
        greet: fn(): str { return "Hello ${this.name}" }
    }
    let x = "name"
    print(person[x])
    person[x] = "Alice"
    print(person.greet())
    x = "age"
    person[x] += 1
    print(person[x])
}
main()
```

ใช้ string เลือก data field ได้ทั้งอ่านและแก้ค่า แม้ object จะมี methods
Key expressions are evaluated once. Missing data fields and incompatible writes
report an error. Field types remain fixed; new fields cannot be inserted.
`const` and read-only `this` reject writes. Methods still use `person.greet()`;
computed method calls and detached methods are not supported.
Writes currently target a local object directly (`person[key] = value`);
writes through computed chains or temporary objects are not supported yet.
Available in current source after v0.5.1 in both runtime and hosted native builds.
See [ตัวอย่างเต็ม / Full example](#/examples/object-keys).

## Array tools / การใช้งาน array เพิ่มเติม

```dev
fn make(): [i64; 3] { return [1, 2, 3] }
fn main() {
    let values = make()
    let [first, second, ...rest] = values
    print(first)
    print(rest[0])
    let combined = [0, ...values, ...rest, 4]
    for item in combined { print(item) }
    let middle = values[1..3]
    print(middle[0])
    let zeros: [i32; 4] = [0; 4]
    print(zeros.len())
}
main()
```

- `let [a, b] = values` และ `const [a, b] = values`: แยกสมาชิกตาม index
- `[...values, extra]`: รวม Array, Vec และ Slice เป็น `Vec<T>` ชนิดเดียวกัน
- `let [first, ...rest] = values`: rest ต้องอยู่ท้ายสุด และได้ Vec สำเนาที่แก้ได้
- `for item in values`: วน Array, Vec, Slice หรือ DON array บน snapshot ที่ประเมินครั้งเดียว
- `values[start..end]`: Slice snapshot แบบไม่รวม end ต้องระบุทั้งสอง bounds
- `[value; count]`: fixed array ซ้ำค่า ประเมิน value ครั้งเดียว count เป็น integer literal 1..1000000
- Fixed arrays รองรับ copy, assignment, parameters, return, closures และ data fields ของ object methods ใน native

Spread creates a mutable Vec with one element type; rest bindings are Vec snapshots.
Range slicing creates a read-only Slice. Foreach snapshots the input and evaluates
it once; modifying the original collection does not change the iteration set.
Break and continue retain their normal loop behavior. Fixed arrays pass and return
by value in ordinary Dev functions and function values; native uses an internal
aggregate carrier. C FFI array aggregates remain unsupported. Local fixed arrays
remain one-dimensional; use Vec<Vec<T>> or nested DON arrays for matrices.
Destructuring holes/defaults, open-ended slices and runtime repeat counts are not
supported yet. These expression forms belong to `.dev`; `.don` remains a data format.
Available in current source after v0.5.1 in runtime and native modes.
See [ตัวอย่างเต็ม / Full example](#/examples/array-tools).
