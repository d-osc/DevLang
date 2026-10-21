# d command

CLI 0.5.0 selects dependency versions independently per peer context and shares compatible physical instances (version-4 locks), alongside a configurable resolver budget, `d task`, `d pkg ci/tree/why/dedupe/cache/audit/verify/verify-tag`, extended version ranges, optional dependencies, overrides, host platform policy, bounded version backtracking and opt-in peer installation. See [Package tools](../docs/package-features.md) for examples and limits.

The latest source build also includes project dependencies, formatting, a stdio
language server and native debugger integration. See [Developer tools](../docs/tooling.md)
for `package.don`/`package-lock.don`, `d new`, `d pkg`, `d fmt`, `d lsp` and `d debug`.
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
| `--package NAME` | Select a package in the enclosing workspace; relative paths use its directory |
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

Workspaces and Git-tag SemVer requirements are described in [Packages](../docs/packages.md).
Use `d pkg workspace`, `d pkg install --workspace`, and `d run --package NAME`.

Static registries use `index.don` plus checksum-verified archives. Create one with
`d pkg publish --registry DIRECTORY` (local output), discover packages with
`d pkg search --registry INDEX_URL [QUERY]`, and add a matching release with
`d pkg add NAME --registry INDEX_URL --version REQUIREMENT [--dev]`. The manifest
stores the selected exact version and archive checksum; repeat `pkg add` to select
a newer registry release. See [the registry example](../examples/registry/README.md).

### GitHub Package Store

`d pkg inspect --json` validates the current manifest without installing or running dependencies. See [repository conventions and store discovery](../docs/package-store.md). The store indexes public `devlang-package` repositories with root `package.don` manifests and matching SemVer Git tags.

### Store and isolated dependency graphs (CLI/runtime 0.2.0)

Search with d pkg search QUERY, inspect with d pkg info @owner/repo, and add with d pkg add @owner/repo --as math --version '^1'. Use --store URL for a custom catalog. pkg outdated reports current/wanted/latest; pkg upgrade preserves ranges and --latest explicitly opts into new stable majors. Both support direct development dependencies. Git sources support --subdir packages/math. pkg publish --github OWNER/REPO [--dry-run] prepares/upload tagged immutable assets using gh; see the Store guide for authentication and clean/tag requirements.

Transitive namespace collisions now use version-2 graph locks with package-local module scopes across auto/AST/native/LSP/global bins. Compatible graphs keep version-1 locks. Optional devlang constrains CLI SemVer; pkg compatibility --json reports it. Build all three components from current source to use scoped module mappings.

## Executable scripts (shebang)

`dev` is the same launcher as `d` and is meant for `#!/usr/bin/env dev`.
`#!/usr/bin/env d` works as well. The `#!` line is read as a comment, and
arguments after the script path go to the program without `--`. With `d`,
arguments that start with `-` stay with the launcher (use `dev` when a script
needs flags):

```sh
#!/usr/bin/env dev
use "std/args"
print(args.get(0))
```

```sh
chmod +x hello && ./hello world
```

## Updating

```sh
d update --check          # report the newest release
d update                  # install it
d update --version v0.5.3 # install a specific release
```

The installer is verified against the release `SHA256SUMS` before it runs. See [Install DevLang](../docs/install.md).
