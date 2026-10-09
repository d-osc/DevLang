# JSON in the source runtime

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

Errors stop runtime execution with source location; there is no try/catch API.
Use `valid`, `has`, `kind` and `len` before parsing/accessing untrusted data when
appropriate. These APIs do not provide application schema validation.

Validation: `python scripts/smoke_json.py --runtime target/release/devrun.exe`
exercises both engines with no compiler in PATH, including malformed input,
Unicode, exact large numbers, copies, typed values, tasks and bounds/type errors.
