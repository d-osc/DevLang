---
name: devlang
description: Write, explain, debug and validate DevLang .dev programs, modules, managed collections, generics, threaded tasks and C FFI using the d CLI. Use for DevLang language work, not unrelated languages or Dev OS package management.
---

# DevLang

Write runnable DevLang using the actual implementation's syntax and execution mode. This skill targets the v0.4 language implementation; local parser/runtime/compiler and current project docs take precedence if the installed version differs. Release and component version numbers are independent.

## Choose context and mode

- For Node-style net/path/os/stream/url/module/process/events/buffer/dgram,
  read `docs/node-core.md`. Use the updated source runtime with `std/NAME`
  imports, not native builds or Node require. Close handles in their owning
  thread. TCP/UDP callbacks run after main statements finish. Network/file
  stream bytes use Vec<u8>; Buffer has toBytes/fromBytes conversions. Do not
  assume Node overloads, timers, generic streams, Buffer views or Promises.

- For filesystem or HTTP clients, consult `docs/fs-http.md`: `std/fs` and `std/http`
  currently require the updated source runtime, not native compilation. Requests
  block; failures abort with located runtime errors. Use `Response.bytes` for binary
  payloads and `Response.body` for text. Node-style Sync filesystem names and
  `std/fs/promises` (Task results) are available. HTTP servers use `createServer`,
  `listen`, `listenOn`, `close` and typed request/response callbacks. Server
  callbacks run after main statements complete, and must call `res.end(text)`.
  Do not invent JavaScript Promises, streaming, optional arguments or catch APIs.

- For syntax, core types, modules, collections and functions, read [references/core.md](references/core.md).
- For recursive data, generic constraints, threads, callbacks or C ABI, read [references/advanced.md](references/advanced.md).
- For project manifests/dependencies, formatting, language-server or native-debugger work, read [references/tooling.md](references/tooling.md). These tools require the latest source build.
- In a DevLang checkout, consult `docs/language.md`, `docs/advanced.md`, `docs/runtime.md` and the relevant `.dev` examples rather than inventing APIs. The repository is https://github.com/d-osc/DevLang.
- Use `d FILE.dev` or `d run FILE.dev` for source execution. Use `d build FILE.dev --release` for native compilation. `d run` is different from `devc run`, which compiles then executes.
- Keep compiler, source interpreter and native stdlib separate when changing the implementation. Pure Dev runtime execution never needs a C compiler; first-use C source FFI preparation does.

## Essential language rules

- DON (`.don`) is the JSON-compatible data format: comments, bare keys, colon/equal,
  newline separators, single/double/triple-quoted strings. Read `docs/don.md` in a
  checkout. `std/don` parse/stringify/toJSON/fromJSON runs in the source runtime.
  New projects use `package.don` with the existing nested package/dependencies
  schema; legacy `dev.toml` and TOML `dev.lock` remain supported. This is not Node's
  package.json schema. Reformatting or package mutation discards DON comments.
  DON `@version` / `@server.port` references are root-relative, support forward
  declarations and preserve value types; missing targets/cycles fail. Quoted
  references stay strings. Serialization writes resolved copies. Optional root
  manifest `version` is metadata, not a semver solver.

- Declare typed parameters as `fn add(a i64, b i64) i64`. Return type follows the parameter list; colons and arrows are optional. Statements end at newline or optional semicolon.
- Explicitly invoke `main()` after its declaration. Declarations never auto-run. Use file-level `return main()` only when the result should become exit status. Imported files contain declarations only.
- `let` is mutable and needs an initializer. Default literals are i64/f64. Numeric types do not implicitly mix; use `as` and matching widths. Conditions require bool.
- `for i in start..end` is ascending, end-exclusive and uses i64 bounds. There is no collection iteration, inclusive range or custom step.
- Struct constructors are positional. Methods are `fn Type.method(self Type, ...) R` with a value receiver. Fixed native arrays cannot be copied/passed/returned directly; use a struct or Vec.
- `Ref<T>` is immutable managed storage; `deref` returns a copy. Vec/Map mutation uses copy-on-write; Slice is a read-only snapshot. Missing-key Map.get and empty Vec.pop are errors.
- Struct/enum identity is nominal. Recursive layouts need Ref/raw-pointer/collection edges. Payload enums use exhaustive match statements; guard/nested patterns need an unguarded fallback when refutable.
- Closures capture by value at creation. Captured mutations do not persist between calls or update the outer variable. Generic functions need concrete wrappers before becoming function values.
- `export` preserves a C symbol; it is not required to call functions from an imported Dev module.

## Unsafe and tasks

Use lexical `unsafe { ... }` for extern calls, raw pointer operations, volatile access and C callback creation. Called functions and closures need their own unsafe blocks. Do not infer that foreign pointer lifetimes or C signatures are proven safe. Native str borrows C memory; Ref around a pointer does not own its allocation. Address-of interpreter locals remains native-only.

`async fn`, spawn and then create OS threads. Await blocks; ready polls. This is not a coroutine/event-loop scheduler. Await required work before process exit and observe worker errors. Keep a callback_context record alive until C unregisters and finishes all invocations; userdata is the last C argument. Raw capturing native callbacks have a 64-pair limit per signature/module.

## Validate the chosen path

Use the project's existing d installation or `target/release/d[.exe]` / `dist/d/<platform>/d[.exe]`. If missing, explain the concrete requirement instead of treating another language as a Dev interpreter.

```sh
d check main.dev
d main.dev --engine auto
d main.dev --engine ast
d build main.dev --release --output out/app
```

Native builds need Clang/GCC; Linux bundles offer TinyCC for basic fast builds. Thread/callback native programs need a C11 backend. Stdlib imports/linking require explicit `--module-dir std=... --link ...` and workdir-correct paths.

For pure Dev programs, [scripts/verify.py](scripts/verify.py) runs native frontend checking plus both source engines and compares exit status/stdout. It does not prove C ABI correctness or run the native output. Supply the d path and source file; add `--skip-native-check` only for a deliberate runtime-only program. When validation requires C dependencies, hardware or external resources, check that those are available and describe the verification boundary.

Avoid unsupported syntax such as JavaScript let syntax with untyped function parameters, Rust impl blocks, Python indentation or collection for-each. Report what actually ran and distinguish source execution, frontend check and native build. Do not promise universal performance or production memory safety.
