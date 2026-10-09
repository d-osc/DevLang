# Compiler architecture

The compiler is a dependency-free Rust executable. The compilation path is:

```text
.dev files → lexer → parser → module graph/signatures → type checking + C emission
           → parallel C object compilation → native link / static archive
```

`compiler/src/lexer.rs` preserves source locations and statement newlines. `compiler/src/parser.rs` builds the AST and parses expressions by precedence. `compiler/src/program.rs` canonicalizes and loads the import graph, handles cyclic imports, validates function signatures and assigns module-specific symbols. `compiler/src/codegen.rs` performs contextual type inference, validates expressions/control flow, and emits a source/header pair per module. `compiler/src/build.rs` handles backend discovery, parallel native compilation, caching, linking and publication. `compiler/src/main.rs` supplies the `devc` CLI.

The generated type header uses standard C11 integer and pointer-sized types. Module headers contain prototypes; each source includes its own header and those of direct imports. This supports forward calls and cycles without combining the program into one large C translation unit.

An object's cache key includes the compiler executable's path/size/modification time, cached compiler version, compiler version of Dev Lang, platform, options, generated C, type header and directly imported interface headers. The compiler's file metadata is checked every build, while its `--version` process runs only when the tool identity changes. Object bytes and the linked output are validated against saved content hashes. These hashes use FNV-1a for inexpensive local cache checks and are not cryptographic integrity guarantees.

A body edit changes the edited object's key. A signature edit also changes its consumers' interface fingerprints. Flags invalidate incompatible objects. Custom `--cflag` values trigger object recompilation because flags can refer to externally changing headers. C source link inputs and custom link flags always trigger relinking; raw object/archive inputs participate by content. Changes to external system headers/toolchain dependencies outside the recorded compiler identity can require deleting the cache manually.

Generated sources and backend output live in a private directory inside the cache during a build. A failed compile/link leaves the previously successful executable intact. Successful outputs are renamed into place; replacement is atomic on Unix, while Windows removes an existing destination before renaming. Concurrent builds should use separate output/cache directories on Windows. The cache directory is disposable and no source files depend on it.

The frontend can lower unsigned arithmetic whose low 32 bits are masked, selecting the narrowed form for SSE4.1-capable backends. Backend `-O0` favors compile latency; `--release` uses `-O3`, and `--fast` uses TinyCC. The project currently favors a small maintainable frontend and C interoperability over a dedicated machine-code backend. Cold-build performance therefore still includes native compiler/linker startup and work. A future direct backend could improve that, but requires target/ABI and optimization infrastructure.

## Independent compiler and runtime

The Cargo workspace contains `compiler/`, `runtime/` and shared `syntax/`.
The independent `cli/` launcher exposes `dev`: a file or run alias dispatches
to `devrun`, and build/compiler aliases dispatch to `devc build`. It passes
arguments and streams through without linking the two implementations together.
The compiler emits C and invokes a native backend. The runtime directly evaluates
AST with scoped call frames and intrinsic modules; it never invokes the compiler
or a native backend. Both binaries depend only on `syntax/`, not each other.
`stdlib/build.py` independently builds the native C library; generic
`--module-dir` and `--link` select it for compiled programs.
Distribution packages live in `dist/compiler/<platform>`,
`dist/runtime/<platform>` and `dist/stdlib/<platform>`.
