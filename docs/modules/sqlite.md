# std/sqlite

ฐานข้อมูล SQLite พร้อม parameter binding และ transactions

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/sqlite"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `sqlite.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
enum Value { Null, Integer(i64), Real(f64), Text(str), Blob(Vec<u8>) }
struct Database { id i64 }
struct Rows { columns Vec<str>, rows Vec<Vec<Value>> }
fn open(path str) Database
fn version() str
fn Database.execute(self Database, sql str, params Vec<Value>) i64
fn Database.query(self Database, sql str, params Vec<Value>) Rows
fn Database.lastInsertRowId(self Database) i64
fn Database.setTimeout(self Database, timeoutMs i64) void
fn Database.close(self Database) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/sqlite"

fn main() {
    let db = sqlite.open(":memory:")
    let params = Vec<sqlite.Value>()
    print(db.query("SELECT 42 AS answer", params).rows.len())
    db.close()
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/sqlite/main.dev
d examples/modules-api/sqlite/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

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

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../storage-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/sqlite/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
