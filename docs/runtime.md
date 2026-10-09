# Interpreter API

Run `devrun FILE.dev [-- arguments]` or `devrun -e 'statements'`.
`--eval` is an alias for `-e`; `--timings` prints load/execution times to stderr.
`-h`/`--help` and `-v`/`-V`/`--version` show help and version.
Runtime errors include source path, line and column. `main` defaults to return type `i32`;
falling through it exits successfully. An explicit return sets the exit code.
Other non-void functions must return a value.

Use `use "std/io"` (or `as alias`). Built-in modules need no files or libraries:

| Module | Functions |
| --- | --- |
| `std/io` | `write(str) bool`, `writeln(str) bool`, `read_line() str`, `read_file(path str) str`, `write_file(path str, text str) bool` |
| `std/strings` | `len(str) usize`, `concat(str, str) str`, `equal(str, str) bool` |
| `std/time` | `now_ns() u64`, `now_ms() u64`, `sleep_ms(integer) bool` |
| `std/args` | `len() usize`, `get(integer) str` |

I/O failures produce runtime errors. Text files and strings use UTF-8. String
length and indexing count bytes; indexing produces `u8`. `read_line` removes
the final newline. Arguments include only values following `--`, without the
source filename. Time measures elapsed monotonic time since runtime startup.

`print(value)` writes a line; string `+` concatenates. Fixed-width integer
arithmetic wraps, division by zero and invalid shifts produce errors. Locals
and parameters enforce declared types. Explicit numeric casts use `as`.
Boolean operators short-circuit. Array indices are checked. Unexecuted code is
not type-checked: use `devc check` for native programs, noting that runtime
intrinsics and the native stdlib have different APIs.

The runtime evaluates source, with no compilation cache, C translation or
native code generation. No raw pointer, C FFI, hardware access, networking,
async, JIT or JavaScript compatibility is provided in this release.
