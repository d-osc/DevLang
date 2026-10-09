# Compiler architecture

The compiler is a dependency-free Rust executable. The compilation path is:

```text
.dev files â†’ lexer â†’ parser â†’ module graph/signatures â†’ type checking + C emission
           â†’ parallel C object compilation â†’ native link / static archive
```

`compiler/src/lexer.rs` preserves source locations and statement newlines. `compiler/src/parser.rs` builds the AST and parses expressions by precedence. `compiler/src/program.rs` canonicalizes and loads the import graph, handles cyclic imports, validates function signatures and assigns module-specific symbols. `compiler/src/codegen.rs` performs contextual type inference, validates expressions/control flow, and emits a source/header pair per module. `compiler/src/build.rs` handles backend discovery, parallel native compilation, caching, linking and publication. `compiler/src/main.rs` supplies the `devc` CLI.

The generated type header uses standard C11 integer and pointer-sized types. Module headers contain prototypes; each source includes its own header and those of direct imports. This supports forward calls and cycles without combining the program into one large C translation unit.

An object's cache key includes the compiler executable's path/size/modification time, cached compiler version, compiler version of Dev Lang, platform, options, generated C, type header and directly imported interface headers. The compiler's file metadata is checked every build, while its `--version` process runs only when the tool identity changes. Object bytes and the linked output are validated against saved content hashes. These hashes use FNV-1a for inexpensive local cache checks and are not cryptographic integrity guarantees.

A body edit changes the edited object's key. A signature edit also changes its consumers' interface fingerprints. Flags invalidate incompatible objects. Custom `--cflag` values trigger object recompilation because flags can refer to externally changing headers. C source link inputs and custom link flags always trigger relinking; raw object/archive inputs participate by content. Changes to external system headers/toolchain dependencies outside the recorded compiler identity can require deleting the cache manually.

Generated sources and backend output live in a private directory inside the cache during a build. A failed compile/link leaves the previously successful executable intact. Successful outputs are renamed into place; replacement is atomic on Unix, while Windows removes an existing destination before renaming. Concurrent builds should use separate output/cache directories on Windows. The cache directory is disposable and no source files depend on it.

The frontend can lower unsigned arithmetic whose low 32 bits are masked, selecting the narrowed form for SSE4.1-capable backends. Backend `-O0` favors compile latency; `--release` uses `-O3`, and `--fast` uses TinyCC. The project currently favors a small maintainable frontend and C interoperability over a dedicated machine-code backend. Cold-build performance therefore still includes native compiler/linker startup and work. A future direct backend could improve that, but requires target/ABI and optimization infrastructure.

## Independent compiler and runtime

The Cargo workspace contains `compiler/`, `runtime/` and shared `syntax/`.
The independent `cli/` launcher exposes `d`: a file or run alias dispatches
to `devrun`, and build/compiler aliases dispatch to `devc build`. It passes
arguments and streams through without linking the two implementations together.
The compiler emits C and invokes a native backend. The runtime directly evaluates
typed in-memory numeric instruction plans, with AST fallback and intrinsic modules. Pure Dev execution needs
no compiler. Source-only C dependencies are prepared through sibling `devc`,
which owns backend invocation and caching in `compiler/src/native.rs`. Both binaries share `syntax/` and do not depend on each other;
the runtime additionally uses libffi and libloading for native calls.
`stdlib/build.py` independently builds the native C library; generic
`--module-dir` and `--link` select it for compiled programs.
Distribution packages live in `dist/compiler/<platform>`,
`dist/runtime/<platform>` and `dist/stdlib/<platform>`.

Both execution paths execute entry-file statements in source order. Function declarations are inert; `main()` must be called explicitly. Imported modules contain declarations only. The compiler emits an internal script function and a C entry wrapper; it does not implicitly invoke the Dev function named main.

Native FFI uses bundled libffi and libloading, with cached platform C call interfaces and persistent shared-library handles. Interpreter Values are marshalled into stable typed ABI slots and temporary C strings. Native symbols are resolved from explicitly loaded libraries and system CRTs, with a lazy fallback to libraries adjacent to modules declaring extern functions; unresolved symbols backed by adjacent C sources can trigger automatic dependency
preparation by the compiler executable. Dev source is never compiled by this path.

`runtime/src/numeric.rs` conservatively lowers eligible functions into typed
register instructions with static local slots and jumps. Unsupported constructs
or inconsistent types select AST fallback, preserving lazy errors. No native
backend is involved. Plans retain source spans for runtime errors and cache at
most one reusable workspace per function; recursive calls never borrow a live
workspace. Function ASTs have shared immutable ownership and a name index,
avoiding a whole-body clone on each call. Both execution paths avoid copying an
entire named array just to read one indexed element. Array assignment and
parameter passing retain value-copy semantics. Differential checks exercise
both `--engine auto` and `--engine ast`.

`syntax/src/expand.rs` resolves nominal types and infers arguments and specializes generic
functions and structs after the import graph is loaded. Both execution paths
consume the same expansion, including cross-module/cyclic imports. Generic
instances are deduplicated and bounded; by-value recursive layouts are rejected; pointer/reference edges use finite nominal identities. Payload enums emit tagged unions, and `match` binds payload copies. Immutable managed references use reference counting in both modes.
The parser lowers range loops to scoped blocks and while loops with hidden
counters, evaluating bounds once and preserving continue behavior. Runtime
structs own value fields; the compiler emits guarded dependency-ordered C
record/enum definitions in module headers. Payload-free native enums use int32; payload enums use a tag and C union.


The shared expansion lifts closures into functions plus immutable environment
records. Native function values carry a C adapter and a retained context; runtime
function values carry a module/function identity and shared snapshot. Traits are
checked against concrete method signatures before specialization. Containers use
copy-on-write storage and typed keep/drop visitors; Map uses open addressing and backward-shift deletion
with a shared bucket index. Thread-backed tasks detach on last-handle cleanup;
workers retain their own storage until completion. Completion polling is nonblocking. Native shared counts
use C11 atomics in every translation unit of a threaded program. Runtime workers
have independent numeric-plan workspaces and native bindings. FFI callbacks use
libffi trampolines in runtime and ordinary C function pointers for plain native
functions. Native capturing function values use a bounded pool of C trampolines
that retain environments until exit. Both forms support scalar C signatures.

Context callbacks carry an ordinary C bridge, opaque userdata and a managed Ref to
a function value. Native bridges retain the owner during invocation; runtime
trampolines upgrade a weak owner and validate userdata. Task continuations lower
in shared syntax expansion to a concrete helper capturing task/function arguments
inside a spawned worker. Both execution paths use the same lowering.
