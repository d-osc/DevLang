# Focused DevLang examples

Run pure Dev examples with `d examples/howto/NAME.dev` from the repository root,
or extract a website example ZIP and run the same path from its root.
Each file explicitly invokes `main()`.

- Basics: variables, conditions, logical aliases and short-circuit evaluation,
  operators, loops, functions and recursion, arrays and UTF-8 bytes.
- Data: structs and value methods, enums/payloads, match guards/wildcards/nested
  patterns, immutable Ref/recursive layout, Vec/Map copy-on-write and Slice.
- Abstraction: inference, constraints, const generics, traits, function values
  and closure capture snapshots.
- Systems: async/spawn/await/ready/then, raw/context callbacks, owned C allocation,
  volatile access, variadic C calls, export and defensive checks.
- Runtime APIs: console/string/time/arguments (`runtime_io.dev`), file I/O
  (`runtime_files.dev`, creates or overwrites `devlang-example.txt` in cwd), and
  input (`runtime_input.dev`, waits for one line).

Runtime API examples deliberately use interpreter intrinsics; the separate
native stdlib has different APIs. See `examples/stdlib/main.dev` for native use.
Foreign-memory examples allocate their own live memory; they do not access
physical device addresses. Raw pointers and C signatures remain the caller's
responsibility. `examples/memory.dev` requires native compilation because it
takes addresses of local variables. `examples/aggregate` includes a C source
dependency; its first source run needs `devc` and a C backend.

Build a pure Dev example using `d build examples/howto/NAME.dev --release -o out/app.exe`.
Run `./out/app.exe` on Windows or Linux. The output name is explicit; the compiler
does not add `.exe` automatically. Task examples need a C11
backend. All published validation commands and outputs appear on the website.
