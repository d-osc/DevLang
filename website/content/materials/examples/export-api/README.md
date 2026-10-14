# Export functions to C, C++, Rust and Go

`export fn` preserves the declared native C symbol. It is not required for ordinary DevLang imports: `main.dev` can call both `twice` and the exported functions from `api.dev`.

Run these commands from the repository root with the current `d`, Clang and llvm-ar on PATH:

```powershell
# Source runtime (no C compiler needed).
d examples/export-api/main.dev

# Native DevLang executable.
d build examples/export-api/main.dev --release -o out/export-api/dev-app.exe
./out/export-api/dev-app.exe

# Static library for a C consumer. api.dev contains declarations only, no main().
d build examples/export-api/api.dev --lib --release --cc clang --ar llvm-ar -o out/export-api/libdevmath.a
clang examples/export-api/main.c out/export-api/libdevmath.a -o out/export-api/c-app.exe
./out/export-api/c-app.exe
```

On Linux use `out/export-api/dev-app` and `out/export-api/c-app` instead of `.exe` names. Use your C compiler and archiver paths if they are not on PATH. The Dev library and C program must use matching targets/ABIs.

Runtime/native DevLang output:

```text
42
42
12.5
true
DevLang Math
```

C output omits the call to the ordinary `twice` function:

```text
42
12.5
true
DevLang Math
```

`devmath.h` is the explicit public C header; its signatures match the Dev declarations. `i32` maps to `int32_t`, `f64` to `double`, `bool` to C `bool`, and `str` to a borrowed `const char *`. Here the returned string is a literal and must not be freed. `export` neither owns nor copies foreign memory.

Exports must have unique names across the complete linked program. Do not export generic functions directly; export a concrete wrapper. Managed values such as Vec/Map/Ref cannot appear in extern/export C signatures. Only `export fn` is supported; JavaScript-style `export default`, `export { name }` and exported variable declarations are not module syntax.

An `export fn` runs normally in the source runtime; creating a native library for C requires `d build --lib`. Calling an exported Dev function from another Dev module requires no `unsafe` block. Calling a C `extern fn` from Dev requires `unsafe`.

## C++, Rust and Go

See [the complete interoperability guide](../../docs/export-interop.md) for all consumer sources, platform-specific build commands, ABI constraints and verified toolchain versions. `main.cpp`, `main.rs` and `go/main.go` all call the same `api.dev`.
