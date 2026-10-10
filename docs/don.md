# Dev Object Notation (DON)

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

## Package configuration

`d new my_app` creates `package.don`. The package manager reads the **existing
DevLang manifest schema**, shown above: `package.name`, `package.entry`,
`package.modules`, optional root `version` (string), and `dependencies`. Version
is metadata and is checked against dependency `version` requirements when supplied.
A dependency uses `path`, `git` with optional `rev` or `version`, or `workspace: true`.
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
continues to use the existing **`dev.lock` in TOML**, not a new DON/JSON lock schema.
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
