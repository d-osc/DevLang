# SQLite, CSV, TOML and YAML

These are interpreter APIs; run with `d file.dev`. Use `std/result` to catch failures. General text/model/output limits are 8 MiB; this is not a guarantee that total process memory stays below 8 MiB.

## std/sqlite

SQLite is bundled into the runtime. `sqlite.open(path)` opens a database (`":memory:"` is supported); `version()` returns the SQLite version. Up to 16 databases can be open per interpreter; handles belong to that interpreter.

`Database.execute(sql, params Vec<sqlite.Value>)` returns affected rows. `query` returns `Rows(columns Vec<str>, rows Vec<Vec<Value>>)`. `lastInsertRowId`, `setTimeout(ms)` and `close` manage the connection. `Value` has `Null`, `Integer(i64)`, `Real(f64)`, `Text(str)` and `Blob(Vec<u8>)` variants. Bind parameters rather than concatenating user data into SQL.

```dev
use "std/sqlite"
fn main() {
    let db = sqlite.open(":memory:")
    let empty = Vec<sqlite.Value>()
    db.execute("CREATE TABLE users(name TEXT)", empty)
    let args = Vec<sqlite.Value>()
    args.push(sqlite.Value.Text("Dev"))
    db.execute("INSERT INTO users VALUES (?)", args)
    print(db.query("SELECT name FROM users", empty).rows.len())
    db.close()
}
main()
```

Only one SQL statement is accepted per call; SQL is capped at 64 KiB, parameters at 999, columns at 256 and result rows at 4096. Parameter/result model budgets include allocation metadata. Nonfinite real parameters and invalid UTF-8 text results are rejected. Foreign keys are enabled. Busy timeout defaults to 5000 ms; `setTimeout` accepts 1–300000 ms and also installs a progress check for expensive queries. It is not a hard overall operation deadline.

Transactions use explicit `BEGIN`, `COMMIT`, `ROLLBACK`. Catching an error does not roll back automatically. Closing a connection rolls back an unfinished transaction. A query with side effects may perform those effects before a later result conversion/size error.

## std/csv

`parse(text, delimiter, hasHeaders)` returns `Table(headers Vec<str>, rows Vec<Vec<str>>)`. `stringify(table, delimiter)` emits CSV. Delimiters are a single ASCII byte other than NUL, quote, CR or LF. Quoting, escaped quotes, embedded newlines, Unicode, CRLF and tab delimiters are supported. Every value is a string; duplicate header names are preserved.

Rows must have consistent widths. Limits are 4096 data rows and 256 columns. Blank lines are ignored. The CSV parser follows the `csv` crate's permissive quoting behavior; it is not a strict validator for every malformed RFC 4180 quote sequence. NUL-containing input/cells are rejected.

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

See [the executable example](../examples/storage-libs/main.dev). `scripts/smoke_storage_libs.py` checks typed SQLite rows, SQL binding, transactions, budgets, and Python SQLite/CSV/TOML interoperability.
