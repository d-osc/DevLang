# Dev command

`dev` dispatches to the independent `devrun` runtime or `devc` compiler located
next to its executable. It preserves arguments, working directory, standard
input/output and exit status. It never falls back to compiling a runtime request.

```sh
dev examples/modules/main.dev
dev run examples/modules/main.dev
dev --run examples/modules/main.dev
dev -r examples/modules/main.dev

dev build examples/modules/main.dev --release
dev --build examples/modules/main.dev
dev -b examples/modules/main.dev
dev compiler examples/modules/main.dev
dev --compiler examples/modules/main.dev
dev -c examples/modules/main.dev
```

Runtime arguments follow `--`. Compiler options are passed unchanged to `devc
build`. `dev check`, `dev emit`, `dev -e`, `dev --help` and `dev --version` are
also supported. Selectors occupy the first argument.

Basic options:

| Option | Behavior |
| --- | --- |
| `-h`, `--help` | Show help, including `dev build --help` or `dev FILE.dev -h` |
| `-v`, `-V`, `--version` | Show launcher version |
| `-C DIR`, `--cwd DIR` | Execute relative to DIR; accepted before or after the command |
| `-e CODE`, `--eval CODE` | Interpret inline statements |
| `--timings` | Report load/execution timings for runtime, build timings for compiler, on stderr |
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
dev --cwd examples/modules main.dev --timings
dev --eval 'print(40 + 2)'
dev --check examples/modules/main.dev
dev build examples/modules/main.dev --release --output out/app
dev emit examples/modules/main.dev --output out/generated
```

Build all binaries using `cargo build --release`, then use `target/release/dev`
(`dev.exe` on Windows). The distribution bundle is in `dist/dev/<platform>`.
Runtime execution only needs `dev` and `devrun`; compilation only needs `dev`
and `devc` plus a C backend. The launcher has no compiler/runtime crate
dependencies. To use the bare command, add the bundle directory to your PATH.
