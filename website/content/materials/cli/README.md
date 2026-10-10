# d command

The latest source build also includes project dependencies, formatting, a stdio
language server and native debugger integration. See [Developer tools](../docs/tooling.md)
for `dev.toml`/`dev.lock`, `d new`, `d pkg`, `d fmt`, `d lsp` and `d debug`.
These commands are not included in the existing v0.4.0 installers.

`d` dispatches to the independent `devrun` runtime or `devc` compiler located
next to its executable. It preserves arguments, working directory, standard
input/output and exit status. It never falls back to compiling a runtime request.

```sh
d examples/modules/main.dev
d run examples/modules/main.dev
d --run examples/modules/main.dev
d -r examples/modules/main.dev

d build examples/modules/main.dev --release
d --build examples/modules/main.dev
d -b examples/modules/main.dev
d compiler examples/modules/main.dev
d --compiler examples/modules/main.dev
d -c examples/modules/main.dev
```

Runtime arguments follow `--`. Compiler options are passed unchanged to `devc
build`. `d check`, `d emit`, `d -e`, `d --help` and `d --version` are
also supported. Selectors occupy the first argument.

Basic options:

| Option | Behavior |
| --- | --- |
| `-h`, `--help` | Show help, including `d build --help` or `d FILE.dev -h` |
| `-v`, `-V`, `--version` | Show launcher version |
| `-C DIR`, `--cwd DIR` | Execute relative to DIR; accepted before or after the command |
| `-e CODE`, `--eval CODE` | Interpret inline statements |
| `--engine auto\|ast` | Runtime engine: numeric plans with AST fallback (default), or reference AST execution |
| `--timings` | Report load/execution timings for runtime, build timings for compiler, on stderr |
| `--ffi-lib PATH` | Load an existing native shared library in runtime mode (repeatable) |
| `check`, `--check` | Validate a native program without compiling it |
| `emit`, `--emit` | Generate C source |
| `-o PATH`, `--output PATH` | Compiler output executable/archive/C directory |
| `--debug`, `--release` | Compiler -O0 / -O3; the last option wins |
| `--native`, `--fast`, `--jobs N`, `--cc PATH` | CPU tuning, TinyCC, build concurrency and compiler selection |
| `--module-dir NAME=DIR`, `--link PATH` | External modules and native libraries |
| `-- ARGUMENTS` | Pass program arguments literally, even values such as `--help` |

Compiler options follow the source filename. `--cwd` and `--timings` can precede
the command. Runtime mode rejects compiler-only options. `check` validates native
Dev modules; interpreter-only intrinsic APIs need runtime validation instead.

```sh
d --cwd examples/modules main.dev --timings
d --eval 'print(40 + 2)'
d --check examples/modules/main.dev
d build examples/modules/main.dev --release --output out/app
d emit examples/modules/main.dev --output out/generated
```

Build all binaries using `cargo build --release`, then use `target/release/d`
(`d.exe` on Windows). The distribution bundle is in `dist/d/<platform>`.
Pure Dev or prebuilt-library FFI execution only needs `d` and `devrun`.
Source-only C FFI is prepared automatically through sibling `devc`, with a
C backend required on the first run; later runs reuse `.dev-cache/native/`.
Compilation only needs `d`
and `devc` plus a C backend. The launcher has no compiler/runtime crate
dependencies. To use the bare command, add the bundle directory to your PATH.
