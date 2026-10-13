# Dev Object Notation (DON)

DON is a data format for DevLang, with the `.don` extension. It stores JSON-compatible
objects, arrays, strings, booleans, null and decimal numbers. It does not execute
DevLang, expand environment variables, fetch imports or evaluate expressions.
It supports bare `key` references to values in the same document.

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

### Arrays with commas

Commas are supported in multiline arrays and nested objects. Elements can have
mixed types:

```don
items: [
  'hello',
  42,
  true,
  null,
  {
    name: 'Dev',
    age: 18
  }
]
```

The same literal can be used inside `.dev` as `let data = { items: [...] }`.

## DevLang runtime API

### References

```don
version: 'v1.0.0'
package: { name: 'my_app', entry: 'src/main.dev', modules: 'src' }
dependencies: {
  utils: {
    git: 'https://github.com/example/utils.git'
    tag: version
  }
}
```

`version` references a root key. `server.port` follows object fields from the
root. Each path segment must be a bare identifier; quoted keys/array indices are
not supported in reference paths. Forward references are supported. Values retain
their types, including numbers, booleans, arrays and objects, and are copied into
the result. `'@version'` remains a literal string; `tag: version` copies the value.
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

## Package configuration

`d new my_app` creates `package.don`. The package manager reads the **existing
DevLang manifest schema**, shown above: `package.name`, `package.entry`,
`package.modules`, optional root `version` (string), and `dependencies`. Version
is metadata and is checked against dependency `version` requirements when supplied.
A dependency uses `path`, `git` with optional `tag`, `branch` or `version`, `url` with required archive `sha256` and optional `version`, or `workspace: true`. Git selectors are mutually exclusive.
See [workspaces and SemVer](packages.md) for the resolver contracts. Entry/modules paths must be relative without `..`; package
names and dependency namespaces must be Dev identifiers.

```sh
d new my_app
cd my_app
d pkg add math --path ../math
d pkg install
d run
```

`package.don` takes precedence when both it and legacy `dev.toml` exist.
Legacy TOML manifests remain supported. Dependency projects can use either format.
`d pkg add/remove` rewrites the manifest, so comments are not retained. Locking
uses **`package-lock.don` in DON**. Legacy `dev.lock` TOML files remain readable; successful install writes the new DON lock without deleting the old file. When both exist, `package-lock.don` takes precedence.
This does not implement Node `package.json` fields such as scripts, imports,
semver registries or automatic downloads beyond the existing path/Git package APIs.

Runnable example: `d examples/don/main.dev`. Regression tests:
`cargo test -p dev-syntax` and `python scripts/smoke_don.py --bin-dir out/don/debug`.

## CLI data tools

```sh
d don check package.don
d don fmt package.don
d don to-json package.don
d don from-json config.json
```

Formatting and conversion write to stdout and do not change the input file.
Formatting removes comments and normalizes whitespace. `d fmt` continues to
format `.dev` source. The LSP avoids native semantic errors for programs importing
DON, as it does for JSON; full semantic checking of dynamic data is not implemented.

## VS Code support

Extension `n-devs.devlang-language` version 0.2.0 adds `.don` as **Dev Object
Notation**, including syntax colors, file icons, Format Document (Shift+Alt+F),
and the shared `d lsp --stdio` server. Set `devlang.executablePath` to the latest
source-built d; existing installers do not contain these editor additions.

`d don fmt-source FILE` prints formatted source while preserving comments,
references, quoted string contents and key order. The editor calls this command
with unsaved text and applies edits only if the document version still matches.
It uses four spaces and LF outside literal tokens. Invalid input is not rewritten.
`d don fmt` remains the data serializer: it resolves references and drops comments.

DON LSP supports syntax/reference diagnostics, nested outline, value-kind hover,
reference completion and same-document Go to Definition. Missing targets,
cycles, duplicate keys and invalid escapes are errors, including in untitled
DON documents. Positions use UTF-16. No generic schema validation, external-file
references, expression evaluation or rename is provided; the parser reports its
first error. See [extension instructions](../editors/vscode/README.md).

## String expressions

```don
name: 'Dev'
prefix: 'Hello '
message: prefix + name + '!'
message2: "Hello ${name}!"
age: 18
summary: "${name}: ${age}"
literal: "\${name}"
```

`+` concatenates strings only. References use bare names (`name`); the old `@name` syntax is rejected. Double-quoted interpolation accepts root-relative paths such as
`${user.name}` and converts scalar values to text. Single quotes and triple
quotes remain literal. `\${...}` escapes a placeholder. Forward references
work; missing names, cycles, non-string concatenation and expansion over 8 MiB
are rejected. DON does not execute DevLang expressions or function calls.
Available in source builds after v0.5.1.

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

## Object destructuring in DevLang

Extract object fields into variables in the surrounding scope:

```dev
fn main() {
    let user = { name: 'test', age: 10 }
    let {name, age} = user
    print(name) // test
    print(age)  // 10

    let {name: displayName, age: years} = user
    print(displayName)
    print(years)
}
main()
```

The right-hand expression is evaluated once. Bindings are mutable local copies;
changing them does not change the original object. Commas separate bindings and a
trailing comma is allowed. Missing fields report the usual field access error.
Defaults, rest bindings and nested patterns are not supported yet.


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
