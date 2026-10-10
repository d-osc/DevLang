# std/cli

แยก command-line flags, values และ positionals

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/cli"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `cli.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
struct Option { name str, short str, takesValue bool }
struct Parsed { values Map<str,str>, flags Map<str,bool>, positionals Vec<str> }
fn args() Vec<str>
fn parse(args Vec<str>, options Vec<Option>) Parsed
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/cli"

fn main() {
    let options = Vec<cli.Option>()
    options.push(cli.Option("name", "n", true))
    let argv = Vec<str>()
    argv.push("--name")
    argv.push("Dev")
    let parsed = cli.parse(argv, options)
    print(parsed.values.get("name"))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/cli/main.dev
d examples/modules-api/cli/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

`std/dns` and `std/cli` are built into the updated **source runtime**. Their
signatures are shared with the Language Server. Native builds do not implement
these modules. They need no C compiler or external executable.

## CLI argument parsing

```dev
use "std/cli"
fn main() {
    let options = Vec<cli.Option>()
    options.push(cli.Option("verbose", "v", false))
    options.push(cli.Option("port", "p", true))
    let parsed = cli.parse(cli.args(), options)
    if parsed.flags.contains("verbose") { print("verbose enabled") }
    if parsed.values.contains("port") { print(parsed.values.get("port")) }
    for i in 0..parsed.positionals.len() as i64 {
        print(parsed.positionals[i as usize])
    }
}
main()
```

```sh
d main.dev -- -vp3000 serve -- --literal
```

The first `--` belongs to `d` and forwards program arguments. The second belongs
to `cli.parse` and ends option parsing, making `--literal` positional.

| API/type | Contract |
| --- | --- |
| `args()` | `Vec<str>` of program arguments only; excludes CLI executable and source path |
| `Option(name str, short str, takesValue bool)` | Declares long name, optional one-letter alias, and whether a value is required |
| `parse(args Vec<str>, options Vec<Option>)` | `Parsed { values Map<str,str>, flags Map<str,bool>, positionals Vec<str> }` |

Map keys are the canonical long names, irrespective of the alias used. Present
flags contain `true`; absent flags/values have no map entry. Check `.contains`
before `.get`, because a missing-key Map.get is an error. Values stay strings:
parse and validate numeric values with your application's rules.

Supported forms with the example schema:

| Input | Meaning |
| --- | --- |
| `--verbose`, `-v` | Flag |
| `--port 3000`, `--port=3000` | Value option |
| `-p 3000`, `-p3000`, `-p=3000` | Short value option |
| `-vp3000` | Grouped flag followed by value option |
| `--port -1` | The value is the string `-1` |
| `--port=` | Explicit empty string value |
| `serve --verbose file.dev` | Options interleaved with positionals |
| `-- --verbose -p3000` | Both following tokens are positional |
| `-` | Positional, e.g. stdin marker in your application |

A value option consumes the next token verbatim, even if it starts with `-`:
`--port --verbose` sets port to `--verbose` and does not enable the flag.
An isolated `--` is never consumed as a separated value; use `--port=--` or
`-p=--` for that literal. Attached values consume the remainder of a short
group, so `-pv` means port `v`, not two flags. Quoting is handled by the calling
shell; the parser receives already separated tokens and preserves Unicode.

Unknown options, repeated options (including long/short aliases), missing
values, and values attached to long flags are errors. There is no implicit
`--help`, `--version`, `--no-*`, optional value, abbreviations or repeatable
option. Define help/version flags explicitly and print your own message; the
library never exits the process. Subcommands can be interpreted from
`positionals`; no automatic command dispatcher is provided.

Option names start with an ASCII letter, contain only letters/digits/hyphens,
and are at most 64 bytes. A short alias is empty or one ASCII letter; it is
case-sensitive. Duplicate names/aliases are rejected even if unused. Parsing
allows at most 128 definitions, 4096 arguments and 8 MiB of argument text;
embedded NUL is rejected. These limits apply to args/parse, not to all runtime
strings or all OS argument APIs.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../system-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/cli/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
