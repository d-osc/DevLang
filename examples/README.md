# DevLang examples

Run commands from the repository root with the current source-built `d`. On Windows without `d` on PATH use `.\target\release\d.exe`.

| Examples | How to use |
|---|---|
| `hello.dev`, `howto/`, `features/` | Run a main program with `d FILE.dev`; helper module files contain declarations and are imported by main |
| `modules-api/` | [Reference examples for every standard module](modules-api/README.md); native examples have separate build/link instructions in the module docs |
| `runtime/`, `don/`, `node-io/`, `core-modules/` | Source runtime APIs; do not assume native stdlib has identical APIs |
| `tooling/` | [Local dependency and native build](tooling/README.md) |
| `workspace/` | [Workspace members and version requirements](workspace/README.md) |
| `package-don/` | [Complete manifest and dependency example](package-don/README.md) |
| `dev-dependencies/` | [Development tools and production-only installation](dev-dependencies/README.md) |
| `package-bin/` | [CLI commands exported by local and dependency packages](package-bin/README.md) |
| `github-tag/` | [Git tag/branch example](github-tag/README.md); replace GitHub placeholder URL before install, or run its local Git verification script |
| `archive-url/` | [ZIP/TAR URL example](archive-url/README.md); replace URL and hash before install, or run its localhost HTTP verification script |
| `ffi/` | [C FFI](ffi/README.md); first run may need a C compiler for the adjacent C dependency |
| `memory.dev`, `stdlib/` | Native examples; use the relevant compiler/module/link options |

All programs explicitly invoke `main()`. Library files such as `src/lib.dev` are imported, rather than run as applications. Standard modules use `use "std/io"`; they are not file paths such as `examples/stdlib/io.dev`.

Package examples require `pkg install` before run. It creates `package-lock.don`; `pkg install --locked` verifies an existing lock. URL examples with `YOUR_USER`, `YOUR_HOST`, `example.com` or placeholder checksums are templates for your own packages.

Validate all package examples without modifying their manifests or caches:

```powershell
python scripts/verify_package_examples.py --d target/release/d.exe
```

This includes a real native build of the tooling app, so a C backend must be available. The Git and archive tests use local temporary repositories and a localhost HTTP server.

Validate website examples and every module's documented sample:

```powershell
python website/build.py --d target/release/d.exe
python scripts/build_module_docs.py --d target/release/d.exe
```

`peer-dependencies/` demonstrates a plugin requiring the consumer’s shared `math` version, with verification of missing and incompatible peers.
